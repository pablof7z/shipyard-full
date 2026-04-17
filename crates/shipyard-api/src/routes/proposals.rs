mod helpers;
mod signing;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use shipyard_core::Pubkey;
use sqlx::Row;
use uuid::Uuid;

use super::models::{
    owner_from_headers_or_self, require_account_access, require_owner, require_session, ApiState,
    AppError,
};
use super::publish_items::{
    fetch_publish_item, parse_event_json, row_to_publish_item, validate_queue_for_owner,
    validate_trigger_inputs, PublishItemResponse,
};
use helpers::{insert_revision, require_proposal_mutation_access};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/proposals", get(list_proposals).post(create_proposal))
        .route("/proposals/batch-sign", post(signing::batch_sign_proposals))
        .route(
            "/proposals/:publish_item_id",
            patch(edit_proposal).delete(cancel_proposal),
        )
        .route("/proposals/:publish_item_id/reject", post(reject_proposal))
        .route(
            "/proposals/:publish_item_id/sign",
            post(signing::sign_proposal),
        )
}

async fn list_proposals(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PublishItemResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = if session.user_pubkey == owner_pubkey {
        sqlx::query(super::publish_items::PUBLISH_ITEM_SELECT_OWNER_STATE)
            .bind(owner_pubkey.as_str())
            .bind("PROPOSED")
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(super::publish_items::PUBLISH_ITEM_SELECT_CREATOR_STATE)
            .bind(owner_pubkey.as_str())
            .bind(session.user_pubkey.as_str())
            .bind("PROPOSED")
            .fetch_all(&state.pool)
            .await?
    };

    rows.into_iter()
        .map(row_to_publish_item)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

async fn create_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CreateProposalRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = Pubkey::parse(request.owner_pubkey)
        .map_err(|err| AppError::bad_request("owner_pubkey_invalid", err.to_string()))?;
    require_account_access(&state, &session, &owner_pubkey).await?;
    validate_trigger_inputs(&request.trigger, request.publish_time, request.queue_id)?;
    validate_queue_for_owner(&state, request.queue_id, &owner_pubkey).await?;

    let event = parse_event_json(&request.unsigned_event)?;
    if event.pubkey != owner_pubkey {
        return Err(AppError::bad_request(
            "event_owner_mismatch",
            "Unsigned event pubkey must match owner pubkey.",
        ));
    }
    if event.sig.as_deref().is_some_and(|sig| !sig.is_empty()) {
        return Err(AppError::bad_request(
            "proposal_must_be_unsigned",
            "Proposals must be unsigned until the owner signs.",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "INSERT INTO publish_items
           (owner_pubkey, created_by_pubkey, state, trigger, unsigned_event_json, publish_time, queue_id)
         VALUES ($1, $2, 'PROPOSED', $3::publish_trigger, $4, $5, $6)
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .bind(request.trigger)
    .bind(request.unsigned_event.clone())
    .bind(request.publish_time)
    .bind(request.queue_id)
    .fetch_one(&mut *tx)
    .await?;

    let publish_item_id: Uuid = row.try_get("id")?;
    insert_revision(
        &mut tx,
        publish_item_id,
        &session.user_pubkey,
        &request.unsigned_event,
        Some("created"),
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(row_to_publish_item(row)?)))
}

async fn edit_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
    Json(request): Json<EditProposalRequest>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_proposal_mutation_access(&state, &session, &item, true).await?;

    if let Some(unsigned_event) = &request.unsigned_event {
        let event = parse_event_json(unsigned_event)?;
        if event.pubkey != item.owner_pubkey {
            return Err(AppError::bad_request(
                "event_owner_mismatch",
                "Unsigned event pubkey must match owner pubkey.",
            ));
        }
    }
    if let Some(trigger) = request.trigger.as_deref() {
        validate_trigger_inputs(trigger, request.publish_time, request.queue_id)?;
    }
    validate_queue_for_owner(&state, request.queue_id, &item.owner_pubkey).await?;

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET unsigned_event_json = COALESCE($2, unsigned_event_json),
             trigger = COALESCE($3::publish_trigger, trigger),
             publish_time = COALESCE($4, publish_time),
             queue_id = COALESCE($5, queue_id),
             updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(request.unsigned_event.clone())
    .bind(request.trigger)
    .bind(request.publish_time)
    .bind(request.queue_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Err(AppError::bad_request(
            "proposal_not_editable",
            "Only proposed items can be edited.",
        ));
    };

    if let Some(unsigned_event) = request.unsigned_event {
        insert_revision(
            &mut tx,
            publish_item_id,
            &session.user_pubkey,
            &unsigned_event,
            Some("edited"),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(Json(row_to_publish_item(row)?))
}

async fn cancel_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_proposal_mutation_access(&state, &session, &item, false).await?;

    sqlx::query(
        "UPDATE publish_items
         SET state = 'CANCELLED', updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'",
    )
    .bind(publish_item_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn reject_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
    Json(request): Json<RejectProposalRequest>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;

    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'REJECTED',
             failure_message = $2,
             updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(request.reason)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::bad_request(
            "proposal_not_rejectable",
            "Only proposed items can be rejected.",
        ));
    };

    Ok(Json(row_to_publish_item(row)?))
}

#[derive(Debug, Deserialize)]
struct CreateProposalRequest {
    owner_pubkey: String,
    unsigned_event: serde_json::Value,
    trigger: String,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct EditProposalRequest {
    unsigned_event: Option<serde_json::Value>,
    trigger: Option<String>,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RejectProposalRequest {
    reason: Option<String>,
}
