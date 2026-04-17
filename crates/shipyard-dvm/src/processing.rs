use anyhow::Context;
use chrono::{DateTime, Utc};
use shipyard_core::{
    dvm::{
        build_signed_feedback_event, parse_dvm_request, parse_encrypted_dvm_request,
        DvmRequestEvent,
    },
    NostrEvent,
};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::{
    feedback::{
        build_encrypted_feedback_event, build_error_feedback, decrypt_request_tags,
        has_encrypted_tag,
    },
    relay::publish_feedback_to_relays,
};

pub(crate) async fn process_pending_dvm_requests(
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
                let feedback = build_error_feedback(
                    feedback_secret_hex,
                    &request_event_id,
                    raw_request_event,
                    &error_message,
                )?;
                publish_feedback_to_relays(relay_urls, &feedback).await;
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
) -> anyhow::Result<DvmProcessOutcome> {
    let request_event: DvmRequestEvent =
        serde_json::from_value(raw_request_event).context("stored DVM request event is invalid")?;
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
    let feedback_event_id = feedback.id.clone();
    let feedback_content = feedback.content.clone();
    let feedback_pubkey = feedback.pubkey.as_str().to_string();
    let decrypted_tags_json = serde_json::to_value(&decrypted_tags)?;

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
            "INSERT INTO jobs (kind, available_after, payload)
             VALUES ('publish_event', $1, jsonb_build_object('publish_item_id', $2::text))",
        )
        .bind(publish_time)
        .bind(publish_item_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        "UPDATE dvm_requests
         SET status = 'scheduled',
             error = NULL,
             decrypted_tags = $2,
             relays = $3,
             feedback_event_id = $4,
             feedback_content = $5,
             feedback_pubkey = $6,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(dvm_request_id)
    .bind(decrypted_tags_json)
    .bind(&parsed.relay_targets)
    .bind(feedback_event_id)
    .bind(feedback_content)
    .bind(feedback_pubkey)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(DvmProcessOutcome { feedback })
}

async fn mark_dvm_request_error(
    pool: &PgPool,
    dvm_request_id: Uuid,
    error: String,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE dvm_requests
         SET status = 'error', error = $2, updated_at = now()
         WHERE id = $1",
    )
    .bind(dvm_request_id)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug)]
struct DvmProcessOutcome {
    feedback: NostrEvent,
}
