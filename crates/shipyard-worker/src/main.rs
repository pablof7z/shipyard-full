use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod backoff;
mod publish;
mod relay;

use backoff::{configured_base_backoff_seconds, retries_exhausted, retry_backoff_seconds};
use publish::{fail_publish_item, process_publish_event, PublishJobFailure};

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
    #[cfg(unix)]
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("failed to install SIGTERM handler")?;

    tracing::info!(%worker_id, tick_seconds, "shipyard-worker starting");

    loop {
        #[cfg(unix)]
        let shutdown = shutdown_signal(&mut sigterm);
        #[cfg(not(unix))]
        let shutdown = shutdown_signal();

        tokio::select! {
            result = run_once(&pool, &worker_id) => {
                if let Err(error) = result {
                    tracing::error!(%error, "worker tick failed");
                }
            }
            _ = shutdown => {
                tracing::info!("shipyard-worker shutting down");
                break;
            }
        }

        tokio::time::sleep(Duration::from_secs(tick_seconds)).await;
    }

    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal(sigterm: &mut tokio::signal::unix::Signal) {
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = sigterm.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
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
                mark_job_failed(pool, &job, error).await?;
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
         WHERE state = 'ready' AND available_after <= now()
         ORDER BY available_after ASC, created_at ASC
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
         SET state = 'running',
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

pub(crate) async fn reschedule_job(
    pool: &PgPool,
    job_id: Uuid,
    run_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE jobs
         SET state = 'ready',
             available_after = $2,
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
         SET state = 'succeeded',
             locked_at = NULL,
             locked_by = NULL,
             updated_at = now()
         WHERE id = $1 AND state = 'running'",
    )
    .bind(job_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn mark_job_failed(pool: &PgPool, job: &Job, error: anyhow::Error) -> anyhow::Result<()> {
    let publish_failure = error.downcast_ref::<PublishJobFailure>().cloned();
    let error = error.to_string();
    let exhausted = retries_exhausted(job.attempts, job.max_attempts);
    let state = if exhausted { "failed" } else { "ready" };
    let available_after = if exhausted {
        Utc::now()
    } else {
        let base_backoff_seconds = configured_base_backoff_seconds();
        Utc::now()
            + chrono::Duration::seconds(retry_backoff_seconds(job.attempts, base_backoff_seconds))
    };

    sqlx::query(
        "UPDATE jobs
         SET state = $2::job_status,
             available_after = $3,
             locked_at = NULL,
             locked_by = NULL,
             last_error = $4,
             updated_at = now()
         WHERE id = $1",
    )
    .bind(job.id)
    .bind(state)
    .bind(available_after)
    .bind(error)
    .execute(pool)
    .await?;

    if exhausted {
        if let Some(failure) = publish_failure {
            fail_publish_item(
                pool,
                failure.publish_item_id,
                failure.failure_code,
                failure.failure_message,
            )
            .await?;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) struct Job {
    pub(crate) id: Uuid,
    pub(crate) kind: String,
    pub(crate) attempts: i32,
    pub(crate) max_attempts: i32,
    pub(crate) payload: serde_json::Value,
}
