mod failure;

use anyhow::Context;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use shipyard_core::{
    dvm::{parse_dvm_request, parse_encrypted_dvm_request, DvmFeedbackMetadata, DvmRequestEvent},
    pubkey_from_secret_hex, NostrEvent,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    feedback::{build_feedback_event, decrypt_request_tags, has_encrypted_tag},
    relay::publish_feedback_to_relays,
};
use failure::{build_failure_outcome, mark_dvm_request_failed};

const DEFAULT_DVM_REQUEST_MAX_AGE_MINUTES: i64 = 10;
const DVM_SUCCESS_STATUS: &str = "succeeded";
const DVM_FAILURE_STATUS: &str = "failed";
const DVM_SUCCESS_RESULT: &str = "scheduled";
const DVM_FAILURE_RESULT: &str = "failed";
const DVM_FAILURE_CODE: &str = "processing_error";

pub(crate) async fn process_pending_dvm_requests(
    pool: &PgPool,
    feedback_secret_hex: &str,
) -> anyhow::Result<()> {
    let dvm_pubkey = pubkey_from_secret_hex(feedback_secret_hex)?;
    let max_age_minutes = request_max_age_minutes();
    let rows = sqlx::query(
        "UPDATE dvm_requests
         SET status = 'processing', dvm_pubkey = $1, updated_at = now()
         WHERE id IN (
             SELECT id
             FROM dvm_requests
             WHERE (
                 (status = 'pending' AND (dvm_pubkey = $1 OR dvm_pubkey = ''))
                 OR (
                     status = 'processing'
                     AND dvm_pubkey = $1
                     AND updated_at < now() - INTERVAL '5 minutes'
                 )
             )
             ORDER BY created_at ASC
             LIMIT 25
             FOR UPDATE SKIP LOCKED
         )
         RETURNING id, request_event_id, raw_request_event, relays",
    )
    .bind(dvm_pubkey.as_str())
    .fetch_all(pool)
    .await?;

    for row in rows {
        let dvm_request_id: Uuid = row.try_get("id")?;
        let request_event_id: String = row.try_get("request_event_id")?;
        let raw_request_event: serde_json::Value = row.try_get("raw_request_event")?;
        let stored_relays: Vec<String> = row.try_get("relays")?;
        let result = process_one_dvm_request(
            pool,
            dvm_request_id,
            raw_request_event.clone(),
            feedback_secret_hex,
            dvm_pubkey.as_str(),
            max_age_minutes,
        )
        .await;
        match result {
            Ok(outcome) => {
                publish_feedback_to_relays(&outcome.relay_targets, &outcome.feedback).await;
                tracing::info!(%request_event_id, feedback_id = ?outcome.feedback.id, "DVM request succeeded");
            }
            Err(error) => {
                let failure_message = error.to_string();
                tracing::warn!(%request_event_id, error = %failure_message, "DVM request failed");
                let failure = build_failure_outcome(
                    feedback_secret_hex,
                    &request_event_id,
                    raw_request_event,
                    stored_relays,
                    &failure_message,
                )?;
                mark_dvm_request_failed(
                    pool,
                    dvm_request_id,
                    DVM_FAILURE_CODE,
                    &failure_message,
                    &failure.feedback,
                )
                .await?;
                publish_feedback_to_relays(&failure.relay_targets, &failure.feedback).await;
            }
        }
    }

    Ok(())
}

async fn process_one_dvm_request(
    pool: &PgPool,
    dvm_request_id: Uuid,
    raw_request_event: serde_json::Value,
    feedback_secret_hex: &str,
    dvm_pubkey: &str,
    max_age_minutes: i64,
) -> anyhow::Result<DvmProcessOutcome> {
    let request_event: DvmRequestEvent =
        serde_json::from_value(raw_request_event).context("stored DVM request event is invalid")?;
    validate_request_event(&request_event, Utc::now(), max_age_minutes)?;

    let encrypted = has_encrypted_tag(&request_event);
    let decrypted_tags = if encrypted {
        decrypt_request_tags(&request_event, feedback_secret_hex)?
    } else {
        request_event.tags.clone()
    };
    let parsed = if encrypted {
        parse_encrypted_dvm_request(&request_event, decrypted_tags.clone())?
    } else {
        parse_dvm_request(&request_event)?
    };
    let feedback = build_feedback_event(
        feedback_secret_hex,
        parsed.encrypted,
        DvmFeedbackMetadata {
            status: DVM_SUCCESS_STATUS,
            request_event_id: &parsed.request_event_id,
            recipient_pubkey: &parsed.request_pubkey,
            method: &parsed.method,
            result: DVM_SUCCESS_RESULT,
            message: Some("Scheduled."),
        },
        Utc::now().timestamp(),
    )?;
    let feedback_event_id = feedback.id.clone();
    let feedback_content = feedback.content.clone();
    let feedback_pubkey = feedback.pubkey.as_str().to_string();
    let decrypted_tags_json = serde_json::to_value(&decrypted_tags)?;
    let relay_targets = parsed.relay_targets.clone();

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
        .bind(&event_id)
        .bind(publish_time)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO jobs (kind, available_after, payload)
             SELECT 'publish_event', $1, jsonb_build_object('publish_item_id', $2::text)
             WHERE NOT EXISTS (
                 SELECT 1 FROM jobs
                 WHERE kind = 'publish_event'
                   AND payload->>'publish_item_id' = $2::text
                   AND state IN ('ready', 'running', 'succeeded')
             )",
        )
        .bind(publish_time)
        .bind(publish_item_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE dvm_requests
         SET status = 'succeeded',
             error = NULL,
             failure_code = NULL,
             failure_message = NULL,
             input_event_id = CASE
                 WHEN NOT EXISTS (
                     SELECT 1 FROM dvm_requests existing
                     WHERE existing.input_event_id = $4
                       AND existing.dvm_pubkey = $8
                       AND existing.id <> $1
                 ) THEN $4
                 ELSE input_event_id
             END,
             decrypted_tags = $2,
             relays = $3,
             feedback_event_id = $5,
             feedback_content = $6,
             feedback_pubkey = $7,
             updated_at = now()
         WHERE id = $1 AND status = 'processing'",
    )
    .bind(dvm_request_id)
    .bind(decrypted_tags_json)
    .bind(&relay_targets)
    .bind(&parsed.target_event_id)
    .bind(feedback_event_id)
    .bind(feedback_content)
    .bind(feedback_pubkey)
    .bind(dvm_pubkey)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(DvmProcessOutcome {
        feedback,
        relay_targets,
    })
}

fn validate_request_event(
    request_event: &DvmRequestEvent,
    now: DateTime<Utc>,
    max_age_minutes: i64,
) -> anyhow::Result<()> {
    let outer_event = NostrEvent::from(request_event);
    outer_event
        .validate_signed_for_owner(&request_event.pubkey, None)
        .context("DVM request signature is invalid")?;
    validate_request_freshness_at(request_event, now, max_age_minutes)
}

fn validate_request_freshness_at(
    request_event: &DvmRequestEvent,
    now: DateTime<Utc>,
    max_age_minutes: i64,
) -> anyhow::Result<()> {
    let created_at = DateTime::<Utc>::from_timestamp(request_event.created_at, 0)
        .context("DVM request created_at is invalid")?;
    let max_age = ChronoDuration::minutes(max_age_minutes.max(1));
    if created_at < now - max_age {
        anyhow::bail!("DVM request is stale");
    }
    Ok(())
}

fn request_max_age_minutes() -> i64 {
    std::env::var("SHIPYARD_DVM_REQUEST_MAX_AGE_MINUTES")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|minutes| *minutes > 0)
        .unwrap_or(DEFAULT_DVM_REQUEST_MAX_AGE_MINUTES)
}

#[derive(Debug)]
struct DvmProcessOutcome {
    feedback: NostrEvent,
    relay_targets: Vec<String>,
}

#[cfg(test)]
#[path = "processing_tests.rs"]
mod tests;
