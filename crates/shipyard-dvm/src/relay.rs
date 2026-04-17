use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use shipyard_core::NostrEvent;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub(crate) async fn publish_feedback_to_relays(relay_urls: &[String], event: &NostrEvent) {
    for relay_url in relay_urls {
        match publish_feedback_to_relay(relay_url, event).await {
            Ok(()) => {
                tracing::info!(%relay_url, event_id = ?event.id, "published DVM feedback");
            }
            Err(error) => {
                tracing::warn!(%relay_url, event_id = ?event.id, %error, "failed to publish DVM feedback");
            }
        }
    }
}

async fn publish_feedback_to_relay(relay_url: &str, event: &NostrEvent) -> anyhow::Result<()> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        anyhow::bail!("invalid relay URL");
    }

    let event_id = event.id.as_deref().context("feedback event missing id")?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .context("relay connection timed out")??;

    socket
        .send(Message::Text(
            serde_json::json!(["EVENT", event]).to_string(),
        ))
        .await?;

    loop {
        let Some(message) = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .context("relay OK timed out")?
        else {
            anyhow::bail!("relay closed before OK");
        };
        let Message::Text(text) = message? else {
            continue;
        };
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let Some(array) = value.as_array() else {
            continue;
        };
        if array.first().and_then(serde_json::Value::as_str) != Some("OK") {
            continue;
        }
        if array.get(1).and_then(serde_json::Value::as_str) != Some(event_id) {
            continue;
        }

        let accepted = array
            .get(2)
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if accepted {
            return Ok(());
        }

        let relay_message = array
            .get(3)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("relay rejected feedback event");
        anyhow::bail!(relay_message.to_string());
    }
}

pub(crate) fn parse_relay_urls(relay_urls: &str) -> Vec<String> {
    relay_urls
        .split(',')
        .map(str::trim)
        .filter(|relay_url| !relay_url.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_relay_urls() {
        assert_eq!(
            parse_relay_urls("wss://a.example, wss://b.example ,,"),
            vec!["wss://a.example", "wss://b.example"]
        );
    }
}
