use axum::{extract::State, http::HeaderMap, routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use shipyard_core::Pubkey;

use super::models::{
    owner_from_headers_or_self, require_account_access, require_owner, require_session, ApiState,
    AppError,
};

pub(super) fn router() -> Router<ApiState> {
    Router::new().route("/relays", get(get_relays).put(update_relays))
}

async fn get_relays(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<RelaySettingsResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let relay_urls: Vec<String> =
        sqlx::query_scalar("SELECT relay_urls FROM relay_settings WHERE owner_pubkey = $1")
            .bind(owner_pubkey.as_str())
            .fetch_optional(&state.pool)
            .await?
            .unwrap_or_default();

    Ok(Json(RelaySettingsResponse {
        owner_pubkey,
        relay_urls,
    }))
}

async fn update_relays(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<UpdateRelaySettingsRequest>,
) -> Result<Json<RelaySettingsResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_owner(&session, &owner_pubkey)?;
    validate_relay_urls(&request.relay_urls)?;

    let relay_urls: Vec<String> = sqlx::query_scalar(
        "INSERT INTO relay_settings (owner_pubkey, relay_urls)
         VALUES ($1, $2)
         ON CONFLICT (owner_pubkey)
         DO UPDATE SET relay_urls = excluded.relay_urls, updated_at = now()
         RETURNING relay_urls",
    )
    .bind(owner_pubkey.as_str())
    .bind(&request.relay_urls)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(RelaySettingsResponse {
        owner_pubkey,
        relay_urls,
    }))
}

fn validate_relay_urls(relay_urls: &[String]) -> Result<(), AppError> {
    if relay_urls.is_empty() {
        return Err(AppError::bad_request(
            "relay_list_empty",
            "Add at least one relay before publishing.",
        ));
    }

    for relay_url in relay_urls {
        let allowed = relay_url.starts_with("wss://")
            || relay_url.starts_with("ws://localhost")
            || relay_url.starts_with("ws://127.0.0.1");
        if !allowed {
            return Err(AppError::bad_request(
                "relay_url_invalid",
                "Relay URLs must use wss://, except local development ws:// URLs.",
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
struct UpdateRelaySettingsRequest {
    relay_urls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RelaySettingsResponse {
    owner_pubkey: Pubkey,
    relay_urls: Vec<String>,
}
