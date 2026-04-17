use anyhow::Context;
use chrono::{DateTime, Utc};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use shipyard_core::NostrEvent;
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
                .unwrap_or_else(|_| "shipyard_worker=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let worker_id = std::env::var("SHIPYARD_WORKER_ID")
        .unwrap_or_else(|_| format!("shipyard-worker-{}", std::process::id()));
    let tick_seconds = std::env::var("SHIPYARD_WORKER_TICK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);

    tracing::info!(%worker_id, tick_seconds, "shipyard-worker starting");

    loop {
        tokio::select! {
            result = run_once(&pool, &worker_id) => {
                if let Err(error) = result {
                    tracing::error!(%error, "worker tick failed");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shipyard-worker shutting down");
                break;
            }
        }

        tokio::time::sleep(Duration::from_secs(tick_seconds)).await;
    }

    Ok(())
}

async fn run_once(pool: &PgPool, worker_id: &str) -> anyhow::Result<()> {
    while let Some(job) = claim_job(pool, worker_id).await? {
        let result = match job.kind.as_str() {
            "publish_event" | "retry_publish_event" => process_publish_event(pool, &job).await,
            other => {
                tracing::warn!(job_id = %job.id, kind = other, "unsupported job kind");
                Ok(())
            }
        };

        match result {
            Ok(()) => mark_job_succeeded(pool, job.id).await?,
            Err(error) => {
                tracing::error!(job_id = %job.id, %error, "job failed");
                mark_job_failed(pool, &job, error.to_string()).await?;
            }
        }
    }

    Ok(())
}

async fn claim_job(pool: &PgPool, worker_id: &str) -> anyhow::Result<Option<Job>> {
    let mut tx = pool.begin().await?;
    let row = sqlx::query(
        "SELECT id, kind::text AS kind, attempts, max_attempts, payload
         FROM jobs
         WHERE status = 'ready' AND run_at <= now()
         ORDER BY run_at ASC, created_at ASC
         FOR UPDATE SKIP LOCKED
         LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };

    let job = Job {
        id: row.try_get("id")?,
        kind: row.try_get("kind")?,
        attempts: row.try_get("attempts")?,
        max_attempts: row.try_get("max_attempts")?,
        payload: row.try_get("payload")?,
    };

    sqlx::query(
        "UPDATE jobs
         SET status = 'running',
             locked_at = now(),
             locked_by = $2,
             attempts = attempts + 1,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(job.id)
    .bind(worker_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Some(Job {
        attempts: job.attempts + 1,
        ..job
    }))
}

async fn process_publish_event(pool: &PgPool, job: &Job) -> anyhow::Result<()> {
    let payload: PublishEventPayload = serde_json::from_value(job.payload.clone())
        .context("publish_event payload must include publish_item_id")?;

    let row = sqlx::query(
        "SELECT p.id, p.owner_pubkey, p.signed_event_json, p.publish_time, r.relay_urls
         FROM publish_items p
         LEFT JOIN relay_settings r ON r.owner_pubkey = p.owner_pubkey
         WHERE p.id = $1 AND p.state IN ('SCHEDULED', 'FAILED')",
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
    let publish_time: DateTime<Utc> = row.try_get("publish_time")?;
    let relay_urls: Vec<String> = row
        .try_get::<Option<Vec<String>>, _>("relay_urls")?
        .unwrap_or_default();

    if relay_urls.is_empty() {
        fail_publish_item(
            pool,
            payload.publish_item_id,
            "no_relays_configured",
            "No relays set up. Add at least one in Settings before publishing.",
        )
        .await?;
        return Ok(());
    }

    if publish_time > Utc::now() {
        reschedule_job(pool, job.id, publish_time).await?;
        return Ok(());
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
        fail_publish_item(
            pool,
            payload.publish_item_id,
            "relay_publish_failed",
            "None of your relays accepted this post. Check relay settings.",
        )
        .await?;
    } else {
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
        .bind(payload.publish_item_id)
        .bind(&accepted_relays)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn publish_to_relay(relay_url: &str, event: &NostrEvent) -> Result<(), String> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        return Err("invalid relay URL".to_string());
    }

    let event_id = event
        .id
        .as_deref()
        .ok_or_else(|| "event missing id".to_string())?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .map_err(|_| "relay connection timed out".to_string())?
        .map_err(|error| format!("relay connection failed: {error}"))?;

    let message = serde_json::json!(["EVENT", event]).to_string();
    socket
        .send(Message::Text(message))
        .await
        .map_err(|error| format!("relay send failed: {error}"))?;

    loop {
        let next = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .map_err(|_| "relay OK timed out".to_string())?;

        let Some(message) = next else {
            return Err("relay closed before OK".to_string());
        };
        let message = message.map_err(|error| format!("relay read failed: {error}"))?;
        let Message::Text(text) = message else {
            continue;
        };

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("relay sent invalid JSON: {error}"))?;
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
        let relay_message = array
            .get(3)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("relay rejected event");
        return if accepted {
            Ok(())
        } else {
            Err(relay_message.to_string())
        };
    }
}

async fn record_publish_attempt(
    pool: &PgPool,
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
        "INSERT INTO publish_attempts (publish_item_id, attempt, relay_url, status, error)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(publish_item_id)
    .bind(attempt)
    .bind(relay_url)
    .bind(status)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

async fn fail_publish_item(
    pool: &PgPool,
    publish_item_id: Uuid,
    failure_code: &str,
    failure_message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE publish_items
         SET state = 'FAILED',
             failure_code = $2,
             failure_message = $3,
             failed_at = now(),
             updated_at = now()
         WHERE id = $1",
    )
    .bind(publish_item_id)
    .bind(failure_code)
    .bind(failure_message)
    .execute(pool)
    .await?;

    Ok(())
}

async fn reschedule_job(pool: &PgPool, job_id: Uuid, run_at: DateTime<Utc>) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs
         SET status = 'ready',
             run_at = $2,
             locked_at = NULL,
             locked_by = NULL,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(job_id)
    .bind(run_at)
    .execute(pool)
    .await?;

    Ok(())
}

async fn mark_job_succeeded(pool: &PgPool, job_id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs
         SET status = 'succeeded',
             locked_at = NULL,
             locked_by = NULL,
             updated_at = now()
         WHERE id = $1 AND status = 'running'",
    )
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn mark_job_failed(pool: &PgPool, job: &Job, error: String) -> anyhow::Result<()> {
    let exhausted = job.attempts >= job.max_attempts;
    let status = if exhausted { "failed" } else { "ready" };
    let run_at = if exhausted {
        Utc::now()
    } else {
        Utc::now() + chrono::Duration::seconds(30 * i64::from(job.attempts))
    };

    sqlx::query(
        "UPDATE jobs
         SET status = $2::job_status,
             run_at = $3,
             locked_at = NULL,
             locked_by = NULL,
             last_error = $4,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(job.id)
    .bind(status)
    .bind(run_at)
    .bind(error)
    .execute(pool)
    .await?;

    Ok(())
}

#[derive(Debug)]
struct Job {
    id: Uuid,
    kind: String,
    attempts: i32,
    max_attempts: i32,
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PublishEventPayload {
    publish_item_id: Uuid,
}
