use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use shipyard_core::{ApiErrorBody, Pubkey, Queue};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::num::NonZeroU32;
use std::sync::Arc;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Rate limiters
// ---------------------------------------------------------------------------

/// In-memory, keyed rate limiters for the login endpoint.
///
/// * `per_ip`     – max 10 attempts per minute from the same IP address.
/// * `per_pubkey` – max 5 attempts per minute for the same Nostr pubkey.
#[derive(Clone)]
pub(crate) struct LoginRateLimiters {
    pub(crate) per_ip: Arc<DefaultKeyedRateLimiter<String>>,
    pub(crate) per_pubkey: Arc<DefaultKeyedRateLimiter<String>>,
}

impl LoginRateLimiters {
    pub(crate) fn new() -> Self {
        Self {
            per_ip: Arc::new(RateLimiter::dashmap(Quota::per_minute(
                NonZeroU32::new(10).unwrap(),
            ))),
            per_pubkey: Arc::new(RateLimiter::dashmap(Quota::per_minute(
                NonZeroU32::new(5).unwrap(),
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// API state
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ApiState {
    pub(crate) pool: PgPool,
    pub(crate) auth_domain: String,
    pub(crate) auth_url: String,
    /// When `true`, session validation rejects requests whose IP address does
    /// not match the IP recorded at session creation.
    pub(crate) strict_ip_binding: bool,
    pub(crate) login_limiters: LoginRateLimiters,
}

impl ApiState {
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://shipyard:shipyard@localhost:5432/shipyard".into());
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        let strict_ip_binding = std::env::var("SHIPYARD_SESSION_STRICT_IP_BINDING")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");

        Ok(Self {
            pool,
            auth_domain: std::env::var("SHIPYARD_AUTH_DOMAIN")
                .unwrap_or_else(|_| "localhost".into()),
            auth_url: std::env::var("SHIPYARD_AUTH_URL")
                .unwrap_or_else(|_| "http://localhost:8080/v1/auth/login".into()),
            strict_ip_binding,
            login_limiters: LoginRateLimiters::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Session helpers
// ---------------------------------------------------------------------------

/// Extract the best-effort client IP address from request headers.
///
/// Checks (in order):
/// 1. `X-Forwarded-For` – first entry in the comma-separated list
/// 2. `X-Real-IP`
///
/// Returns `None` when neither header is present or parseable.
pub(crate) fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    if let Some(forwarded_for) = headers.get("x-forwarded-for") {
        if let Ok(value) = forwarded_for.to_str() {
            let first = value.split(',').next().map(|s| s.trim().to_string());
            if first.as_deref().map(|s| !s.is_empty()).unwrap_or(false) {
                return first;
            }
        }
    }
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
}

pub(crate) async fn require_session(
    state: &ApiState,
    headers: &HeaderMap,
) -> Result<AuthenticatedSession, AppError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or_else(|| AppError::unauthorized("missing_session", "Login required."))?;
    let session_id = Uuid::parse_str(token)
        .map_err(|_| AppError::unauthorized("invalid_session", "Session token is invalid."))?;

    let row = sqlx::query(
        "SELECT user_pubkey, expires_at, created_ip::TEXT AS created_ip
         FROM sessions
         WHERE id = $1 AND revoked_at IS NULL AND expires_at > now()",
    )
    .bind(session_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::unauthorized(
            "session_expired",
            "Session expired.",
        ));
    };

    // Optional strict IP binding: reject if the request comes from a different IP
    if state.strict_ip_binding {
        let created_ip: Option<String> = row.try_get("created_ip")?;
        if let Some(created_ip) = created_ip {
            let current_ip = extract_client_ip(headers);
            if current_ip.as_deref() != Some(created_ip.as_str()) {
                return Err(AppError::unauthorized(
                    "ip_mismatch",
                    "Session is not valid from this IP address.",
                ));
            }
        }
    }

    let user_pubkey = parse_db_pubkey(row.try_get("user_pubkey")?)?;

    // Update last_seen_at and track the last IP this session was used from
    let current_ip = extract_client_ip(headers);
    sqlx::query("UPDATE users SET last_seen_at = now() WHERE pubkey = $1")
        .bind(user_pubkey.as_str())
        .execute(&state.pool)
        .await?;

    sqlx::query("UPDATE sessions SET last_ip = $1::INET WHERE id = $2")
        .bind(current_ip.as_deref())
        .bind(session_id)
        .execute(&state.pool)
        .await?;

    Ok(AuthenticatedSession {
        session_id,
        user_pubkey,
        expires_at: row.try_get("expires_at")?,
    })
}

pub(crate) fn require_owner(
    session: &AuthenticatedSession,
    owner_pubkey: &Pubkey,
) -> Result<(), AppError> {
    if &session.user_pubkey == owner_pubkey {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "owner_required",
            "You do not have permission to manage this account.",
        ))
    }
}

pub(crate) async fn require_account_access(
    state: &ApiState,
    session: &AuthenticatedSession,
    owner_pubkey: &Pubkey,
) -> Result<(), AppError> {
    if &session.user_pubkey == owner_pubkey {
        return Ok(());
    }

    let has_access: bool = sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM account_delegates
           WHERE owner_pubkey = $1 AND delegate_pubkey = $2 AND status = 'active' AND revoked_at IS NULL
         )",
    )
    .bind(owner_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .fetch_one(&state.pool)
    .await?;

    if has_access {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "delegate_not_authorized",
            "You're not authorized to propose for this account.",
        ))
    }
}

pub(crate) fn owner_from_headers_or_self(
    headers: &HeaderMap,
    session: &AuthenticatedSession,
) -> Result<Pubkey, AppError> {
    headers
        .get("x-shipyard-owner-pubkey")
        .and_then(|value| value.to_str().ok())
        .map(Pubkey::parse)
        .transpose()
        .map_err(|err| AppError::bad_request("owner_pubkey_invalid", err.to_string()))?
        .map_or_else(|| Ok(session.user_pubkey.clone()), Ok)
}

pub(crate) async fn queue_owner(state: &ApiState, queue_id: Uuid) -> Result<Pubkey, AppError> {
    Ok(fetch_queue(state, queue_id).await?.owner_pubkey)
}

pub(crate) async fn fetch_queue(state: &ApiState, queue_id: Uuid) -> Result<Queue, AppError> {
    let row = sqlx::query(
        "SELECT id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at
         FROM queues
         WHERE id = $1",
    )
    .bind(queue_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::not_found("queue_not_found", "Queue not found."));
    };

    row_to_queue(row)
}

pub(crate) fn row_to_queue(row: sqlx::postgres::PgRow) -> Result<Queue, AppError> {
    Ok(Queue {
        id: row.try_get("id")?,
        owner_pubkey: parse_db_pubkey(row.try_get("owner_pubkey")?)?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        cadence_seconds: row.try_get("cadence_seconds")?,
        start_at: row.try_get("start_at")?,
        archived_at: row.try_get("archived_at")?,
    })
}

pub(crate) fn parse_db_pubkey(value: String) -> Result<Pubkey, AppError> {
    Pubkey::parse(value).map_err(|err| AppError::internal("stored_pubkey_invalid", err.to_string()))
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct AuthenticatedSession {
    pub(crate) session_id: Uuid,
    pub(crate) user_pubkey: Pubkey,
    pub(crate) expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub(crate) struct AppError {
    pub(crate) status: StatusCode,
    pub(crate) body: ApiErrorBody,
}

impl AppError {
    pub(crate) fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    pub(crate) fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    pub(crate) fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    pub(crate) fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub(crate) fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, code, message)
    }

    pub(crate) fn too_many_requests(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, code, message)
    }

    fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            body: ApiErrorBody {
                code: code.into(),
                message: message.into(),
                details: None,
                request_id: None,
            },
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        tracing::error!(%error, "database error");
        AppError::internal("database_error", "The database query failed.")
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const SOURCE: &str = include_str!("models.rs");

    #[test]
    fn account_access_requires_delegate_row_to_be_active_and_non_revoked() {
        let implementation = SOURCE.split("#[cfg(test)]").next().unwrap();

        assert!(
            implementation.contains("owner_pubkey = $1 AND delegate_pubkey = $2 AND status = 'active' AND revoked_at IS NULL"),
            "delegate access must require an active, non-revoked owner/delegate row"
        );
    }

    #[test]
    fn extract_client_ip_returns_none_when_no_headers_present() {
        let headers = HeaderMap::new();
        assert_eq!(extract_client_ip(&headers), None);
    }

    #[test]
    fn extract_client_ip_reads_x_forwarded_for_first_entry() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("10.0.0.1, 10.0.0.2"),
        );
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("10.0.0.1"));
    }

    #[test]
    fn extract_client_ip_falls_back_to_x_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.5"));
        assert_eq!(extract_client_ip(&headers).as_deref(), Some("192.168.1.5"));
    }

    #[test]
    fn login_rate_limiter_per_ip_blocks_after_ten_attempts() {
        let limiters = LoginRateLimiters::new();
        let key = "203.0.113.42".to_string();

        for _ in 0..10 {
            assert!(
                limiters.per_ip.check_key(&key).is_ok(),
                "first 10 attempts should succeed"
            );
        }

        assert!(
            limiters.per_ip.check_key(&key).is_err(),
            "11th attempt should be rate-limited"
        );
    }

    #[test]
    fn login_rate_limiter_per_pubkey_blocks_after_five_attempts() {
        let limiters = LoginRateLimiters::new();
        let key = "abcdef1234567890".to_string();

        for _ in 0..5 {
            assert!(
                limiters.per_pubkey.check_key(&key).is_ok(),
                "first 5 attempts should succeed"
            );
        }

        assert!(
            limiters.per_pubkey.check_key(&key).is_err(),
            "6th attempt should be rate-limited"
        );
    }

    #[test]
    fn login_rate_limiter_tracks_different_keys_independently() {
        let limiters = LoginRateLimiters::new();

        // Exhaust one IP
        for _ in 0..10 {
            let _ = limiters.per_ip.check_key(&"1.1.1.1".to_string());
        }
        assert!(limiters.per_ip.check_key(&"1.1.1.1".to_string()).is_err());

        // Different IP should still be fine
        assert!(limiters.per_ip.check_key(&"2.2.2.2".to_string()).is_ok());
    }

    #[test]
    fn strict_ip_binding_env_defaults_to_false() {
        std::env::remove_var("SHIPYARD_SESSION_STRICT_IP_BINDING");
        let enabled = std::env::var("SHIPYARD_SESSION_STRICT_IP_BINDING")
            .unwrap_or_default()
            .eq_ignore_ascii_case("true");
        assert!(!enabled);
    }
}
