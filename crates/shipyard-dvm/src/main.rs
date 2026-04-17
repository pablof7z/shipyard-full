mod feedback;
mod processing;
mod relay;
mod subscription;

use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "shipyard_dvm=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let relay_urls = std::env::var("SHIPYARD_DVM_RELAYS")
        .context("SHIPYARD_DVM_RELAYS is required for kind 5905 subscriptions")?;
    let feedback_secret_hex = std::env::var("SHIPYARD_DVM_SECRET_KEY")
        .context("SHIPYARD_DVM_SECRET_KEY is required to sign kind 7000 feedback")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let tick_seconds = std::env::var("SHIPYARD_DVM_TICK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);

    let relay_urls = relay::parse_relay_urls(&relay_urls);
    tracing::info!(?relay_urls, tick_seconds, "shipyard-dvm starting");

    for relay_url in relay_urls.clone() {
        let pool = pool.clone();
        tokio::spawn(async move {
            subscription::subscribe_relay_forever(pool, relay_url).await;
        });
    }

    loop {
        tokio::select! {
            result = processing::process_pending_dvm_requests(&pool, &feedback_secret_hex, &relay_urls) => {
                if let Err(error) = result {
                    tracing::error!(%error, "DVM processing tick failed");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shipyard-dvm shutting down");
                break;
            }
        }

        tokio::time::sleep(Duration::from_secs(tick_seconds)).await;
    }

    Ok(())
}
