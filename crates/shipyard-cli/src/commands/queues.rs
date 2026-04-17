use anyhow::bail;
use chrono::{DateTime, Utc};
use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use uuid::Uuid;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum QueuesCommand {
    List,
    Create {
        #[arg(long)]
        name: String,
        #[arg(long)]
        cadence: i64,
        #[arg(long)]
        start: DateTime<Utc>,
        #[arg(long)]
        description: Option<String>,
    },
    Archive {
        id: Uuid,
    },
    Update {
        id: Uuid,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        cadence: Option<i64>,
        #[arg(long)]
        start: Option<DateTime<Utc>>,
        #[arg(long)]
        description: Option<String>,
    },
    NextSlot {
        id: Uuid,
    },
}

pub(crate) async fn run(client: &ApiClient, command: QueuesCommand) -> anyhow::Result<Value> {
    match command {
        QueuesCommand::List => {
            client
                .request(Method::GET, "/v1/queues", None, true, true)
                .await
        }
        QueuesCommand::Create {
            name,
            cadence,
            start,
            description,
        } => {
            client
                .request(
                    Method::POST,
                    "/v1/queues",
                    Some(serde_json::json!({
                        "name": name,
                        "description": description,
                        "cadence_seconds": cadence,
                        "start_at": start
                    })),
                    true,
                    true,
                )
                .await
        }
        QueuesCommand::Archive { id } => {
            client
                .request(
                    Method::POST,
                    &format!("/v1/queues/{id}/archive"),
                    None,
                    true,
                    false,
                )
                .await
        }
        QueuesCommand::Update {
            id,
            name,
            cadence,
            start,
            description,
        } => update_queue(client, id, name, cadence, start, description).await,
        QueuesCommand::NextSlot { id } => {
            client
                .request(
                    Method::GET,
                    &format!("/v1/queues/{id}/next-slot"),
                    None,
                    true,
                    false,
                )
                .await
        }
    }
}

async fn update_queue(
    client: &ApiClient,
    id: Uuid,
    name: Option<String>,
    cadence: Option<i64>,
    start: Option<DateTime<Utc>>,
    description: Option<String>,
) -> anyhow::Result<Value> {
    let mut body = serde_json::Map::new();
    if let Some(name) = name {
        body.insert("name".to_string(), Value::String(name));
    }
    if let Some(cadence) = cadence {
        body.insert("cadence_seconds".to_string(), serde_json::json!(cadence));
    }
    if let Some(start) = start {
        body.insert("start_at".to_string(), serde_json::json!(start));
    }
    if let Some(description) = description {
        body.insert("description".to_string(), Value::String(description));
    }
    if body.is_empty() {
        bail!("queue update requires at least one field");
    }

    client
        .request(
            Method::PATCH,
            &format!("/v1/queues/{id}"),
            Some(Value::Object(body)),
            true,
            false,
        )
        .await
}
