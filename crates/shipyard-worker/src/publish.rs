use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shipyard_core::NostrEvent;
use sqlx::{PgPool, Row};
use std::fmt;
use uuid::Uuid;

use crate::{relay::publish_to_relay, Job};

pub(crate) async fn process_publish_event(pool: &PgPool, job: &Job) -> anyhow::Result<()> {
    let payload: PublishEventPayload = serde_json::from_value(job.payload.clone())
        .context("publish_event payload must include publish_item_id")?;

    let row = sqlx::query(
        "SELECT p.id, p.owner_pubkey, p.signed_event_json, p.publish_time, r.relay_urls
         FROM publish_items p
         LEFT JOIN relay_settings r ON r.owner_pubkey = p.owner_pubkey
         WHERE p.id = $1 AND p.state IN ('SCHEDULED', 'PUBLISHING', 'FAILED')",
    )
    .bind(payload.publish_item_id)
    .fetch_optional(pool)
    .await?;

    let Some(row) = row else {
        tracing::warn!(publish_item_id = %payload.publish_item_id, "publish item not ready");
        return Ok(());
    };

    let signed_event_json: serde_json::Value = row.try_get("signed_event_json")?;
    let signed_event: NostrEvent =
        serde_json::from_value(signed_event_json).context("stored signed event is invalid")?;
    let owner_pubkey: String = row.try_get("owner_pubkey")?;
    let publish_time: DateTime<Utc> = row.try_get("publish_time")?;
    let relay_urls: Vec<String> = row
        .try_get::<Option<Vec<String>>, _>("relay_urls")?
        .unwrap_or_default();

    if publish_time > Utc::now() {
        crate::reschedule_job(pool, job.id, publish_time).await?;
        return Ok(());
    }

    if relay_urls.is_empty() {
        return Err(PublishJobFailure::new(
            payload.publish_item_id,
            "no_relays_configured",
            "No relays set up. Add at least one in Settings before publishing.",
        )
        .into());
    }

    sqlx::query("UPDATE publish_items SET state = 'PUBLISHING', updated_at = now() WHERE id = $1")
        .bind(payload.publish_item_id)
        .execute(pool)
        .await?;

    let attempt = job.attempts.max(1);
    let mut accepted_relays = Vec::new();
    for relay_url in &relay_urls {
        let result = publish_to_relay(relay_url, &signed_event).await;
        let status_or_error = result
            .as_ref()
            .err()
            .map(String::as_str)
            .unwrap_or("accepted");
        record_publish_attempt(
            pool,
            job.id,
            payload.publish_item_id,
            attempt,
            relay_url,
            status_or_error,
        )
        .await?;

        if result.is_ok() {
            accepted_relays.push(relay_url.clone());
        }
    }

    if accepted_relays.is_empty() {
        return Err(PublishJobFailure::new(
            payload.publish_item_id,
            "relay_publish_failed",
            "None of your relays accepted this post. Check relay settings.",
        )
        .into());
    } else {
        publish_item_succeeded(
            pool,
            payload.publish_item_id,
            &owner_pubkey,
            &accepted_relays,
        )
        .await?;
    }

    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) struct PublishJobFailure {
    pub(crate) publish_item_id: Uuid,
    pub(crate) failure_code: &'static str,
    pub(crate) failure_message: &'static str,
}

impl PublishJobFailure {
    fn new(
        publish_item_id: Uuid,
        failure_code: &'static str,
        failure_message: &'static str,
    ) -> Self {
        Self {
            publish_item_id,
            failure_code,
            failure_message,
        }
    }
}

impl fmt::Display for PublishJobFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.failure_code, self.failure_message)
    }
}

impl std::error::Error for PublishJobFailure {}

async fn publish_item_succeeded(
    pool: &PgPool,
    publish_item_id: Uuid,
    owner_pubkey: &str,
    accepted_relays: &[String],
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query(
        "UPDATE publish_items
         SET state = 'PUBLISHED',
             published_at = now(),
             published_to = $2,
             failure_code = NULL,
             failure_message = NULL,
             failed_at = NULL,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(publish_item_id)
    .bind(accepted_relays)
    .execute(&mut *tx)
    .await?;
    record_publish_state_change_audit(&mut *tx, owner_pubkey, publish_item_id, "PUBLISHED").await?;
    tx.commit().await?;

    Ok(())
}

async fn record_publish_state_change_audit<'e, E>(
    executor: E,
    owner_pubkey: &str,
    publish_item_id: Uuid,
    new_state: &str,
) -> anyhow::Result<()>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query(
        "INSERT INTO audit_events
            (actor_pubkey, owner_pubkey, action, resource_type, resource_id, metadata)
         VALUES
            (NULL, $1, 'PUBLISH_ITEM_STATE_CHANGE', 'publish_item', $2,
             jsonb_build_object('new_state', $3))",
    )
    .bind(owner_pubkey)
    .bind(publish_item_id.to_string())
    .bind(new_state)
    .execute(executor)
    .await?;

    Ok(())
}

async fn record_publish_attempt(
    pool: &PgPool,
    job_id: Uuid,
    publish_item_id: Uuid,
    attempt: i32,
    relay_url: &str,
    status_or_error: &str,
) -> anyhow::Result<()> {
    let status = if status_or_error == "accepted" {
        "accepted"
    } else {
        "error"
    };
    let error = (status == "error").then_some(status_or_error);

    sqlx::query(
        "INSERT INTO publish_attempts (publish_item_id, job_id, attempt, relay_url, status, error)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(publish_item_id)
    .bind(job_id)
    .bind(attempt)
    .bind(relay_url)
    .bind(status)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

pub(crate) async fn fail_publish_item(
    pool: &PgPool,
    publish_item_id: Uuid,
    failure_code: &str,
    failure_message: &str,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'FAILED',
             failure_code = $2,
             failure_message = $3,
             failed_at = now(),
             updated_at = now()
         WHERE id = $1
         RETURNING owner_pubkey",
    )
    .bind(publish_item_id)
    .bind(failure_code)
    .bind(failure_message)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(row) = row {
        let owner_pubkey: String = row.try_get("owner_pubkey")?;
        record_publish_state_change_audit(&mut *tx, &owner_pubkey, publish_item_id, "FAILED")
            .await?;
    }

    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PublishEventPayload {
    publish_item_id: Uuid,
}
