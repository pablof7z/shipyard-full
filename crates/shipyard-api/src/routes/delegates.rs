use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::Pubkey;
use sqlx::Row;

use super::models::{
    parse_db_pubkey, require_account_access, require_owner, require_session, ApiState, AppError,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route(
            "/accounts/:owner_pubkey/delegates",
            get(list_delegates).post(invite_delegate),
        )
        .route(
            "/accounts/:owner_pubkey/delegates/:delegate_pubkey",
            delete(revoke_delegate),
        )
}

async fn list_delegates(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(owner_pubkey): Path<String>,
) -> Result<Json<Vec<DelegateResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = Pubkey::parse(owner_pubkey)
        .map_err(|err| AppError::bad_request("pubkey_invalid", err.to_string()))?;
    require_owner(&session, &owner_pubkey)?;

    let rows = sqlx::query(
        "SELECT delegate_pubkey, status::text AS status, created_at, revoked_at
         FROM account_delegates
         WHERE owner_pubkey = $1
         ORDER BY created_at DESC",
    )
    .bind(owner_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    let delegates = rows
        .into_iter()
        .map(|row| {
            Ok(DelegateResponse {
                delegate_pubkey: parse_db_pubkey(row.try_get("delegate_pubkey")?)?,
                status: row.try_get("status")?,
                created_at: row.try_get("created_at")?,
                revoked_at: row.try_get("revoked_at")?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    Ok(Json(delegates))
}

async fn invite_delegate(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(owner_pubkey): Path<String>,
    Json(request): Json<InviteDelegateRequest>,
) -> Result<Json<DelegateResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = Pubkey::parse(owner_pubkey)
        .map_err(|err| AppError::bad_request("pubkey_invalid", err.to_string()))?;
    let delegate_pubkey = Pubkey::parse(request.delegate_pubkey)
        .map_err(|err| AppError::bad_request("delegate_pubkey_invalid", err.to_string()))?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO users (pubkey)
         VALUES ($1)
         ON CONFLICT (pubkey) DO NOTHING",
    )
    .bind(delegate_pubkey.as_str())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO account_delegates
           (owner_pubkey, delegate_pubkey, created_by_pubkey, status, revoked_at)
         VALUES ($1, $2, $3, 'active', NULL)
         ON CONFLICT (owner_pubkey, delegate_pubkey)
         DO UPDATE SET status = 'active', revoked_at = NULL",
    )
    .bind(owner_pubkey.as_str())
    .bind(delegate_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .execute(&mut *tx)
    .await?;

    let row = sqlx::query(
        "SELECT delegate_pubkey, status::text AS status, created_at, revoked_at
         FROM account_delegates
         WHERE owner_pubkey = $1 AND delegate_pubkey = $2",
    )
    .bind(owner_pubkey.as_str())
    .bind(delegate_pubkey.as_str())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(DelegateResponse {
        delegate_pubkey: parse_db_pubkey(row.try_get("delegate_pubkey")?)?,
        status: row.try_get("status")?,
        created_at: row.try_get("created_at")?,
        revoked_at: row.try_get("revoked_at")?,
    }))
}

async fn revoke_delegate(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path((owner_pubkey, delegate_pubkey)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = Pubkey::parse(owner_pubkey)
        .map_err(|err| AppError::bad_request("pubkey_invalid", err.to_string()))?;
    let delegate_pubkey = Pubkey::parse(delegate_pubkey)
        .map_err(|err| AppError::bad_request("delegate_pubkey_invalid", err.to_string()))?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    sqlx::query(
        "UPDATE account_delegates
         SET status = 'revoked', revoked_at = now()
         WHERE owner_pubkey = $1 AND delegate_pubkey = $2",
    )
    .bind(owner_pubkey.as_str())
    .bind(delegate_pubkey.as_str())
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct InviteDelegateRequest {
    delegate_pubkey: String,
}

#[derive(Debug, Serialize)]
struct DelegateResponse {
    delegate_pubkey: Pubkey,
    status: String,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("delegates.rs");

    #[test]
    fn mutating_delegate_routes_allow_owner_or_active_delegate_access() {
        let implementation = SOURCE.split("#[cfg(test)]").next().unwrap();
        let access_checks = implementation
            .matches("require_account_access(&state, &session, &owner_pubkey).await?")
            .count();

        assert_eq!(
            access_checks, 2,
            "invite and revoke delegate routes must both use account access checks"
        );
    }
}
