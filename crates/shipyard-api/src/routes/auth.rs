use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::{AuthProof, Pubkey};

use super::models::{extract_client_ip, require_session, ApiState, AppError};

pub(super) fn router() -> Router<ApiState> {
    Router::new()
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/session", get(auth_session))
}

async fn auth_login(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    // --- Rate limiting (per IP and per pubkey) ----------------------------
    let client_ip = extract_client_ip(&headers).unwrap_or_else(|| "unknown".to_string());

    if state.login_limiters.per_ip.check_key(&client_ip).is_err() {
        return Err(AppError::too_many_requests(
            "rate_limit_exceeded",
            "Too many login attempts from this IP. Please try again later.",
        ));
    }

    // Rate-limit by the pubkey in the event before signature verification,
    // so attackers cannot bypass per-pubkey limiting with malformed events.
    let event_pubkey = request.event.pubkey.clone();
    if state
        .login_limiters
        .per_pubkey
        .check_key(&event_pubkey)
        .is_err()
    {
        return Err(AppError::too_many_requests(
            "rate_limit_exceeded",
            "Too many login attempts for this key. Please try again later.",
        ));
    }

    // --- Auth proof verification -------------------------------------------
    let proof = AuthProof {
        event: request.event,
        expected_domain: state.auth_domain.clone(),
        expected_method: "POST".to_string(),
        expected_url: state.auth_url.clone(),
    };
    let user_pubkey = proof
        .verify(Utc::now())
        .map_err(|err| AppError::bad_request("auth_proof_invalid", err.to_string()))?;

    // --- Persist session ---------------------------------------------------
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO users (pubkey, last_seen_at)
         VALUES ($1, now())
         ON CONFLICT (pubkey) DO UPDATE SET last_seen_at = excluded.last_seen_at",
    )
    .bind(user_pubkey.as_str())
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO accounts (pubkey)
         VALUES ($1)
         ON CONFLICT (pubkey) DO UPDATE SET updated_at = now()",
    )
    .bind(user_pubkey.as_str())
    .execute(&mut *tx)
    .await?;

    let expires_at = Utc::now() + Duration::days(30);

    // Store the client IP at session creation (created_ip column already exists).
    let ip_for_db = if client_ip == "unknown" {
        None
    } else {
        Some(client_ip)
    };

    let session_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO sessions (user_pubkey, expires_at, created_ip)
         VALUES ($1, $2, $3::INET)
         RETURNING id",
    )
    .bind(user_pubkey.as_str())
    .bind(expires_at)
    .bind(ip_for_db.as_deref())
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(LoginResponse {
        session_token: session_id.to_string(),
        user_pubkey,
        expires_at,
    }))
}

async fn auth_logout(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1")
        .bind(session.session_id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn auth_session(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<SessionResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    Ok(Json(SessionResponse {
        session_id: session.session_id,
        user_pubkey: session.user_pubkey,
        expires_at: session.expires_at,
    }))
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    event: shipyard_core::AuthEvent,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    session_token: String,
    user_pubkey: Pubkey,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct SessionResponse {
    session_id: uuid::Uuid,
    user_pubkey: Pubkey,
    expires_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn session_response_includes_session_id_user_pubkey_and_expiry() {
        let user_pubkey =
            Pubkey::parse("1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap();
        let expires_at = Utc.with_ymd_and_hms(2026, 4, 17, 12, 0, 0).unwrap();
        let response = SessionResponse {
            session_id: Uuid::nil(),
            user_pubkey: user_pubkey.clone(),
            expires_at,
        };

        assert_eq!(
            serde_json::to_value(response).unwrap(),
            json!({
                "session_id": Uuid::nil(),
                "user_pubkey": user_pubkey,
                "expires_at": expires_at,
            })
        );
    }
}
