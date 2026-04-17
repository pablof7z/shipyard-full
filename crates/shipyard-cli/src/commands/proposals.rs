use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum ProposalsCommand {
    List,
    Delete {
        id: Uuid,
    },
    Reject {
        id: Uuid,
        #[arg(long)]
        reason: Option<String>,
    },
    Sign {
        id: Uuid,
        #[arg(long)]
        event_json: PathBuf,
    },
    BatchSign {
        #[arg(long)]
        file: PathBuf,
    },
}

pub(crate) async fn run(client: &ApiClient, command: ProposalsCommand) -> anyhow::Result<Value> {
    match command {
        ProposalsCommand::List => {
            client
                .request(Method::GET, "/v1/proposals", None, true, true)
                .await
        }
        ProposalsCommand::Delete { id } => {
            client
                .request(
                    Method::DELETE,
                    &format!("/v1/proposals/{id}"),
                    None,
                    true,
                    false,
                )
                .await
        }
        ProposalsCommand::Reject { id, reason } => {
            client
                .request(
                    Method::POST,
                    &format!("/v1/proposals/{id}/reject"),
                    Some(serde_json::json!({ "reason": reason })),
                    true,
                    false,
                )
                .await
        }
        ProposalsCommand::Sign { id, event_json } => {
            client
                .request(
                    Method::POST,
                    &format!("/v1/proposals/{id}/sign"),
                    Some(serde_json::json!({
                        "signed_event": super::read_json_file(event_json)?
                    })),
                    true,
                    false,
                )
                .await
        }
        ProposalsCommand::BatchSign { file } => {
            let value = super::read_json_file(file)?;
            let body = if value.is_array() {
                serde_json::json!({ "items": value })
            } else {
                value
            };
            client
                .request(
                    Method::POST,
                    "/v1/proposals/batch-sign",
                    Some(body),
                    true,
                    false,
                )
                .await
        }
    }
}
