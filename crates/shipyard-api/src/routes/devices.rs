use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::Pubkey;
use sqlx::Row;
use uuid::Uuid;

use super::models::{parse_db_pubkey, require_account_access, require_session, ApiState, AppError};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/devices", get(list_devices).post(register_device))
        .route(
            "/devices/:device_id",
            patch(update_device).delete(delete_device),
        )
}

async fn list_devices(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DeviceTokenResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;

    let rows = sqlx::query(
        "SELECT id, user_pubkey, owner_pubkey, platform, token, enabled, created_at, updated_at
         FROM device_tokens
         WHERE user_pubkey = $1
         ORDER BY updated_at DESC",
    )
    .bind(session.user_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    rows.into_iter()
        .map(row_to_device_token)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

async fn register_device(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDeviceRequest>,
) -> Result<(StatusCode, Json<DeviceTokenResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    validate_device_platform(&request.platform)?;
    if request.token.trim().is_empty() {
        return Err(AppError::bad_request(
            "device_token_required",
            "Device token is required.",
        ));
    }
    let owner_pubkey = parse_optional_owner_pubkey(request.owner_pubkey)?;
    if let Some(owner_pubkey) = &owner_pubkey {
        require_account_access(&state, &session, owner_pubkey).await?;
    }

    let row = sqlx::query(
        "INSERT INTO device_tokens (user_pubkey, owner_pubkey, platform, token, enabled)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (platform, token)
         DO UPDATE SET user_pubkey = excluded.user_pubkey,
                       owner_pubkey = excluded.owner_pubkey,
                       enabled = excluded.enabled,
                       updated_at = now()
         RETURNING id, user_pubkey, owner_pubkey, platform, token, enabled, created_at, updated_at",
    )
    .bind(session.user_pubkey.as_str())
    .bind(owner_pubkey.as_ref().map(Pubkey::as_str))
    .bind(request.platform)
    .bind(request.token.trim())
    .bind(request.enabled.unwrap_or(true))
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row_to_device_token(row)?)))
}

async fn update_device(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
    Json(request): Json<UpdateDeviceRequest>,
) -> Result<Json<DeviceTokenResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = parse_optional_owner_pubkey(request.owner_pubkey)?;
    if let Some(owner_pubkey) = &owner_pubkey {
        require_account_access(&state, &session, owner_pubkey).await?;
    }

    let row = sqlx::query(
        "UPDATE device_tokens
         SET enabled = COALESCE($3, enabled),
             owner_pubkey = COALESCE($4, owner_pubkey),
             updated_at = now()
         WHERE id = $1 AND user_pubkey = $2
         RETURNING id, user_pubkey, owner_pubkey, platform, token, enabled, created_at, updated_at",
    )
    .bind(device_id)
    .bind(session.user_pubkey.as_str())
    .bind(request.enabled)
    .bind(owner_pubkey.as_ref().map(Pubkey::as_str))
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("device_not_found", "Device not found."));
    };

    Ok(Json(row_to_device_token(row)?))
}

async fn delete_device(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let result = sqlx::query("DELETE FROM device_tokens WHERE id = $1 AND user_pubkey = $2")
        .bind(device_id)
        .bind(session.user_pubkey.as_str())
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("device_not_found", "Device not found."));
    }

    Ok(StatusCode::NO_CONTENT)
}

fn row_to_device_token(row: sqlx::postgres::PgRow) -> Result<DeviceTokenResponse, AppError> {
    let owner_pubkey: Option<String> = row.try_get("owner_pubkey")?;
    Ok(DeviceTokenResponse {
        id: row.try_get("id")?,
        user_pubkey: parse_db_pubkey(row.try_get("user_pubkey")?)?,
        owner_pubkey: owner_pubkey.map(parse_db_pubkey).transpose()?,
        platform: row.try_get("platform")?,
        token: row.try_get("token")?,
        enabled: row.try_get("enabled")?,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn validate_device_platform(platform: &str) -> Result<(), AppError> {
    if matches!(platform, "ios" | "android") {
        Ok(())
    } else {
        Err(AppError::bad_request(
            "device_platform_invalid",
            "Device platform must be ios or android.",
        ))
    }
}

fn parse_optional_owner_pubkey(value: Option<String>) -> Result<Option<Pubkey>, AppError> {
    value
        .map(Pubkey::parse)
        .transpose()
        .map_err(|err| AppError::bad_request("owner_pubkey_invalid", err.to_string()))
}

#[derive(Debug, Deserialize)]
struct RegisterDeviceRequest {
    platform: String,
    token: String,
    owner_pubkey: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct UpdateDeviceRequest {
    owner_pubkey: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct DeviceTokenResponse {
    id: Uuid,
    user_pubkey: Pubkey,
    owner_pubkey: Option<Pubkey>,
    platform: String,
    token: String,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
