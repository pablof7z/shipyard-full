mod accounts;
mod auth;
mod delegates;
mod devices;
mod dvm;
mod models;
mod proposals;
mod publish_items;
mod queues;
mod relays;

use axum::{extract::State, routing::get, Json, Router};

pub use models::ApiState;
use models::AppError;

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/status", get(status))
        .merge(auth::router())
        .merge(accounts::router())
        .merge(delegates::router())
        .merge(queues::router())
        .merge(proposals::router())
        .merge(publish_items::router())
        .merge(relays::router())
        .merge(dvm::router())
        .merge(devices::router())
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
