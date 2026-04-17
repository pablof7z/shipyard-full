use futures_util::{SinkExt, StreamExt};
use shipyard_core::NostrEvent;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub(crate) async fn publish_to_relay(relay_url: &str, event: &NostrEvent) -> Result<(), String> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        return Err("invalid relay URL".to_string());
    }

    let event_id = event
        .id
        .as_deref()
        .ok_or_else(|| "event missing id".to_string())?;
    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .map_err(|_| "relay connection timed out".to_string())?
        .map_err(|error| format!("relay connection failed: {error}"))?;

    let message = serde_json::json!(["EVENT", event]).to_string();
    socket
        .send(Message::Text(message))
        .await
        .map_err(|error| format!("relay send failed: {error}"))?;

    loop {
        let next = tokio::time::timeout(Duration::from_secs(10), socket.next())
            .await
            .map_err(|_| "relay OK timed out".to_string())?;

        let Some(message) = next else {
            return Err("relay closed before OK".to_string());
        };
        let message = message.map_err(|error| format!("relay read failed: {error}"))?;
        let Message::Text(text) = message else {
            continue;
        };

        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|error| format!("relay sent invalid JSON: {error}"))?;
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
        let relay_message = array
            .get(3)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("relay rejected event");
        return if accepted {
            Ok(())
        } else {
            Err(relay_message.to_string())
        };
    }
}
