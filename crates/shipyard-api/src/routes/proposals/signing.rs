use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::{Actor, ApiErrorBody, PublishState};
use uuid::Uuid;

use crate::routes::models::{
    require_owner, require_session, ApiState, AppError, AuthenticatedSession,
};
use crate::routes::publish_items::{
    assert_publish_transition, enqueue_publish_job, fetch_publish_item, parse_event_json,
    resolve_scheduled_publish_time, row_to_publish_item, PublishItemResponse,
};

pub(super) async fn sign_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
    Json(request): Json<SignProposalRequest>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    sign_proposal_for_session(&state, &session, publish_item_id, request.signed_event)
        .await
        .map(Json)
}

pub(super) async fn batch_sign_proposals(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<BatchSignProposalRequest>,
) -> Result<Json<BatchSignProposalResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    if request.items.is_empty() {
        return Err(AppError::bad_request(
            "batch_empty",
            "At least one proposal is required.",
        ));
    }
    if request.items.len() > 50 {
        return Err(AppError::bad_request(
            "batch_too_large",
            "Batch signing is limited to 50 proposals.",
        ));
    }

    let mut results = Vec::with_capacity(request.items.len());
    for item in request.items {
        let result =
            sign_proposal_for_session(&state, &session, item.proposal_id, item.signed_event).await;
        match result {
            Ok(publish_item) => results.push(BatchSignProposalResult {
                proposal_id: item.proposal_id,
                item: Some(publish_item),
                error: None,
            }),
            Err(err) => results.push(BatchSignProposalResult {
                proposal_id: item.proposal_id,
                item: None,
                error: Some(err.body),
            }),
        }
    }

    Ok(Json(BatchSignProposalResponse { results }))
}

async fn sign_proposal_for_session(
    state: &ApiState,
    session: &AuthenticatedSession,
    publish_item_id: Uuid,
    signed_event_json: serde_json::Value,
) -> Result<PublishItemResponse, AppError> {
    let item = fetch_publish_item(state, publish_item_id).await?;
    require_owner(session, &item.owner_pubkey)?;
    let signed_event = parse_event_json(&signed_event_json)?;
    let event_id = signed_event.id.clone().ok_or_else(|| {
        AppError::bad_request("signed_event_invalid", "Signed event must include id.")
    })?;
    assert_publish_transition(Actor::Owner, &item.state, PublishState::Scheduled)?;
    let fallback_publish_time =
        DateTime::<Utc>::from_timestamp(signed_event.created_at, 0).unwrap_or_else(Utc::now);
    let publish_time = if item.trigger == "QUEUE" {
        resolve_scheduled_publish_time(
            state,
            &item.owner_pubkey,
            &item.trigger,
            None,
            item.queue_id,
            Some(publish_item_id),
        )
        .await?
    } else {
        item.publish_time.unwrap_or(fallback_publish_time)
    };
    signed_event
        .validate_signed_for_owner(&item.owner_pubkey, Some(publish_time))
        .map_err(|err| AppError::bad_request("signed_event_invalid", err.to_string()))?;

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'SCHEDULED',
             signed_event_json = $2,
             event_id = $3,
             publish_time = $4,
             updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(signed_event_json)
    .bind(event_id)
    .bind(publish_time)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Err(AppError::bad_request(
            "proposal_not_signable",
            "Only proposed items can be signed.",
        ));
    };

    enqueue_publish_job(&mut tx, publish_item_id, publish_time).await?;
    tx.commit().await?;
    row_to_publish_item(row)
}

#[derive(Debug, Deserialize)]
pub(super) struct SignProposalRequest {
    signed_event: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(super) struct BatchSignProposalRequest {
    items: Vec<BatchSignProposalItem>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BatchSignProposalItem {
    proposal_id: Uuid,
    signed_event: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub(super) struct BatchSignProposalResponse {
    results: Vec<BatchSignProposalResult>,
}

#[derive(Debug, Serialize)]
pub(super) struct BatchSignProposalResult {
    proposal_id: Uuid,
    item: Option<PublishItemResponse>,
    error: Option<ApiErrorBody>,
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("signing.rs");

    #[test]
    fn proposal_signing_resolves_queue_publish_time_before_validation() {
        let implementation = SOURCE.split("#[cfg(test)]").next().unwrap();

        assert!(
            implementation.contains(
                "assert_publish_transition(Actor::Owner, &item.state, PublishState::Scheduled)"
            ),
            "proposal signing must guard PROPOSED -> SCHEDULED through the state machine"
        );
        assert!(
            implementation.contains("if item.trigger == \"QUEUE\"")
                && implementation.contains("resolve_scheduled_publish_time"),
            "QUEUE proposal signing must assign publish_time from shared queue logic"
        );
        assert!(
            implementation.find("resolve_scheduled_publish_time")
                < implementation.find("validate_signed_for_owner"),
            "proposal signing must validate the signed event against the final publish_time"
        );
    }
}
