mod routes;

use anyhow::Context;
use axum::Router;
use routes::{router, ApiState};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shipyard_api=info,tower_http=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let bind_addr = std::env::var("SHIPYARD_API_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let state = ApiState::from_env().await?;
    let app = Router::new()
        .nest("/v1", router(state))
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind API on {bind_addr}"))?;

    tracing::info!(%bind_addr, "shipyard-api listening");
    axum::serve(listener, app).await?;
    Ok(())
}
