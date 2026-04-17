use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::{next_queue_slot, Pubkey, Queue};
use uuid::Uuid;

use super::models::{
    fetch_queue, owner_from_headers_or_self, queue_owner, require_account_access, require_owner,
    require_session, row_to_queue, ApiState, AppError,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/queues", get(list_queues).post(create_queue))
        .route("/queues/:queue_id", patch(update_queue))
        .route("/queues/:queue_id/next-slot", get(next_queue_slot_route))
        .route("/queues/:queue_id/archive", post(archive_queue))
}

async fn list_queues(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Queue>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(
        "SELECT id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at
         FROM queues
         WHERE owner_pubkey = $1
         ORDER BY archived_at NULLS FIRST, created_at DESC",
    )
    .bind(owner_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    let queues = rows
        .into_iter()
        .map(row_to_queue)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(queues))
}

async fn create_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CreateQueueRequest>,
) -> Result<(axum::http::StatusCode, Json<Queue>), AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_owner(&session, &owner_pubkey)?;

    if request.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "queue_name_required",
            "Queue name is required.",
        ));
    }
    if request.cadence_seconds <= 0 {
        return Err(AppError::bad_request(
            "queue_cadence_invalid",
            "Queue cadence must be positive.",
        ));
    }

    let row = sqlx::query(
        "INSERT INTO queues (owner_pubkey, name, description, cadence_seconds, start_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(request.name.trim())
    .bind(request.description)
    .bind(request.cadence_seconds)
    .bind(request.start_at)
    .fetch_one(&state.pool)
    .await?;

    Ok((axum::http::StatusCode::CREATED, Json(row_to_queue(row)?)))
}

async fn update_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
    Json(request): Json<UpdateQueueRequest>,
) -> Result<Json<Queue>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = queue_owner(&state, queue_id).await?;
    require_owner(&session, &owner_pubkey)?;

    if request
        .name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "queue_name_required",
            "Queue name is required.",
        ));
    }
    if request
        .cadence_seconds
        .is_some_and(|cadence_seconds| cadence_seconds <= 0)
    {
        return Err(AppError::bad_request(
            "queue_cadence_invalid",
            "Queue cadence must be positive.",
        ));
    }
    let name = request.name.map(|name| name.trim().to_string());

    let row = sqlx::query(
        "UPDATE queues
         SET name = COALESCE($2, name),
             description = COALESCE($3, description),
             cadence_seconds = COALESCE($4, cadence_seconds),
             start_at = COALESCE($5, start_at),
             updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(queue_id)
    .bind(name)
    .bind(request.description)
    .bind(request.cadence_seconds)
    .bind(request.start_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row_to_queue(row)?))
}

async fn archive_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
) -> Result<Json<Queue>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = queue_owner(&state, queue_id).await?;
    require_owner(&session, &owner_pubkey)?;

    let row = sqlx::query(
        "UPDATE queues
         SET archived_at = COALESCE(archived_at, now()), updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(queue_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row_to_queue(row)?))
}

async fn next_queue_slot_route(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
) -> Result<Json<QueueNextSlotResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let queue = fetch_queue(&state, queue_id).await?;
    require_account_access(&state, &session, &queue.owner_pubkey).await?;

    let latest_queue_slot: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT max(publish_time)
         FROM publish_items
         WHERE queue_id = $1
           AND publish_time IS NOT NULL
           AND state NOT IN ('REJECTED', 'CANCELLED', 'FAILED')",
    )
    .bind(queue_id)
    .fetch_one(&state.pool)
    .await?;
    let now = Utc::now();
    let next_slot = next_queue_slot(&queue, now, latest_queue_slot)
        .map_err(|err| AppError::bad_request("queue_next_slot_unavailable", err.to_string()))?;

    Ok(Json(QueueNextSlotResponse {
        queue_id,
        owner_pubkey: queue.owner_pubkey,
        next_slot,
        latest_queue_slot,
        now,
    }))
}

#[derive(Debug, Deserialize)]
struct CreateQueueRequest {
    name: String,
    description: Option<String>,
    cadence_seconds: i64,
    start_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpdateQueueRequest {
    name: Option<String>,
    description: Option<String>,
    cadence_seconds: Option<i64>,
    start_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct QueueNextSlotResponse {
    queue_id: Uuid,
    owner_pubkey: Pubkey,
    next_slot: DateTime<Utc>,
    latest_queue_slot: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
}
