use anyhow::Context;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shipyard_core::dvm::{DvmRequestEvent, DVM_SCHEDULE_KIND};
use sqlx::PgPool;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub(crate) async fn subscribe_relay_forever(pool: PgPool, relay_url: String) {
    loop {
        if let Err(error) = subscribe_once(&pool, &relay_url).await {
            tracing::warn!(%relay_url, %error, "DVM relay subscription failed");
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn subscribe_once(pool: &PgPool, relay_url: &str) -> anyhow::Result<()> {
    if !(relay_url.starts_with("wss://") || relay_url.starts_with("ws://")) {
        anyhow::bail!("invalid relay URL");
    }

    let (mut socket, _) = tokio::time::timeout(Duration::from_secs(10), connect_async(relay_url))
        .await
        .context("relay connection timed out")??;
    let subscription_id = format!("shipyard-dvm-{}", std::process::id());
    let since = Utc::now().timestamp().saturating_sub(60);
    let request = serde_json::json!([
        "REQ",
        subscription_id,
        {
            "kinds": [DVM_SCHEDULE_KIND],
            "since": since
        }
    ]);
    socket.send(Message::Text(request.to_string())).await?;
    tracing::info!(%relay_url, "DVM relay subscription established");

    while let Some(message) = socket.next().await {
        let message = message?;
        let Message::Text(text) = message else {
            continue;
        };
        let Some(request_event) = parse_relay_event_message(&text, &subscription_id)? else {
            continue;
        };
        insert_dvm_request(pool, &request_event).await?;
    }

    Ok(())
}

fn parse_relay_event_message(
    text: &str,
    subscription_id: &str,
) -> anyhow::Result<Option<DvmRequestEvent>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let Some(array) = value.as_array() else {
        return Ok(None);
    };
    if array.first().and_then(serde_json::Value::as_str) != Some("EVENT") {
        return Ok(None);
    }
    if array.get(1).and_then(serde_json::Value::as_str) != Some(subscription_id) {
        return Ok(None);
    }
    let Some(raw_event) = array.get(2) else {
        return Ok(None);
    };
    let request_event: DvmRequestEvent = serde_json::from_value(raw_event.clone())?;
    if request_event.kind != DVM_SCHEDULE_KIND {
        return Ok(None);
    }
    Ok(Some(request_event))
}

async fn insert_dvm_request(pool: &PgPool, request_event: &DvmRequestEvent) -> anyhow::Result<()> {
    let raw_request_event = serde_json::to_value(request_event)?;
    let encrypted = has_encrypted_tag(request_event);
    let encrypted_tags = if encrypted {
        Some(serde_json::Value::String(request_event.content.clone()))
    } else {
        None
    };
    let decrypted_tags = if encrypted {
        None
    } else {
        Some(serde_json::to_value(&request_event.tags)?)
    };
    let relays = if encrypted {
        Vec::new()
    } else {
        relay_targets(&request_event.tags)
    };

    sqlx::query(
        "INSERT INTO dvm_requests
           (request_event_id, request_pubkey, encrypted, encrypted_tags, decrypted_tags,
            relays, raw_request_event, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'received')
         ON CONFLICT (request_event_id) DO NOTHING",
    )
    .bind(&request_event.id)
    .bind(request_event.pubkey.as_str())
    .bind(encrypted)
    .bind(encrypted_tags)
    .bind(decrypted_tags)
    .bind(&relays)
    .bind(raw_request_event)
    .execute(pool)
    .await?;

    Ok(())
}

fn has_encrypted_tag(request_event: &DvmRequestEvent) -> bool {
    request_event
        .tags
        .iter()
        .any(|tag| tag.first().map(String::as_str) == Some("encrypted"))
}

fn relay_targets(tags: &[Vec<String>]) -> Vec<String> {
    tags.iter()
        .filter(|tag| tag.first().map(String::as_str) == Some("relays"))
        .flat_map(|tag| tag.iter().skip(1))
        .filter(|value| value.starts_with("wss://"))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_core::Pubkey;

    fn request_event() -> DvmRequestEvent {
        DvmRequestEvent {
            id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            pubkey: Pubkey::parse(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )
            .unwrap(),
            created_at: 1_776_432_000,
            kind: DVM_SCHEDULE_KIND,
            tags: vec![],
            content: String::new(),
            sig: Some("sig".into()),
        }
    }

    #[test]
    fn parses_relay_event_for_matching_subscription() {
        let raw = serde_json::json!(["EVENT", "sub", request_event()]).to_string();
        let parsed = parse_relay_event_message(&raw, "sub").unwrap().unwrap();
        assert_eq!(parsed.kind, DVM_SCHEDULE_KIND);
    }

    #[test]
    fn ignores_other_subscription_ids() {
        let raw = serde_json::json!(["EVENT", "other", request_event()]).to_string();
        assert!(parse_relay_event_message(&raw, "sub").unwrap().is_none());
    }
}
