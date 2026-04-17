use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use shipyard_core::{ApiErrorBody, Pubkey, Queue};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiState {
    pub(crate) pool: PgPool,
    pub(crate) auth_domain: String,
    pub(crate) auth_url: String,
}

impl ApiState {
    pub async fn from_env() -> anyhow::Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://shipyard:shipyard@localhost:5432/shipyard".into());
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&database_url)
            .await?;

        Ok(Self {
            pool,
            auth_domain: std::env::var("SHIPYARD_AUTH_DOMAIN")
                .unwrap_or_else(|_| "localhost".into()),
            auth_url: std::env::var("SHIPYARD_AUTH_URL")
                .unwrap_or_else(|_| "http://localhost:8080/v1/auth/login".into()),
        })
    }
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
        "SELECT user_pubkey, expires_at
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

    let user_pubkey = parse_db_pubkey(row.try_get("user_pubkey")?)?;
    sqlx::query("UPDATE users SET last_seen_at = now() WHERE pubkey = $1")
        .bind(user_pubkey.as_str())
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
           WHERE owner_pubkey = $1 AND delegate_pubkey = $2 AND status = 'active'
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

#[derive(Debug)]
pub(crate) struct AuthenticatedSession {
    pub(crate) session_id: Uuid,
    pub(crate) user_pubkey: Pubkey,
    pub(crate) expires_at: DateTime<Utc>,
}

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
