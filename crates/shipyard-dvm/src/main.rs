mod feedback;
mod processing;
mod relay;
mod subscription;

use anyhow::Context;
use shipyard_core::pubkey_from_secret_hex;
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
        .context("SHIPYARD_DVM_RELAYS is required for NIP-65 relay discovery")?;
    let feedback_secret_hex = std::env::var("SHIPYARD_DVM_SECRET_KEY")
        .context("SHIPYARD_DVM_SECRET_KEY is required to sign kind 7000 feedback")?;
    let dvm_pubkey = pubkey_from_secret_hex(&feedback_secret_hex)?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let tick_seconds = std::env::var("SHIPYARD_DVM_TICK_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);

    let discovery_relays = relay::parse_relay_urls(&relay_urls);
    let subscriptions = subscription::relay_subscription_registry();
    tracing::info!(?discovery_relays, %dvm_pubkey, tick_seconds, "shipyard-dvm starting");

    for discovery_relay in discovery_relays.clone() {
        let pool = pool.clone();
        let subscriptions = subscriptions.clone();
        let dvm_pubkey = dvm_pubkey.as_str().to_string();
        tokio::spawn(async move {
            subscription::discover_kind_5905_relays_forever(
                pool,
                discovery_relay,
                dvm_pubkey,
                subscriptions,
            )
            .await;
        });
    }

    loop {
        tokio::select! {
            result = processing::process_pending_dvm_requests(&pool, &feedback_secret_hex) => {
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
