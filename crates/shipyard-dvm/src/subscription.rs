use anyhow::Context;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use shipyard_core::{
    dvm::{
        first_input_event_id_from_tags, relay_targets_from_tags, DvmRequestEvent, DVM_SCHEDULE_KIND,
    },
    NostrEvent,
};
use sqlx::PgPool;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub(crate) const NIP65_RELAY_LIST_KIND: u64 = 10002;
pub(crate) type SharedRelaySubscriptions = Arc<Mutex<HashSet<String>>>;

pub(crate) fn relay_subscription_registry() -> SharedRelaySubscriptions {
    Arc::new(Mutex::new(HashSet::new()))
}

pub(crate) async fn discover_kind_5905_relays_forever(
    pool: PgPool,
    discovery_relay_url: String,
    dvm_pubkey: String,
    subscriptions: SharedRelaySubscriptions,
) {
    loop {
        if let Err(error) =
            discover_kind_5905_relays_once(&pool, &discovery_relay_url, &dvm_pubkey, &subscriptions)
                .await
        {
            tracing::warn!(%discovery_relay_url, %error, "DVM NIP-65 discovery failed");
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn discover_kind_5905_relays_once(
    pool: &PgPool,
    discovery_relay_url: &str,
    dvm_pubkey: &str,
    subscriptions: &SharedRelaySubscriptions,
) -> anyhow::Result<()> {
    if !is_relay_url(discovery_relay_url) {
        anyhow::bail!("invalid relay URL");
    }

    let (mut socket, _) =
        tokio::time::timeout(Duration::from_secs(10), connect_async(discovery_relay_url))
            .await
            .context("relay connection timed out")??;
    let subscription_id = format!("shipyard-dvm-nip65-{}", std::process::id());
    let request = serde_json::json!([
        "REQ",
        subscription_id,
        {
            "kinds": [NIP65_RELAY_LIST_KIND],
            "#k": [DVM_SCHEDULE_KIND.to_string()]
        }
    ]);
    socket.send(Message::Text(request.to_string())).await?;
    tracing::info!(%discovery_relay_url, "DVM NIP-65 discovery subscription established");

    while let Some(message) = socket.next().await {
        let message = message?;
        let Message::Text(text) = message else {
            continue;
        };
        for relay_url in parse_nip65_relay_list_message(&text, &subscription_id)? {
            start_kind_5905_subscription(
                pool.clone(),
                relay_url,
                dvm_pubkey.to_string(),
                Arc::clone(subscriptions),
            );
        }
    }

    Ok(())
}

fn start_kind_5905_subscription(
    pool: PgPool,
    relay_url: String,
    dvm_pubkey: String,
    subscriptions: SharedRelaySubscriptions,
) {
    let should_subscribe = {
        let mut subscriptions = subscriptions
            .lock()
            .expect("DVM relay subscription registry poisoned");
        subscriptions.insert(relay_url.clone())
    };

    if should_subscribe {
        tracing::info!(%relay_url, "starting DVM kind 5905 subscription from NIP-65 relay list");
        tokio::spawn(async move {
            subscribe_relay_forever(pool, relay_url, dvm_pubkey).await;
        });
    }
}

pub(crate) async fn subscribe_relay_forever(pool: PgPool, relay_url: String, dvm_pubkey: String) {
    loop {
        if let Err(error) = subscribe_once(&pool, &relay_url, &dvm_pubkey).await {
            tracing::warn!(%relay_url, %error, "DVM relay subscription failed");
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

async fn subscribe_once(pool: &PgPool, relay_url: &str, dvm_pubkey: &str) -> anyhow::Result<()> {
    if !is_relay_url(relay_url) {
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
        insert_dvm_request(pool, &request_event, dvm_pubkey).await?;
    }

    Ok(())
}

fn parse_nip65_relay_list_message(
    text: &str,
    subscription_id: &str,
) -> anyhow::Result<Vec<String>> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let Some(array) = value.as_array() else {
        return Ok(Vec::new());
    };
    if array.first().and_then(serde_json::Value::as_str) != Some("EVENT") {
        return Ok(Vec::new());
    }
    if array.get(1).and_then(serde_json::Value::as_str) != Some(subscription_id) {
        return Ok(Vec::new());
    }
    let Some(raw_event) = array.get(2) else {
        return Ok(Vec::new());
    };
    let relay_list: NostrEvent = serde_json::from_value(raw_event.clone())?;
    Ok(parse_nip65_relay_list_event(&relay_list))
}

pub(crate) fn parse_nip65_relay_list_event(event: &NostrEvent) -> Vec<String> {
    if event.kind != NIP65_RELAY_LIST_KIND || !has_kind_5905_marker(&event.tags) {
        return Vec::new();
    }

    let mut relays = Vec::new();
    for relay_url in event
        .tags
        .iter()
        .filter(|tag| tag.first().map(String::as_str) == Some("r"))
        .filter_map(|tag| tag.get(1))
        .filter(|relay_url| is_relay_url(relay_url))
    {
        if !relays.iter().any(|existing| existing == relay_url) {
            relays.push(relay_url.clone());
        }
    }
    relays
}

fn has_kind_5905_marker(tags: &[Vec<String>]) -> bool {
    tags.iter().any(|tag| {
        tag.first().map(String::as_str) == Some("k")
            && tag.get(1).map(String::as_str) == Some("5905")
    })
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

async fn insert_dvm_request(
    pool: &PgPool,
    request_event: &DvmRequestEvent,
    dvm_pubkey: &str,
) -> anyhow::Result<()> {
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
        relay_targets_from_tags(&request_event.tags)
    };
    let input_event_id = first_input_event_id_from_tags(&request_event.tags)
        .unwrap_or_else(|| request_event.id.clone());

    sqlx::query(
        "INSERT INTO dvm_requests
           (request_event_id, input_event_id, request_pubkey, dvm_pubkey, encrypted,
            encrypted_tags, decrypted_tags, relays, raw_request_event, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending')
         ON CONFLICT (input_event_id, dvm_pubkey) DO NOTHING",
    )
    .bind(&request_event.id)
    .bind(input_event_id)
    .bind(request_event.pubkey.as_str())
    .bind(dvm_pubkey)
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

fn is_relay_url(relay_url: &str) -> bool {
    relay_url.starts_with("wss://") || relay_url.starts_with("ws://")
}

#[cfg(test)]
#[path = "subscription_tests.rs"]
mod tests;
