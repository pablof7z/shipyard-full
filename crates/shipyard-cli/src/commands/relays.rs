use anyhow::Context;
use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum RelaysCommand {
    List,
    Set { relays: Vec<String> },
    Add { relay: String },
    Remove { relay: String },
}

pub(crate) async fn run(client: &ApiClient, command: RelaysCommand) -> anyhow::Result<Value> {
    match command {
        RelaysCommand::List => {
            client
                .request(Method::GET, "/v1/relays", None, true, true)
                .await
        }
        RelaysCommand::Set { relays } => {
            client
                .request(
                    Method::PUT,
                    "/v1/relays",
                    Some(serde_json::json!({ "relay_urls": relays })),
                    true,
                    true,
                )
                .await
        }
        RelaysCommand::Add { relay } => update_relays(client, RelayMutation::Add(relay)).await,
        RelaysCommand::Remove { relay } => {
            update_relays(client, RelayMutation::Remove(relay)).await
        }
    }
}

enum RelayMutation {
    Add(String),
    Remove(String),
}

async fn update_relays(client: &ApiClient, mutation: RelayMutation) -> anyhow::Result<Value> {
    let current = client
        .request(Method::GET, "/v1/relays", None, true, true)
        .await?;
    let relay_urls = current
        .get("relay_urls")
        .and_then(Value::as_array)
        .context("relay settings response did not include relay_urls")?;
    let mut relays = relay_urls
        .iter()
        .map(|relay| {
            relay
                .as_str()
                .map(ToOwned::to_owned)
                .context("relay URL was not a string")
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    match mutation {
        RelayMutation::Add(relay) => {
            if !relays.iter().any(|existing| existing == &relay) {
                relays.push(relay);
            }
        }
        RelayMutation::Remove(relay) => {
            relays.retain(|existing| existing != &relay);
        }
    }

    client
        .request(
            Method::PUT,
            "/v1/relays",
            Some(serde_json::json!({ "relay_urls": relays })),
            true,
            true,
        )
        .await
}
