use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use shipyard_core::{
    next_queue_slot, AccountRelationship, ApiErrorBody, AuthProof, AuthorizedAccount, NostrEvent,
    Pubkey, Queue,
};
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

#[derive(Clone)]
pub struct ApiState {
    pool: PgPool,
    auth_domain: String,
    auth_url: String,
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

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/auth/login", post(auth_login))
        .route("/auth/logout", post(auth_logout))
        .route("/auth/session", get(auth_session))
        .route("/accounts", get(accounts))
        .route(
            "/accounts/:owner_pubkey/delegates",
            get(list_delegates).post(invite_delegate),
        )
        .route(
            "/accounts/:owner_pubkey/delegates/:delegate_pubkey",
            delete(revoke_delegate),
        )
        .route("/queues", get(list_queues).post(create_queue))
        .route("/queues/:queue_id", patch(update_queue))
        .route("/queues/:queue_id/next-slot", get(next_queue_slot_route))
        .route("/queues/:queue_id/archive", post(archive_queue))
        .route("/proposals", get(list_proposals).post(create_proposal))
        .route("/proposals/batch-sign", post(batch_sign_proposals))
        .route(
            "/proposals/:publish_item_id",
            patch(edit_proposal).delete(cancel_proposal),
        )
        .route("/proposals/:publish_item_id/reject", post(reject_proposal))
        .route("/proposals/:publish_item_id/sign", post(sign_proposal))
        .route("/publish-items", get(list_publish_items))
        .route("/publish-items/schedule", post(schedule_signed_event))
        .route("/publish-items/send-now", post(send_now))
        .route(
            "/publish-items/:publish_item_id/cancel",
            post(cancel_publish_item),
        )
        .route(
            "/publish-items/:publish_item_id/retry",
            post(retry_publish_item),
        )
        .route("/relays", get(get_relays).put(update_relays))
        .route("/dvm/requests", get(list_dvm_requests))
        .route("/devices", get(list_devices).post(register_device))
        .route(
            "/devices/:device_id",
            patch(update_device).delete(delete_device),
        )
        .with_state(state)
}

async fn status(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(serde_json::json!({
        "service": "shipyard-api",
        "status": "ok",
        "database": "ok",
        "interfaces": {
            "auth": "/v1/auth/*",
            "accounts": "/v1/accounts",
            "delegates": "/v1/accounts/{owner_pubkey}/delegates",
            "proposals": "/v1/proposals",
            "publish_items": "/v1/publish-items",
            "queues": "/v1/queues",
            "relays": "/v1/relays",
            "dvm_requests": "/v1/dvm/requests",
            "devices": "/v1/devices"
        }
    })))
}

async fn auth_login(
    State(state): State<ApiState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let proof = AuthProof {
        event: request.event,
        expected_domain: state.auth_domain.clone(),
        expected_method: "POST".to_string(),
        expected_url: state.auth_url.clone(),
    };
    let user_pubkey = proof
        .verify(Utc::now())
        .map_err(|err| AppError::bad_request("auth_proof_invalid", err.to_string()))?;

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
    let session_id: Uuid = sqlx::query_scalar(
        "INSERT INTO sessions (user_pubkey, expires_at)
         VALUES ($1, $2)
         RETURNING id",
    )
    .bind(user_pubkey.as_str())
    .bind(expires_at)
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
        user_pubkey: session.user_pubkey,
        expires_at: session.expires_at,
    }))
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
    require_owner(&session, &owner_pubkey)?;

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
    require_owner(&session, &owner_pubkey)?;

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

async fn list_queues(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Queue>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(
        "SELECT id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at
         FROM queues
         WHERE owner_pubkey = $1
         ORDER BY archived_at NULLS FIRST, created_at DESC",
    )
    .bind(owner_pubkey.as_str())
    .fetch_all(&state.pool)
    .await?;

    let queues = rows
        .into_iter()
        .map(row_to_queue)
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(Json(queues))
}

async fn create_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CreateQueueRequest>,
) -> Result<(StatusCode, Json<Queue>), AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_owner(&session, &owner_pubkey)?;

    if request.name.trim().is_empty() {
        return Err(AppError::bad_request(
            "queue_name_required",
            "Queue name is required.",
        ));
    }
    if request.cadence_seconds <= 0 {
        return Err(AppError::bad_request(
            "queue_cadence_invalid",
            "Queue cadence must be positive.",
        ));
    }

    let row = sqlx::query(
        "INSERT INTO queues (owner_pubkey, name, description, cadence_seconds, start_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(request.name.trim())
    .bind(request.description)
    .bind(request.cadence_seconds)
    .bind(request.start_at)
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row_to_queue(row)?)))
}

async fn update_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
    Json(request): Json<UpdateQueueRequest>,
) -> Result<Json<Queue>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = queue_owner(&state, queue_id).await?;
    require_owner(&session, &owner_pubkey)?;

    if request
        .name
        .as_deref()
        .is_some_and(|name| name.trim().is_empty())
    {
        return Err(AppError::bad_request(
            "queue_name_required",
            "Queue name is required.",
        ));
    }
    if request
        .cadence_seconds
        .is_some_and(|cadence_seconds| cadence_seconds <= 0)
    {
        return Err(AppError::bad_request(
            "queue_cadence_invalid",
            "Queue cadence must be positive.",
        ));
    }
    let name = request.name.map(|name| name.trim().to_string());

    let row = sqlx::query(
        "UPDATE queues
         SET name = COALESCE($2, name),
             description = COALESCE($3, description),
             cadence_seconds = COALESCE($4, cadence_seconds),
             start_at = COALESCE($5, start_at),
             updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(queue_id)
    .bind(name)
    .bind(request.description)
    .bind(request.cadence_seconds)
    .bind(request.start_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row_to_queue(row)?))
}

async fn archive_queue(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
) -> Result<Json<Queue>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = queue_owner(&state, queue_id).await?;
    require_owner(&session, &owner_pubkey)?;

    let row = sqlx::query(
        "UPDATE queues
         SET archived_at = COALESCE(archived_at, now()), updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, name, description, cadence_seconds, start_at, archived_at",
    )
    .bind(queue_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row_to_queue(row)?))
}

async fn next_queue_slot_route(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(queue_id): Path<Uuid>,
) -> Result<Json<QueueNextSlotResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let queue = fetch_queue(&state, queue_id).await?;
    require_account_access(&state, &session, &queue.owner_pubkey).await?;

    let latest_queue_slot: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT max(publish_time)
         FROM publish_items
         WHERE queue_id = $1
           AND publish_time IS NOT NULL
           AND state NOT IN ('REJECTED', 'CANCELLED', 'FAILED')",
    )
    .bind(queue_id)
    .fetch_one(&state.pool)
    .await?;
    let now = Utc::now();
    let next_slot = next_queue_slot(&queue, now, latest_queue_slot)
        .map_err(|err| AppError::bad_request("queue_next_slot_unavailable", err.to_string()))?;

    Ok(Json(QueueNextSlotResponse {
        queue_id,
        owner_pubkey: queue.owner_pubkey,
        next_slot,
        latest_queue_slot,
        now,
    }))
}

async fn list_proposals(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PublishItemResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = if session.user_pubkey == owner_pubkey {
        sqlx::query(PUBLISH_ITEM_SELECT_OWNER_STATE)
            .bind(owner_pubkey.as_str())
            .bind("PROPOSED")
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(PUBLISH_ITEM_SELECT_CREATOR_STATE)
            .bind(owner_pubkey.as_str())
            .bind(session.user_pubkey.as_str())
            .bind("PROPOSED")
            .fetch_all(&state.pool)
            .await?
    };

    rows.into_iter()
        .map(row_to_publish_item)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

async fn create_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CreateProposalRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = Pubkey::parse(request.owner_pubkey)
        .map_err(|err| AppError::bad_request("owner_pubkey_invalid", err.to_string()))?;
    require_account_access(&state, &session, &owner_pubkey).await?;
    validate_trigger_inputs(&request.trigger, request.publish_time, request.queue_id)?;
    validate_queue_for_owner(&state, request.queue_id, &owner_pubkey).await?;

    let event = parse_event_json(&request.unsigned_event)?;
    if event.pubkey != owner_pubkey {
        return Err(AppError::bad_request(
            "event_owner_mismatch",
            "Unsigned event pubkey must match owner pubkey.",
        ));
    }
    if event.sig.as_deref().is_some_and(|sig| !sig.is_empty()) {
        return Err(AppError::bad_request(
            "proposal_must_be_unsigned",
            "Proposals must be unsigned until the owner signs.",
        ));
    }

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "INSERT INTO publish_items
           (owner_pubkey, created_by_pubkey, state, trigger, unsigned_event_json, publish_time, queue_id)
         VALUES ($1, $2, 'PROPOSED', $3::publish_trigger, $4, $5, $6)
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .bind(request.trigger)
    .bind(request.unsigned_event.clone())
    .bind(request.publish_time)
    .bind(request.queue_id)
    .fetch_one(&mut *tx)
    .await?;

    let publish_item_id: Uuid = row.try_get("id")?;
    insert_revision(
        &mut tx,
        publish_item_id,
        &session.user_pubkey,
        &request.unsigned_event,
        Some("created"),
    )
    .await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(row_to_publish_item(row)?)))
}

async fn edit_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
    Json(request): Json<EditProposalRequest>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_proposal_mutation_access(&state, &session, &item, true).await?;

    if let Some(unsigned_event) = &request.unsigned_event {
        let event = parse_event_json(unsigned_event)?;
        if event.pubkey != item.owner_pubkey {
            return Err(AppError::bad_request(
                "event_owner_mismatch",
                "Unsigned event pubkey must match owner pubkey.",
            ));
        }
    }
    if let Some(trigger) = request.trigger.as_deref() {
        validate_trigger_inputs(trigger, request.publish_time, request.queue_id)?;
    }
    validate_queue_for_owner(&state, request.queue_id, &item.owner_pubkey).await?;

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET unsigned_event_json = COALESCE($2, unsigned_event_json),
             trigger = COALESCE($3::publish_trigger, trigger),
             publish_time = COALESCE($4, publish_time),
             queue_id = COALESCE($5, queue_id),
             updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(request.unsigned_event.clone())
    .bind(request.trigger)
    .bind(request.publish_time)
    .bind(request.queue_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(row) = row else {
        return Err(AppError::bad_request(
            "proposal_not_editable",
            "Only proposed items can be edited.",
        ));
    };

    if let Some(unsigned_event) = request.unsigned_event {
        insert_revision(
            &mut tx,
            publish_item_id,
            &session.user_pubkey,
            &unsigned_event,
            Some("edited"),
        )
        .await?;
    }

    tx.commit().await?;
    Ok(Json(row_to_publish_item(row)?))
}

async fn cancel_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_proposal_mutation_access(&state, &session, &item, false).await?;

    sqlx::query(
        "UPDATE publish_items
         SET state = 'CANCELLED', updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'",
    )
    .bind(publish_item_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn reject_proposal(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
    Json(request): Json<RejectProposalRequest>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;

    let row = sqlx::query(
        "UPDATE publish_items
         SET state = 'REJECTED',
             failure_message = $2,
             updated_at = now()
         WHERE id = $1 AND state = 'PROPOSED'
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(request.reason)
    .fetch_optional(&state.pool)
    .await?;

    let Some(row) = row else {
        return Err(AppError::bad_request(
            "proposal_not_rejectable",
            "Only proposed items can be rejected.",
        ));
    };

    Ok(Json(row_to_publish_item(row)?))
}

async fn sign_proposal(
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

async fn batch_sign_proposals(
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
    signed_event
        .validate_signed_for_owner(&item.owner_pubkey, item.publish_time)
        .map_err(|err| AppError::bad_request("signed_event_invalid", err.to_string()))?;

    let event_id = signed_event.id.clone().ok_or_else(|| {
        AppError::bad_request("signed_event_invalid", "Signed event must include id.")
    })?;
    let publish_time = item.publish_time.unwrap_or_else(|| {
        DateTime::<Utc>::from_timestamp(signed_event.created_at, 0).unwrap_or_else(Utc::now)
    });

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

async fn list_publish_items(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<PublishItemResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(PUBLISH_ITEM_SELECT_OWNER)
        .bind(owner_pubkey.as_str())
        .fetch_all(&state.pool)
        .await?;

    rows.into_iter()
        .map(row_to_publish_item)
        .collect::<Result<Vec<_>, AppError>>()
        .map(Json)
}

async fn schedule_signed_event(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<ScheduleSignedEventRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    create_signed_publish_item(&state, &session, request, false).await
}

async fn send_now(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(mut request): Json<ScheduleSignedEventRequest>,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let session = require_session(&state, &headers).await?;
    request.trigger = "SEND_NOW".to_string();
    create_signed_publish_item(&state, &session, request, true).await
}

async fn cancel_publish_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;
    if matches!(item.state.as_str(), "PUBLISHED" | "PUBLISHING") {
        return Err(AppError::bad_request(
            "publish_item_not_cancellable",
            "Publishing or published items cannot be cancelled.",
        ));
    }

    sqlx::query(
        "UPDATE publish_items
         SET state = 'CANCELLED', updated_at = now()
         WHERE id = $1 AND state NOT IN ('PUBLISHED', 'PUBLISHING')",
    )
    .bind(publish_item_id)
    .execute(&state.pool)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn retry_publish_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(publish_item_id): Path<Uuid>,
) -> Result<Json<PublishItemResponse>, AppError> {
    let session = require_session(&state, &headers).await?;
    let item = fetch_publish_item(&state, publish_item_id).await?;
    require_owner(&session, &item.owner_pubkey)?;
    if item.state != "FAILED" {
        return Err(AppError::bad_request(
            "publish_item_not_retryable",
            "Only failed items can be retried.",
        ));
    }

    let next_state = if item.signed_event_json.is_some() {
        "SCHEDULED"
    } else {
        "NEEDS_SIGNATURE"
    };
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "UPDATE publish_items
         SET state = $2::publish_state,
             failure_code = NULL,
             failure_message = NULL,
             failed_at = NULL,
             updated_at = now()
         WHERE id = $1
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(publish_item_id)
    .bind(next_state)
    .fetch_one(&mut *tx)
    .await?;

    if next_state == "SCHEDULED" {
        let publish_time = item.publish_time.unwrap_or_else(Utc::now);
        enqueue_publish_job(&mut tx, publish_item_id, publish_time).await?;
    }
    tx.commit().await?;

    Ok(Json(row_to_publish_item(row)?))
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

async fn list_dvm_requests(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<DvmRequestResponse>>, AppError> {
    let session = require_session(&state, &headers).await?;
    let owner_pubkey = owner_from_headers_or_self(&headers, &session)?;
    require_account_access(&state, &session, &owner_pubkey).await?;

    let rows = sqlx::query(
        "SELECT id, request_event_id, request_pubkey, encrypted, raw_request_event,
                status::text AS status, error, created_at
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

async fn create_signed_publish_item(
    state: &ApiState,
    session: &AuthenticatedSession,
    request: ScheduleSignedEventRequest,
    due_now: bool,
) -> Result<(StatusCode, Json<PublishItemResponse>), AppError> {
    let signed_event = parse_event_json(&request.signed_event)?;
    let owner_pubkey = signed_event.pubkey.clone();
    require_owner(session, &owner_pubkey)?;

    let publish_time = if due_now {
        DateTime::<Utc>::from_timestamp(signed_event.created_at, 0).unwrap_or_else(Utc::now)
    } else {
        request.publish_time.ok_or_else(|| {
            AppError::bad_request("publish_time_required", "Publish time is required.")
        })?
    };

    validate_trigger_inputs(&request.trigger, Some(publish_time), request.queue_id)?;
    validate_queue_for_owner(state, request.queue_id, &owner_pubkey).await?;
    signed_event
        .validate_signed_for_owner(&owner_pubkey, Some(publish_time))
        .map_err(|err| AppError::bad_request("signed_event_invalid", err.to_string()))?;

    let event_id = signed_event.id.ok_or_else(|| {
        AppError::bad_request("signed_event_invalid", "Signed event must include id.")
    })?;

    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "INSERT INTO publish_items
           (owner_pubkey, created_by_pubkey, state, trigger, signed_event_json,
            event_id, publish_time, queue_id)
         VALUES ($1, $2, 'SCHEDULED', $3::publish_trigger, $4, $5, $6, $7)
         RETURNING id, owner_pubkey, created_by_pubkey, state::text AS state,
                   trigger::text AS trigger, unsigned_event_json, signed_event_json,
                   event_id, publish_time, queue_id, published_at, published_to,
                   failure_code, failure_message, created_at, updated_at",
    )
    .bind(owner_pubkey.as_str())
    .bind(session.user_pubkey.as_str())
    .bind(request.trigger)
    .bind(request.signed_event)
    .bind(event_id)
    .bind(publish_time)
    .bind(request.queue_id)
    .fetch_one(&mut *tx)
    .await?;

    let publish_item_id: Uuid = row.try_get("id")?;
    enqueue_publish_job(&mut tx, publish_item_id, publish_time).await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(row_to_publish_item(row)?)))
}

fn validate_trigger_inputs(
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

async fn validate_queue_for_owner(
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

fn parse_event_json(value: &serde_json::Value) -> Result<NostrEvent, AppError> {
    serde_json::from_value(value.clone()).map_err(|_| {
        AppError::bad_request(
            "event_json_invalid",
            "This event can't be published — missing required fields.",
        )
    })
}

async fn fetch_publish_item(
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

async fn require_proposal_mutation_access(
    state: &ApiState,
    session: &AuthenticatedSession,
    item: &PublishItemResponse,
    allow_owner_edit: bool,
) -> Result<(), AppError> {
    if item.state != "PROPOSED" {
        return Err(AppError::bad_request(
            "proposal_not_editable",
            "Only proposed items can be changed.",
        ));
    }

    if allow_owner_edit && session.user_pubkey == item.owner_pubkey {
        return Ok(());
    }

    if session.user_pubkey == item.created_by_pubkey {
        require_account_access(state, session, &item.owner_pubkey).await?;
        return Ok(());
    }

    Err(AppError::forbidden(
        "proposal_not_owned",
        "You can only edit your own proposed items.",
    ))
}

async fn insert_revision(
    tx: &mut Transaction<'_, Postgres>,
    publish_item_id: Uuid,
    edited_by_pubkey: &Pubkey,
    event_json: &serde_json::Value,
    reason: Option<&str>,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO proposal_revisions
           (publish_item_id, edited_by_pubkey, event_json, reason)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(publish_item_id)
    .bind(edited_by_pubkey.as_str())
    .bind(event_json)
    .bind(reason)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn enqueue_publish_job(
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

fn row_to_publish_item(row: sqlx::postgres::PgRow) -> Result<PublishItemResponse, AppError> {
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

fn row_to_dvm_request(row: sqlx::postgres::PgRow) -> Result<DvmRequestResponse, AppError> {
    Ok(DvmRequestResponse {
        id: row.try_get("id")?,
        request_event_id: row.try_get("request_event_id")?,
        request_pubkey: parse_db_pubkey(row.try_get("request_pubkey")?)?,
        encrypted: row.try_get("encrypted")?,
        raw_request_event: row.try_get("raw_request_event")?,
        status: row.try_get("status")?,
        error: row.try_get("error")?,
        created_at: row.try_get("created_at")?,
    })
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

const PUBLISH_ITEM_SELECT_BY_ID: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE id = $1";

const PUBLISH_ITEM_SELECT_OWNER: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1
     ORDER BY created_at DESC";

const PUBLISH_ITEM_SELECT_OWNER_STATE: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1 AND state = $2::publish_state
     ORDER BY created_at DESC";

const PUBLISH_ITEM_SELECT_CREATOR_STATE: &str =
    "SELECT id, owner_pubkey, created_by_pubkey, state::text AS state,
            trigger::text AS trigger, unsigned_event_json, signed_event_json,
            event_id, publish_time, queue_id, published_at, published_to,
            failure_code, failure_message, created_at, updated_at
     FROM publish_items
     WHERE owner_pubkey = $1 AND created_by_pubkey = $2 AND state = $3::publish_state
     ORDER BY created_at DESC";

async fn require_session(
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

fn require_owner(session: &AuthenticatedSession, owner_pubkey: &Pubkey) -> Result<(), AppError> {
    if &session.user_pubkey == owner_pubkey {
        Ok(())
    } else {
        Err(AppError::forbidden(
            "owner_required",
            "You do not have permission to manage this account.",
        ))
    }
}

async fn require_account_access(
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

fn owner_from_headers_or_self(
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

async fn queue_owner(state: &ApiState, queue_id: Uuid) -> Result<Pubkey, AppError> {
    Ok(fetch_queue(state, queue_id).await?.owner_pubkey)
}

async fn fetch_queue(state: &ApiState, queue_id: Uuid) -> Result<Queue, AppError> {
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

fn row_to_queue(row: sqlx::postgres::PgRow) -> Result<Queue, AppError> {
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

fn parse_db_pubkey(value: String) -> Result<Pubkey, AppError> {
    Pubkey::parse(value).map_err(|err| AppError::internal("stored_pubkey_invalid", err.to_string()))
}

#[derive(Debug)]
struct AuthenticatedSession {
    session_id: Uuid,
    user_pubkey: Pubkey,
    expires_at: DateTime<Utc>,
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
    user_pubkey: Pubkey,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct AccountsResponse {
    user_pubkey: Pubkey,
    accounts: Vec<AuthorizedAccount>,
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

#[derive(Debug, Deserialize)]
struct CreateQueueRequest {
    name: String,
    description: Option<String>,
    cadence_seconds: i64,
    start_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpdateQueueRequest {
    name: Option<String>,
    description: Option<String>,
    cadence_seconds: Option<i64>,
    start_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct QueueNextSlotResponse {
    queue_id: Uuid,
    owner_pubkey: Pubkey,
    next_slot: DateTime<Utc>,
    latest_queue_slot: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct CreateProposalRequest {
    owner_pubkey: String,
    unsigned_event: serde_json::Value,
    trigger: String,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct EditProposalRequest {
    unsigned_event: Option<serde_json::Value>,
    trigger: Option<String>,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RejectProposalRequest {
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SignProposalRequest {
    signed_event: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct BatchSignProposalRequest {
    items: Vec<BatchSignProposalItem>,
}

#[derive(Debug, Deserialize)]
struct BatchSignProposalItem {
    proposal_id: Uuid,
    signed_event: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct BatchSignProposalResponse {
    results: Vec<BatchSignProposalResult>,
}

#[derive(Debug, Serialize)]
struct BatchSignProposalResult {
    proposal_id: Uuid,
    item: Option<PublishItemResponse>,
    error: Option<ApiErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ScheduleSignedEventRequest {
    signed_event: serde_json::Value,
    trigger: String,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
struct PublishItemResponse {
    id: Uuid,
    owner_pubkey: Pubkey,
    created_by_pubkey: Pubkey,
    state: String,
    trigger: String,
    unsigned_event_json: Option<serde_json::Value>,
    signed_event_json: Option<serde_json::Value>,
    event_id: Option<String>,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
    published_at: Option<DateTime<Utc>>,
    published_to: Vec<String>,
    failure_code: Option<String>,
    failure_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

#[derive(Debug, Serialize)]
struct DvmRequestResponse {
    id: Uuid,
    request_event_id: String,
    request_pubkey: Pubkey,
    encrypted: bool,
    raw_request_event: serde_json::Value,
    status: String,
    error: Option<String>,
    created_at: DateTime<Utc>,
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

#[derive(Debug)]
struct AppError {
    status: StatusCode,
    body: ApiErrorBody,
}

impl AppError {
    fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }

    fn unauthorized(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, code, message)
    }

    fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, code, message)
    }

    fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    fn internal(code: impl Into<String>, message: impl Into<String>) -> Self {
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
