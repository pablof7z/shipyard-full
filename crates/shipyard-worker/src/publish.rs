use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shipyard_core::{assert_transition, Actor, NostrEvent, PublishState};
use sqlx::{PgPool, Row};
use std::fmt;
use uuid::Uuid;

use crate::{relay::publish_to_relay, Job};

pub(crate) async fn process_publish_event(pool: &PgPool, job: &Job) -> anyhow::Result<()> {
    let payload: PublishEventPayload = serde_json::from_value(job.payload.clone())
        .context("publish_event payload must include publish_item_id")?;

    let row = sqlx::query(
        "SELECT p.id, p.owner_pubkey, p.state::text AS state,
                p.signed_event_json, p.publish_time, r.relay_urls
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
    let current_state: String = row.try_get("state")?;
    let publish_time: DateTime<Utc> = row.try_get("publish_time")?;
    let relay_urls: Vec<String> = row
        .try_get::<Option<Vec<String>>, _>("relay_urls")?
        .unwrap_or_default();

    if publish_time > Utc::now() {
        crate::reschedule_job(pool, job.id, publish_time).await?;
        return Ok(());
    }

    if !transition_publish_item_to_publishing(pool, payload.publish_item_id, &current_state).await?
    {
        return Ok(());
    }

    if relay_urls.is_empty() {
        return Err(PublishJobFailure {
            publish_item_id: payload.publish_item_id,
            failure_code: "no_relays_configured",
            failure_message: "No relays set up. Add at least one in Settings before publishing.",
        }
        .into());
    }

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
        return Err(PublishJobFailure {
            publish_item_id: payload.publish_item_id,
            failure_code: "relay_publish_failed",
            failure_message: "None of your relays accepted this post. Check relay settings.",
        }
        .into());
    }
    publish_item_succeeded(pool, payload.publish_item_id, &accepted_relays).await?;

    Ok(())
}

#[derive(Clone, Debug)]
pub(crate) struct PublishJobFailure {
    pub(crate) publish_item_id: Uuid,
    pub(crate) failure_code: &'static str,
    pub(crate) failure_message: &'static str,
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
    accepted_relays: &[String],
) -> anyhow::Result<()> {
    assert_worker_transition("PUBLISHING", PublishState::Published)?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'PUBLISHED',
             published_at = now(),
             published_to = $2,
             failure_code = NULL,
             failure_message = NULL,
             failed_at = NULL,
             updated_at = now()
         WHERE id = $1 AND state = 'PUBLISHING'
         RETURNING owner_pubkey",
    )
    .bind(publish_item_id)
    .bind(accepted_relays)
    .fetch_optional(&mut *tx)
    .await?;

    if let Some(row) = row {
        let owner_pubkey: String = row.try_get("owner_pubkey")?;
        record_publish_state_change_audit(&mut *tx, &owner_pubkey, publish_item_id, "PUBLISHED")
            .await?;
    }
    tx.commit().await?;

    Ok(())
}

async fn transition_publish_item_to_publishing(
    pool: &PgPool,
    publish_item_id: Uuid,
    current_state: &str,
) -> anyhow::Result<bool> {
    if current_state == "PUBLISHING" {
        return Ok(true);
    }

    assert_worker_transition(current_state, PublishState::Publishing)?;
    let result = sqlx::query(
        "UPDATE publish_items
         SET state = 'PUBLISHING', updated_at = now()
         WHERE id = $1 AND state = $2::publish_state",
    )
    .bind(publish_item_id)
    .bind(current_state)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() == 1)
}

fn assert_worker_transition(from: &str, to: PublishState) -> anyhow::Result<()> {
    let from: PublishState = serde_json::from_value(serde_json::Value::String(from.to_string()))
        .with_context(|| format!("stored publish state is invalid: {from}"))?;
    assert_transition(Actor::Worker, from, to)
        .map_err(|err| anyhow::anyhow!("invalid worker publish state transition: {err}"))
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
    let error = (status_or_error != "accepted").then_some(status_or_error);
    let status = error.map_or("accepted", |_| "error");

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
    assert_worker_transition("PUBLISHING", PublishState::Failed)?;
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'FAILED',
             failure_code = $2,
             failure_message = $3,
             failed_at = now(),
             updated_at = now()
         WHERE id = $1 AND state = 'PUBLISHING'
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
        enqueue_publish_failure_notification(
            &mut tx,
            publish_item_id,
            &owner_pubkey,
            failure_code,
            failure_message,
        )
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

async fn enqueue_publish_failure_notification(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    publish_item_id: Uuid,
    owner_pubkey: &str,
    failure_code: &str,
    failure_message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO jobs (kind, payload)
         VALUES ('send_notification', $1)",
    )
    .bind(serde_json::json!({
        "type": "publish_failed",
        "publish_item_id": publish_item_id,
        "owner_pubkey": owner_pubkey,
        "failure_code": failure_code,
        "failure_message": failure_message
    }))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
struct PublishEventPayload {
    publish_item_id: Uuid,
}
