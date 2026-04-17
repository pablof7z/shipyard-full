mod model;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

pub(super) use model::{
    enqueue_publish_job, fetch_publish_item, parse_event_json, row_to_publish_item,
    validate_queue_for_owner, validate_trigger_inputs, PublishItemResponse,
    PUBLISH_ITEM_SELECT_CREATOR_STATE, PUBLISH_ITEM_SELECT_OWNER_STATE,
};

use super::models::{
    require_account_access, require_owner, require_session, ApiState, AppError,
    AuthenticatedSession,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/publish-items", get(list_publish_items))
        .route("/publish-items/schedule", post(schedule_signed_event))
        .route("/publish-items/send-now", post(send_now))
        .route(
            "/publish-items/:publish_item_id/cancel",
            post(cancel_publish_item),
        )
        .route(
            "/publish-items/:publish_item_id/retry",
            post(retry_publish_item),
        )
}

async fn list_publish_items(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PublishItemResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = super::models::owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(model::PUBLISH_ITEM_SELECT_OWNER)
        .bind(owner_pubkey.as_str())
        .fetch_all(&state.pool)
        .await?;

    rows.into_iter()
        .map(row_to_publish_item)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

async fn schedule_signed_event(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<ScheduleSignedEventRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    create_signed_publish_item(&state, &session, request, false).await
}

async fn send_now(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(mut request): Json<ScheduleSignedEventRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    request.trigger = "SEND_NOW".to_string();
    create_signed_publish_item(&state, &session, request, true).await
}

async fn cancel_publish_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;
    if matches!(item.state.as_str(), "PUBLISHED" | "PUBLISHING") {
        return Err(AppError::bad_request(
            "publish_item_not_cancellable",
            "Publishing or published items cannot be cancelled.",
        ));
    }

    sqlx::query(
        "UPDATE publish_items
         SET state = 'CANCELLED', updated_at = now()
         WHERE id = $1 AND state NOT IN ('PUBLISHED', 'PUBLISHING')",
    )
    .bind(publish_item_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn retry_publish_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;
    if item.state != "FAILED" {
        return Err(AppError::bad_request(
            "publish_item_not_retryable",
            "Only failed items can be retried.",
        ));
    }

    let next_state = if item.signed_event_json.is_some() {
        "SCHEDULED"
    } else {
        "NEEDS_SIGNATURE"
    };
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = $2::publish_state,
             failure_code = NULL,
             failure_message = NULL,
             failed_at = NULL,
             updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(next_state)
    .fetch_one(&mut *tx)
    .await?;

    if next_state == "SCHEDULED" {
        let publish_time = item.publish_time.unwrap_or_else(Utc::now);
        enqueue_publish_job(&mut tx, publish_item_id, publish_time).await?;
    }
    tx.commit().await?;

    Ok(Json(row_to_publish_item(row)?))
}

async fn create_signed_publish_item(
    state: &ApiState,
    session: &AuthenticatedSession,
    request: ScheduleSignedEventRequest,
    due_now: bool,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let signed_event = parse_event_json(&request.signed_event)?;
    let owner_pubkey = signed_event.pubkey.clone();
    require_owner(session, &owner_pubkey)?;

    let publish_time = if due_now {
        DateTime::<Utc>::from_timestamp(signed_event.created_at, 0).unwrap_or_else(Utc::now)
    } else {
        request.publish_time.ok_or_else(|| {
            AppError::bad_request("publish_time_required", "Publish time is required.")
        })?
    };

    validate_trigger_inputs(&request.trigger, Some(publish_time), request.queue_id)?;
    validate_queue_for_owner(state, request.queue_id, &owner_pubkey).await?;
    signed_event
        .validate_signed_for_owner(&owner_pubkey, Some(publish_time))
        .map_err(|err| AppError::bad_request("signed_event_invalid", err.to_string()))?;

    let event_id = signed_event.id.ok_or_else(|| {
        AppError::bad_request("signed_event_invalid", "Signed event must include id.")
    })?;

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "INSERT INTO publish_items
           (owner_pubkey, created_by_pubkey, state, trigger, signed_event_json,
            event_id, publish_time, queue_id)
         VALUES ($1, $2, 'SCHEDULED', $3::publish_trigger, $4, $5, $6, $7)
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .bind(request.trigger)
    .bind(request.signed_event)
    .bind(event_id)
    .bind(publish_time)
    .bind(request.queue_id)
    .fetch_one(&mut *tx)
    .await?;

    let publish_item_id: Uuid = sqlx::Row::try_get(&row, "id")?;
    enqueue_publish_job(&mut tx, publish_item_id, publish_time).await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(row_to_publish_item(row)?)))
}

#[derive(Debug, Deserialize)]
struct ScheduleSignedEventRequest {
    signed_event: serde_json::Value,
    trigger: String,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}
