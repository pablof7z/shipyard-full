use anyhow::Context;
use chrono::Utc;
use shipyard_core::{
    dvm::{method_from_tags, relay_targets_from_tags, DvmFeedbackMetadata, DvmRequestEvent},
    NostrEvent,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::feedback::{build_feedback_event, decrypt_request_tags, has_encrypted_tag};

use super::{DVM_FAILURE_RESULT, DVM_FAILURE_STATUS};

#[derive(Debug)]
pub(super) struct DvmFailureOutcome {
    pub(super) feedback: NostrEvent,
    pub(super) relay_targets: Vec<String>,
}

pub(super) fn build_failure_outcome(
    feedback_secret_hex: &str,
    request_event_id: &str,
    raw_request_event: serde_json::Value,
    stored_relays: Vec<String>,
    failure_message: &str,
) -> anyhow::Result<DvmFailureOutcome> {
    let request_event: DvmRequestEvent =
        serde_json::from_value(raw_request_event).context("stored DVM request event is invalid")?;
    let encrypted = has_encrypted_tag(&request_event);
    let decrypted_tags = if encrypted {
        decrypt_request_tags(&request_event, feedback_secret_hex).unwrap_or_default()
    } else {
        request_event.tags.clone()
    };
    let method = method_from_tags(&decrypted_tags);
    let mut relay_targets = relay_targets_from_tags(&decrypted_tags);
    if relay_targets.is_empty() {
        relay_targets = stored_relays;
    }
    let feedback = build_feedback_event(
        feedback_secret_hex,
        encrypted,
        DvmFeedbackMetadata {
            status: DVM_FAILURE_STATUS,
            request_event_id,
            recipient_pubkey: &request_event.pubkey,
            method: &method,
            result: DVM_FAILURE_RESULT,
            message: Some(failure_message),
        },
        Utc::now().timestamp(),
    )?;

    Ok(DvmFailureOutcome {
        feedback,
        relay_targets,
    })
}

pub(super) async fn mark_dvm_request_failed(
    pool: &PgPool,
    dvm_request_id: Uuid,
    failure_code: &str,
    failure_message: &str,
    feedback: &NostrEvent,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE dvm_requests
         SET status = 'failed',
             error = $2,
             failure_code = $3,
             failure_message = $4,
             feedback_event_id = $5,
             feedback_content = $6,
             feedback_pubkey = $7,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(dvm_request_id)
    .bind(failure_message)
    .bind(failure_code)
    .bind(failure_message)
    .bind(feedback.id.clone())
    .bind(feedback.content.clone())
    .bind(feedback.pubkey.as_str())
    .execute(pool)
    .await?;

    Ok(())
}
