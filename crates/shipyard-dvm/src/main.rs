mod feedback;
mod processing;
mod relay;
mod subscription;

use anyhow::Context;
use shipyard_core::pubkey_from_secret_hex;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Returns `true` when every hex character in a 64-char key is identical,
/// indicating a trivially-guessable placeholder (e.g. all-`1`s or all-`0`s).
fn is_weak_secret_key(key_hex: &str) -> bool {
    if key_hex.len() != 64 {
        return false;
    }
    let first = match key_hex.chars().next() {
        Some(c) => c,
        None => return false,
    };
    key_hex.chars().all(|c| c == first)
}

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

    if is_weak_secret_key(&feedback_secret_hex) {
        anyhow::bail!(
            "SHIPYARD_DVM_SECRET_KEY looks like a placeholder \
             (all identical hex digits). Set a real 32-byte random key."
        );
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_weak_secret_key_flags_all_same_hex_digit_keys() {
        assert!(is_weak_secret_key(
            "1111111111111111111111111111111111111111111111111111111111111111"
        ));
        assert!(is_weak_secret_key(
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
        assert!(is_weak_secret_key(
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        ));
    }

    #[test]
    fn is_weak_secret_key_accepts_normal_looking_keys() {
        assert!(!is_weak_secret_key(
            "3333333333333333333333333333333333333333333333333333333334444444"
        ));
        assert!(!is_weak_secret_key(
            "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"
        ));
    }

    #[test]
    fn is_weak_secret_key_rejects_wrong_length_strings() {
        assert!(!is_weak_secret_key("1111"));
        assert!(!is_weak_secret_key(""));
    }
}
