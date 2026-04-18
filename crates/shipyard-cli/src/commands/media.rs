use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use clap::Subcommand;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use shipyard_core::{pubkey_from_secret_hex, NostrEvent};

// ── Blossom upload auth (kind 24242) ─────────────────────────────────────────

fn blossom_auth_event(secret_hex: &str, sha256_hex: &str) -> Result<String> {
    let pubkey = pubkey_from_secret_hex(secret_hex).context("invalid secret key")?;
    let expiration = chrono::Utc::now().timestamp() + 600;

    let mut event = NostrEvent::unsigned(
        pubkey,
        chrono::Utc::now().timestamp(),
        24242,
        vec![
            vec!["t".to_string(), "upload".to_string()],
            vec!["x".to_string(), sha256_hex.to_string()],
            vec!["expiration".to_string(), expiration.to_string()],
        ],
        format!("Upload {sha256_hex}"),
    );

    event
        .sign_with_secret_hex(secret_hex)
        .context("failed to sign auth event")?;

    let json = serde_json::to_string(&event).context("failed to serialize auth event")?;
    Ok(STANDARD.encode(json.as_bytes()))
}

// ── Blossom server response ───────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct BlossomResponse {
    url: String,
    sha256: String,
    size: u64,
    #[serde(rename = "type")]
    mime_type: Option<String>,
}

// ── Commands ──────────────────────────────────────────────────────────────────

#[derive(Debug, Subcommand)]
pub(crate) enum MediaCommand {
    /// Upload a file to a Blossom server
    Upload {
        /// Path to the file to upload
        path: PathBuf,

        /// Blossom server base URL
        #[arg(
            long,
            default_value = "https://blossom.primal.net",
            env = "BLOSSOM_SERVER"
        )]
        server: String,

        /// Nostr secret key (64-char hex) used to sign the upload auth event
        #[arg(long, env = "NOSTR_SECRET_KEY")]
        secret_key: String,
    },

    /// List configured Blossom servers (currently shows default)
    Servers {
        /// Blossom server override
        #[arg(
            long,
            default_value = "https://blossom.primal.net",
            env = "BLOSSOM_SERVER"
        )]
        server: String,
    },
}

pub(crate) async fn run(command: MediaCommand) -> Result<Value> {
    match command {
        MediaCommand::Upload {
            path,
            server,
            secret_key,
        } => upload_file(&server, &path, &secret_key).await,
        MediaCommand::Servers { server } => Ok(serde_json::json!({ "servers": [server] })),
    }
}

async fn upload_file(server: &str, path: &PathBuf, secret_key: &str) -> Result<Value> {
    // 1. Read file (blocking read is fine in a CLI context)
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read file: {}", path.display()))?;

    // 2. SHA-256 hash
    let digest = Sha256::digest(&bytes);
    let sha256_hex = hex::encode(digest);

    // 3. Build Blossom auth header
    let auth_b64 = blossom_auth_event(secret_key, &sha256_hex)?;
    let auth_header = HeaderValue::from_str(&format!("Nostr {auth_b64}"))
        .context("failed to build Authorization header")?;

    // 4. Guess MIME type from extension
    let mime = mime_from_path(path);

    // 5. HTTP PUT
    let client = reqwest::Client::new();
    let upload_url = format!("{}/upload", server.trim_end_matches('/'));

    let resp = client
        .put(&upload_url)
        .header(AUTHORIZATION, auth_header)
        .header(CONTENT_TYPE, mime)
        .body(bytes)
        .send()
        .await
        .with_context(|| format!("HTTP PUT to {upload_url} failed"))?;

    let status = resp.status();
    let body = resp.text().await.context("failed to read response body")?;

    if !status.is_success() {
        bail!("Blossom server returned {status}: {body}");
    }

    // 6. Parse response
    let blossom: BlossomResponse =
        serde_json::from_str(&body).with_context(|| format!("failed to parse response: {body}"))?;

    Ok(serde_json::json!({
        "url": blossom.url,
        "sha256": blossom.sha256,
        "size": blossom.size,
        "mime_type": blossom.mime_type,
    }))
}

fn mime_from_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mp3") => "audio/mpeg",
        Some("ogg") => "audio/ogg",
        Some("pdf") => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_from_path_returns_correct_types() {
        assert_eq!(mime_from_path(Path::new("photo.jpg")), "image/jpeg");
        assert_eq!(mime_from_path(Path::new("photo.jpeg")), "image/jpeg");
        assert_eq!(mime_from_path(Path::new("image.png")), "image/png");
        assert_eq!(mime_from_path(Path::new("video.mp4")), "video/mp4");
        assert_eq!(
            mime_from_path(Path::new("file.unknown")),
            "application/octet-stream"
        );
    }

    #[test]
    fn blossom_auth_event_produces_valid_base64_json() {
        let secret = "1111111111111111111111111111111111111111111111111111111111111111";
        let sha256 = "a".repeat(64);
        let b64 = blossom_auth_event(secret, &sha256).unwrap();
        let decoded = STANDARD.decode(&b64).unwrap();
        let json: Value = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(json["kind"], 24242);
        assert_eq!(json["tags"][0][0], "t");
        assert_eq!(json["tags"][0][1], "upload");
        assert_eq!(json["tags"][1][1], sha256);
    }
}
