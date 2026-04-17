use anyhow::Context;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use shipyard_core::{
    dvm::{
        build_signed_feedback_event, feedback_tags, parse_dvm_request, parse_encrypted_dvm_request,
        DvmRequestEvent, DVM_FEEDBACK_KIND, DVM_SCHEDULE_KIND,
    },
    nip04_decrypt, nip04_encrypt, pubkey_from_secret_hex, NostrEvent, Pubkey,
};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shipyard_dvm=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let relay_urls = std::env::var("SHIPYARD_DVM_RELAYS")
        .context("SHIPYARD_DVM_RELAYS is required for kind 5905 subscriptions")?;
    let feedback_secret_hex = std::env::var("SHIPYARD_DVM_SECRET_KEY")
        .context("SHIPYARD_DVM_SECRET_KEY is required to sign kind 7000 feedback")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let tick_seconds = std::env::var("SHIPYARD_DVM_TICK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);

    let relay_urls = parse_relay_urls(&relay_urls);
    tracing::info!(?relay_urls, tick_seconds, "shipyard-dvm starting");

    for relay_url in relay_urls.clone() {
        let pool = pool.clone();
        tokio::spawn(async move {
            subscribe_relay_forever(pool, relay_url).await;
        });
    }

    loop {
        tokio::select! {
            result = process_pending_dvm_requests(&pool, &feedback_secret_hex, &relay_urls) => {
                if let Err(error) = result {
                    tracing::error!(%error, "DVM processing tick failed");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shipyard-dvm shutting down");
                break;
            }
        }

        tokio::time::sleep(Duration::from_secs(tick_seconds)).await;
    }

    Ok(())
}

async fn process_pending_dvm_requests(
    pool: &PgPool,
    feedback_secret_hex: &str,
    relay_urls: &[String],
) -> anyhow::Result<()> {
    let rows = sqlx::query(
        "SELECT id, request_event_id, raw_request_event
         FROM dvm_requests
         WHERE status = 'received'
         ORDER BY created_at ASC
         LIMIT 25",
    )
    .fetch_all(pool)
    .await?;

    for row in rows {
        let dvm_request_id: Uuid = row.try_get("id")?;
        let request_event_id: String = row.try_get("request_event_id")?;
        let raw_request_event: serde_json::Value = row.try_get("raw_request_event")?;
        let result = process_one_dvm_request(
            pool,
            dvm_request_id,
            raw_request_event.clone(),
            feedback_secret_hex,
        )
        .await;
        match result {
            Ok(outcome) => {
                publish_feedback_to_relays(relay_urls, &outcome.feedback).await;
                tracing::info!(%request_event_id, feedback_id = ?outcome.feedback.id, "DVM request scheduled");
            }
            Err(error) => {
                tracing::warn!(%request_event_id, %error, "DVM request failed");
                mark_dvm_request_error(pool, dvm_request_id, error.to_string()).await?;
                let error_message = error.to_string();
                let feedback = match serde_json::from_value::<DvmRequestEvent>(raw_request_event) {
                    Ok(request_event) if has_encrypted_tag(&request_event) => {
                        build_encrypted_feedback_event(
                            feedback_secret_hex,
                            &request_event.pubkey,
                            "error",
                            &request_event_id,
                            Some(&error_message),
                            Utc::now().timestamp(),
                        )?
                    }
                    _ => build_signed_feedback_event(
                        feedback_secret_hex,
                        "error",
                        &request_event_id,
                        Some(&error_message),
                        Utc::now().timestamp(),
                    )?,
                };
                publish_feedback_to_relays(relay_urls, &feedback).await;
            }
        }
    }

    Ok(())
}

async fn subscribe_relay_forever(pool: PgPool, relay_url: String) {
    loop {
        if let Err(error) = subscribe_once(&pool, &relay_url).await {
            tracing::warn!(%relay_url, %error, "DVM relay subscription failed");
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn subscribe_once(pool: &PgPool, relay_url: &str) -> anyhow::Result<()> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        anyhow::bail!("invalid relay URL");
    }

    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .context("relay connection timed out")??;
    let subscription_id = format!("shipyard-dvm-{}", std::process::id());
    let since = Utc::now().timestamp().saturating_sub(60);
    let request = serde_json::json!([
        "REQ",
        subscription_id,
        {
            "kinds": [DVM_SCHEDULE_KIND],
            "since": since
        }
    ]);
    socket.send(Message::Text(request.to_string())).await?;
    tracing::info!(%relay_url, "DVM relay subscription established");

    while let Some(message) = socket.next().await {
        let message = message?;
        let Message::Text(text) = message else {
            continue;
        };
        let Some(request_event) = parse_relay_event_message(&text, &subscription_id)? else {
            continue;
        };
        insert_dvm_request(pool, &request_event).await?;
    }

    Ok(())
}

fn parse_relay_event_message(
    text: &str,
    subscription_id: &str,
) -> anyhow::Result<Option<DvmRequestEvent>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let Some(array) = value.as_array() else {
        return Ok(None);
    };
    if array.first().and_then(serde_json::Value::as_str) != Some("EVENT") {
        return Ok(None);
    }
    if array.get(1).and_then(serde_json::Value::as_str) != Some(subscription_id) {
        return Ok(None);
    }
    let Some(raw_event) = array.get(2) else {
        return Ok(None);
    };
    let request_event: DvmRequestEvent = serde_json::from_value(raw_event.clone())?;
    if request_event.kind != DVM_SCHEDULE_KIND {
        return Ok(None);
    }
    Ok(Some(request_event))
}

async fn insert_dvm_request(pool: &PgPool, request_event: &DvmRequestEvent) -> anyhow::Result<()> {
    let raw_request_event = serde_json::to_value(request_event)?;
    let encrypted = request_event
        .tags
        .iter()
        .all(|tag| tag.first().map(String::as_str) != Some("i"))
        && !request_event.content.is_empty();

    sqlx::query(
        "INSERT INTO dvm_requests
           (request_event_id, request_pubkey, encrypted, raw_request_event, status)
         VALUES ($1, $2, $3, $4, 'received')
         ON CONFLICT (request_event_id) DO NOTHING",
    )
    .bind(&request_event.id)
    .bind(request_event.pubkey.as_str())
    .bind(encrypted)
    .bind(raw_request_event)
    .execute(pool)
    .await?;

    Ok(())
}

async fn process_one_dvm_request(
    pool: &PgPool,
    dvm_request_id: Uuid,
    raw_request_event: serde_json::Value,
    feedback_secret_hex: &str,
) -> anyhow::Result<DvmProcessOutcome> {
    let request_event: DvmRequestEvent =
        serde_json::from_value(raw_request_event).context("stored DVM request event is invalid")?;
    let encrypted = has_encrypted_tag(&request_event);
    let parsed = if encrypted {
        let decrypted_tags = decrypt_request_tags(&request_event, feedback_secret_hex)?;
        parse_encrypted_dvm_request(&request_event, decrypted_tags)?
    } else {
        parse_dvm_request(&request_event)?
    };

    let mut tx = pool.begin().await?;
    for event in &parsed.scheduled_events {
        let event_id = event
            .id
            .clone()
            .context("DVM input event must include id")?;
        let publish_time = DateTime::<Utc>::from_timestamp(event.created_at, 0)
            .context("DVM input event created_at is invalid")?;
        let signed_event_json = serde_json::to_value(event)?;

        sqlx::query(
            "INSERT INTO users (pubkey)
             VALUES ($1)
             ON CONFLICT (pubkey) DO NOTHING",
        )
        .bind(event.pubkey.as_str())
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO accounts (pubkey)
             VALUES ($1)
             ON CONFLICT (pubkey) DO UPDATE SET updated_at = now()",
        )
        .bind(event.pubkey.as_str())
        .execute(&mut *tx)
        .await?;

        let publish_item_id: Uuid = sqlx::query_scalar(
            "INSERT INTO publish_items
               (owner_pubkey, created_by_pubkey, state, trigger, signed_event_json,
                event_id, publish_time)
             VALUES ($1, $1, 'SCHEDULED', 'DVM', $2, $3, $4)
             ON CONFLICT (event_id) DO UPDATE
               SET signed_event_json = excluded.signed_event_json,
                   publish_time = excluded.publish_time,
                   updated_at = now()
             RETURNING id",
        )
        .bind(event.pubkey.as_str())
        .bind(signed_event_json)
        .bind(event_id)
        .bind(publish_time)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO jobs (kind, run_at, payload)
             VALUES ('publish_event', $1, jsonb_build_object('publish_item_id', $2::text))",
        )
        .bind(publish_time)
        .bind(publish_item_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query("UPDATE dvm_requests SET status = 'scheduled', error = NULL WHERE id = $1")
        .bind(dvm_request_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let feedback = if parsed.encrypted {
        build_encrypted_feedback_event(
            feedback_secret_hex,
            &parsed.request_pubkey,
            "scheduled",
            &parsed.request_event_id,
            Some("Scheduled."),
            Utc::now().timestamp(),
        )?
    } else {
        build_signed_feedback_event(
            feedback_secret_hex,
            "scheduled",
            &parsed.request_event_id,
            Some("Scheduled."),
            Utc::now().timestamp(),
        )?
    };

    Ok(DvmProcessOutcome { feedback })
}

fn has_encrypted_tag(request_event: &DvmRequestEvent) -> bool {
    request_event
        .tags
        .iter()
        .any(|tag| tag.first().map(String::as_str) == Some("encrypted"))
}

fn decrypt_request_tags(
    request_event: &DvmRequestEvent,
    dvm_secret_hex: &str,
) -> anyhow::Result<Vec<Vec<String>>> {
    let plaintext = nip04_decrypt(
        dvm_secret_hex,
        &request_event.pubkey,
        &request_event.content,
    )
    .context("Error decrypting event")?;
    serde_json::from_str(&plaintext).context("decrypted DVM request tags are invalid")
}

fn build_encrypted_feedback_event(
    secret_hex: &str,
    recipient_pubkey: &Pubkey,
    status: &str,
    request_event_id: &str,
    message: Option<&str>,
    created_at: i64,
) -> anyhow::Result<NostrEvent> {
    let public_tags = vec![
        vec!["encrypted".to_string()],
        vec!["p".to_string(), recipient_pubkey.as_str().to_string()],
    ];
    let private_tags = feedback_tags(status, request_event_id, message);
    let content = nip04_encrypt(
        secret_hex,
        recipient_pubkey,
        &serde_json::to_string(&private_tags)?,
        random_iv()?,
    )?;
    let dvm_pubkey = pubkey_from_secret_hex(secret_hex)?;
    let mut event = NostrEvent::unsigned(
        dvm_pubkey,
        created_at,
        DVM_FEEDBACK_KIND,
        public_tags,
        content,
    );
    event.sign_with_secret_hex(secret_hex)?;
    Ok(event)
}

fn random_iv() -> anyhow::Result<[u8; 16]> {
    let mut iv = [0u8; 16];
    getrandom::fill(&mut iv)
        .map_err(|error| anyhow::anyhow!("failed to generate feedback IV: {error}"))?;
    Ok(iv)
}

#[derive(Debug)]
struct DvmProcessOutcome {
    feedback: NostrEvent,
}

async fn publish_feedback_to_relays(relay_urls: &[String], event: &NostrEvent) {
    for relay_url in relay_urls {
        match publish_feedback_to_relay(relay_url, event).await {
            Ok(()) => {
                tracing::info!(%relay_url, event_id = ?event.id, "published DVM feedback");
            }
            Err(error) => {
                tracing::warn!(%relay_url, event_id = ?event.id, %error, "failed to publish DVM feedback");
            }
        }
    }
}

async fn publish_feedback_to_relay(relay_url: &str, event: &NostrEvent) -> anyhow::Result<()> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        anyhow::bail!("invalid relay URL");
    }

    let event_id = event.id.as_deref().context("feedback event missing id")?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .context("relay connection timed out")??;

    socket
        .send(Message::Text(
            serde_json::json!(["EVENT", event]).to_string(),
        ))
        .await?;

    loop {
        let Some(message) = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .context("relay OK timed out")?
        else {
            anyhow::bail!("relay closed before OK");
        };
        let Message::Text(text) = message? else {
            continue;
        };
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let Some(array) = value.as_array() else {
            continue;
        };
        if array.first().and_then(serde_json::Value::as_str) != Some("OK") {
            continue;
        }
        if array.get(1).and_then(serde_json::Value::as_str) != Some(event_id) {
            continue;
        }

        let accepted = array
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if accepted {
            return Ok(());
        }

        let relay_message = array
            .get(3)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("relay rejected feedback event");
        anyhow::bail!(relay_message.to_string());
    }
}

async fn mark_dvm_request_error(
    pool: &PgPool,
    dvm_request_id: Uuid,
    error: String,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE dvm_requests SET status = 'error', error = $2 WHERE id = $1")
        .bind(dvm_request_id)
        .bind(error)
        .execute(pool)
        .await?;

    Ok(())
}

fn parse_relay_urls(relay_urls: &str) -> Vec<String> {
    relay_urls
        .split(',')
        .map(str::trim)
        .filter(|relay_url| !relay_url.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_core::Pubkey;

    fn request_event() -> DvmRequestEvent {
        DvmRequestEvent {
            id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            pubkey: Pubkey::parse(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )
            .unwrap(),
            created_at: 1_776_432_000,
            kind: DVM_SCHEDULE_KIND,
            tags: vec![],
            content: String::new(),
            sig: Some("sig".into()),
        }
    }

    #[test]
    fn parses_relay_event_for_matching_subscription() {
        let raw = serde_json::json!(["EVENT", "sub", request_event()]).to_string();
        let parsed = parse_relay_event_message(&raw, "sub").unwrap().unwrap();
        assert_eq!(parsed.kind, DVM_SCHEDULE_KIND);
    }

    #[test]
    fn ignores_other_subscription_ids() {
        let raw = serde_json::json!(["EVENT", "other", request_event()]).to_string();
        assert!(parse_relay_event_message(&raw, "sub").unwrap().is_none());
    }

    #[test]
    fn splits_relay_urls() {
        assert_eq!(
            parse_relay_urls("wss://a.example, wss://b.example ,,"),
            vec!["wss://a.example", "wss://b.example"]
        );
    }
}
