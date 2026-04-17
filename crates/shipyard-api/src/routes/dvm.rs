use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use serde::Serialize;
use shipyard_core::Pubkey;
use sqlx::Row;
use uuid::Uuid;

use super::models::{
    owner_from_headers_or_self, parse_db_pubkey, require_account_access, require_session, ApiState,
    AppError,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new().route("/dvm/requests", get(list_dvm_requests))
}

async fn list_dvm_requests(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DvmRequestResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(
        "SELECT id, request_event_id, request_pubkey, encrypted, encrypted_tags,
                decrypted_tags, relays, raw_request_event, status::text AS status,
                error, feedback_event_id, feedback_content, feedback_pubkey,
                created_at, updated_at
         FROM dvm_requests
         WHERE request_pubkey = $1
         ORDER BY created_at DESC
         LIMIT 100",
    )
    .bind(owner_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    rows.into_iter()
        .map(row_to_dvm_request)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

fn row_to_dvm_request(row: sqlx::postgres::PgRow) -> Result<DvmRequestResponse, AppError> {
    Ok(DvmRequestResponse {
        id: row.try_get("id")?,
        request_event_id: row.try_get("request_event_id")?,
        request_pubkey: parse_db_pubkey(row.try_get("request_pubkey")?)?,
        encrypted: row.try_get("encrypted")?,
        encrypted_tags: row.try_get("encrypted_tags")?,
        decrypted_tags: row.try_get("decrypted_tags")?,
        relays: row.try_get("relays")?,
        raw_request_event: row.try_get("raw_request_event")?,
        status: row.try_get("status")?,
        error: row.try_get("error")?,
        feedback_event_id: row.try_get("feedback_event_id")?,
        feedback_content: row.try_get("feedback_content")?,
        feedback_pubkey: row.try_get("feedback_pubkey")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

#[derive(Debug, Serialize)]
struct DvmRequestResponse {
    id: Uuid,
    request_event_id: String,
    request_pubkey: Pubkey,
    encrypted: bool,
    encrypted_tags: Option<serde_json::Value>,
    decrypted_tags: Option<serde_json::Value>,
    relays: Vec<String>,
    raw_request_event: serde_json::Value,
    status: String,
    error: Option<String>,
    feedback_event_id: Option<String>,
    feedback_content: Option<String>,
    feedback_pubkey: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
