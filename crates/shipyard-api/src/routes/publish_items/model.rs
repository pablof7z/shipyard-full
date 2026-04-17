use chrono::{DateTime, Utc};
use serde::Serialize;
use shipyard_core::{NostrEvent, Pubkey};
use sqlx::{Postgres, Row, Transaction};
use uuid::Uuid;

use crate::routes::models::{parse_db_pubkey, queue_owner, ApiState, AppError};

#[derive(Debug, Serialize)]
pub(in crate::routes) struct PublishItemResponse {
    pub(in crate::routes) id: Uuid,
    pub(in crate::routes) owner_pubkey: Pubkey,
    pub(in crate::routes) created_by_pubkey: Pubkey,
    pub(in crate::routes) state: String,
    pub(in crate::routes) trigger: String,
    pub(in crate::routes) unsigned_event_json: Option<serde_json::Value>,
    pub(in crate::routes) signed_event_json: Option<serde_json::Value>,
    pub(in crate::routes) event_id: Option<String>,
    pub(in crate::routes) publish_time: Option<DateTime<Utc>>,
    pub(in crate::routes) queue_id: Option<Uuid>,
    pub(in crate::routes) published_at: Option<DateTime<Utc>>,
    pub(in crate::routes) published_to: Vec<String>,
    pub(in crate::routes) failure_code: Option<String>,
    pub(in crate::routes) failure_message: Option<String>,
    pub(in crate::routes) created_at: DateTime<Utc>,
    pub(in crate::routes) updated_at: DateTime<Utc>,
}

pub(in crate::routes) const PUBLISH_ITEM_SELECT_BY_ID: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE id = $1";

pub(in crate::routes) const PUBLISH_ITEM_SELECT_OWNER: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1
     ORDER BY created_at DESC";

pub(in crate::routes) const PUBLISH_ITEM_SELECT_OWNER_STATE: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1 AND state = $2::publish_state
     ORDER BY created_at DESC";

pub(in crate::routes) const PUBLISH_ITEM_SELECT_CREATOR_STATE: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1 AND created_by_pubkey = $2 AND state = $3::publish_state
     ORDER BY created_at DESC";

pub(in crate::routes) async fn fetch_publish_item(
    state: &ApiState,
    publish_item_id: Uuid,
) -> Result<PublishItemResponse, AppError> {
    let row = sqlx::query(PUBLISH_ITEM_SELECT_BY_ID)
        .bind(publish_item_id)
        .fetch_optional(&state.pool)
        .await?;

    let Some(row) = row else {
        return Err(AppError::not_found(
            "publish_item_not_found",
            "Publish item not found.",
        ));
    };

    row_to_publish_item(row)
}

pub(in crate::routes) fn parse_event_json(
    value: &serde_json::Value,
) -> Result<NostrEvent, AppError> {
    serde_json::from_value(value.clone()).map_err(|_| {
        AppError::bad_request(
            "event_json_invalid",
            "This event can't be published — missing required fields.",
        )
    })
}

pub(in crate::routes) async fn validate_queue_for_owner(
    state: &ApiState,
    queue_id: Option<Uuid>,
    owner_pubkey: &Pubkey,
) -> Result<(), AppError> {
    let Some(queue_id) = queue_id else {
        return Ok(());
    };

    let queue_owner = queue_owner(state, queue_id).await?;
    if &queue_owner == owner_pubkey {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "queue_owner_mismatch",
            "Queue does not belong to this account.",
        ))
    }
}

pub(in crate::routes) fn validate_trigger_inputs(
    trigger: &str,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
) -> Result<(), AppError> {
    match trigger {
        "TIME" => {
            if publish_time.is_none() {
                return Err(AppError::bad_request(
                    "publish_time_required",
                    "Publish time is required for timed scheduling.",
                ));
            }
            if queue_id.is_some() {
                return Err(AppError::bad_request(
                    "queue_not_allowed",
                    "Timed scheduling cannot include a queue id.",
                ));
            }
        }
        "QUEUE" => {
            if queue_id.is_none() {
                return Err(AppError::bad_request(
                    "queue_required",
                    "Queue id is required for queued scheduling.",
                ));
            }
        }
        "SEND_NOW" => {
            if queue_id.is_some() {
                return Err(AppError::bad_request(
                    "queue_not_allowed",
                    "Publish now cannot include a queue id.",
                ));
            }
        }
        "DVM" => {}
        _ => {
            return Err(AppError::bad_request(
                "trigger_invalid",
                "Trigger must be TIME, QUEUE, SEND_NOW, or DVM.",
            ));
        }
    }
    Ok(())
}

pub(in crate::routes) async fn enqueue_publish_job(
    tx: &mut Transaction<'_, Postgres>,
    publish_item_id: Uuid,
    publish_time: DateTime<Utc>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO jobs (kind, run_at, payload)
         VALUES ('publish_event', $1, jsonb_build_object('publish_item_id', $2::text))",
    )
    .bind(publish_time)
    .bind(publish_item_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub(in crate::routes) fn row_to_publish_item(
    row: sqlx::postgres::PgRow,
) -> Result<PublishItemResponse, AppError> {
    Ok(PublishItemResponse {
        id: row.try_get("id")?,
        owner_pubkey: parse_db_pubkey(row.try_get("owner_pubkey")?)?,
        created_by_pubkey: parse_db_pubkey(row.try_get("created_by_pubkey")?)?,
        state: row.try_get("state")?,
        trigger: row.try_get("trigger")?,
        unsigned_event_json: row.try_get("unsigned_event_json")?,
        signed_event_json: row.try_get("signed_event_json")?,
        event_id: row.try_get("event_id")?,
        publish_time: row.try_get("publish_time")?,
        queue_id: row.try_get("queue_id")?,
        published_at: row.try_get("published_at")?,
        published_to: row.try_get("published_to")?,
        failure_code: row.try_get("failure_code")?,
        failure_message: row.try_get("failure_message")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}
