use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use serde::Serialize;
use shipyard_core::{AccountRelationship, AuthorizedAccount, Pubkey};
use sqlx::Row;

use super::models::{parse_db_pubkey, require_session, ApiState, AppError};

pub(super) fn router() -> Router<ApiState> {
    Router::new().route("/accounts", get(accounts))
}

async fn accounts(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<AccountsResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let mut accounts = vec![AuthorizedAccount {
        owner_pubkey: session.user_pubkey.clone(),
        relationship: AccountRelationship::Owner,
        can_propose: true,
        can_sign: true,
    }];

    let rows = sqlx::query(
        "SELECT owner_pubkey
         FROM account_delegates
         WHERE delegate_pubkey = $1 AND status = 'active'
         ORDER BY created_at DESC",
    )
    .bind(session.user_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    for row in rows {
        accounts.push(AuthorizedAccount {
            owner_pubkey: parse_db_pubkey(row.try_get("owner_pubkey")?)?,
            relationship: AccountRelationship::Delegate,
            can_propose: true,
            can_sign: false,
        });
    }

    Ok(Json(AccountsResponse {
        user_pubkey: session.user_pubkey,
        accounts,
    }))
}

#[derive(Debug, Serialize)]
struct AccountsResponse {
    user_pubkey: Pubkey,
    accounts: Vec<AuthorizedAccount>,
}
