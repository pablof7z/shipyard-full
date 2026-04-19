use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::Value;
use shipyard_core::{NostrEvent, Pubkey};
use std::{fs, path::PathBuf};
use uuid::Uuid;

use crate::client::ApiClient;

#[derive(Debug, Parser)]
pub(crate) struct ProposeArgs {
    #[arg(long)]
    to: String,
    #[arg(long)]
    content: Option<String>,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    time: Option<DateTime<Utc>>,
    #[arg(long)]
    queue: Option<Uuid>,
}

#[derive(Debug, Parser)]
pub(crate) struct ScheduleArgs {
    #[arg(long)]
    event_json: PathBuf,
    #[arg(long)]
    queue: Option<Uuid>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum PostsCommand {
    List,
    Show { id: Uuid },
    Cancel { id: Uuid },
    Retry { id: Uuid },
}

pub(crate) async fn create_proposal(
    client: &ApiClient,
    args: ProposeArgs,
) -> anyhow::Result<Value> {
    let owner = Pubkey::parse(args.to.clone()).context("invalid owner pubkey")?;
    let content = match (args.content, args.file) {
        (Some(content), None) => content,
        (None, Some(path)) => fs::read_to_string(path).context("failed to read proposal file")?,
        (Some(_), Some(_)) => bail!("use either --content or --file, not both"),
        (None, None) => bail!("proposal requires --content or --file"),
    };
    let trigger = trigger_parts(args.time, args.queue)?;
    let created_at = trigger
        .publish_time
        .map_or_else(|| Utc::now().timestamp(), |time| time.timestamp());
    let event = NostrEvent::unsigned(owner, created_at, 1, vec![], content);

    client
        .request(
            Method::POST,
            "/v1/proposals",
            Some(serde_json::json!({
                "owner_pubkey": event.pubkey.as_str(),
                "unsigned_event": event,
                "trigger": trigger.name,
                "publish_time": trigger.publish_time,
                "queue_id": trigger.queue_id
            })),
            true,
            false,
        )
        .await
}

pub(crate) async fn schedule(client: &ApiClient, args: ScheduleArgs) -> anyhow::Result<Value> {
    let signed_event = super::read_json_file(args.event_json)?;
    let trigger = schedule_trigger_parts(&signed_event, args.queue)?;
    client
        .request(
            Method::POST,
            "/v1/publish-items/schedule",
            Some(serde_json::json!({
                "signed_event": signed_event,
                "trigger": trigger.name,
                "publish_time": trigger.publish_time,
                "queue_id": trigger.queue_id
            })),
            true,
            false,
        )
        .await
}

pub(crate) async fn run_posts(client: &ApiClient, command: PostsCommand) -> anyhow::Result<Value> {
    match command {
        PostsCommand::List => {
            client
                .request(Method::GET, "/v1/publish-items", None, true, true)
                .await
        }
        PostsCommand::Show { id } => {
            let items = client
                .request(Method::GET, "/v1/publish-items", None, true, true)
                .await?;
            items
                .as_array()
                .and_then(|items| {
                    items
                        .iter()
                        .find(|item| {
                            item.get("id").and_then(Value::as_str) == Some(&id.to_string())
                        })
                        .cloned()
                })
                .with_context(|| format!("publish item {id} not found"))
        }
        PostsCommand::Cancel { id } => {
            client
                .request(
                    Method::POST,
                    &format!("/v1/publish-items/{id}/cancel"),
                    None,
                    true,
                    false,
                )
                .await
        }
        PostsCommand::Retry { id } => {
            client
                .request(
                    Method::POST,
                    &format!("/v1/publish-items/{id}/retry"),
                    None,
                    true,
                    false,
                )
                .await
        }
    }
}

fn trigger_parts(time: Option<DateTime<Utc>>, queue: Option<Uuid>) -> anyhow::Result<TriggerParts> {
    match (time, queue) {
        (Some(time), None) => Ok(TriggerParts {
            name: "TIME",
            publish_time: Some(time),
            queue_id: None,
        }),
        (None, Some(queue)) => Ok(TriggerParts {
            name: "QUEUE",
            publish_time: None,
            queue_id: Some(queue),
        }),
        (Some(_), Some(_)) => bail!("use either --time or --queue, not both"),
        (None, None) => bail!("provide --time or --queue"),
    }
}

fn schedule_trigger_parts(
    signed_event: &Value,
    queue: Option<Uuid>,
) -> anyhow::Result<TriggerParts> {
    if let Some(queue) = queue {
        return Ok(TriggerParts {
            name: "QUEUE",
            publish_time: None,
            queue_id: Some(queue),
        });
    }

    let created_at = signed_event
        .get("created_at")
        .and_then(Value::as_i64)
        .context("signed event must include numeric created_at")?;
    let publish_time = DateTime::<Utc>::from_timestamp(created_at, 0)
        .context("signed event created_at is outside the supported timestamp range")?;

    Ok(TriggerParts {
        name: "TIME",
        publish_time: Some(publish_time),
        queue_id: None,
    })
}

#[derive(Debug)]
struct TriggerParts {
    name: &'static str,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}
