use anyhow::Context;
use chrono::Utc;
use shipyard_core::{
    dvm::{build_signed_feedback_event, feedback_tags, DvmRequestEvent, DVM_FEEDBACK_KIND},
    nip04_decrypt, nip04_encrypt, pubkey_from_secret_hex, NostrEvent, Pubkey,
};

pub(crate) fn build_error_feedback(
    feedback_secret_hex: &str,
    request_event_id: &str,
    raw_request_event: serde_json::Value,
    error_message: &str,
) -> anyhow::Result<NostrEvent> {
    match serde_json::from_value::<DvmRequestEvent>(raw_request_event) {
        Ok(request_event) if has_encrypted_tag(&request_event) => build_encrypted_feedback_event(
            feedback_secret_hex,
            &request_event.pubkey,
            "error",
            request_event_id,
            Some(error_message),
            Utc::now().timestamp(),
        ),
        _ => Ok(build_signed_feedback_event(
            feedback_secret_hex,
            "error",
            request_event_id,
            Some(error_message),
            Utc::now().timestamp(),
        )?),
    }
}

pub(crate) fn has_encrypted_tag(request_event: &DvmRequestEvent) -> bool {
    request_event
        .tags
        .iter()
        .any(|tag| tag.first().map(String::as_str) == Some("encrypted"))
}

pub(crate) fn decrypt_request_tags(
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

pub(crate) fn build_encrypted_feedback_event(
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
