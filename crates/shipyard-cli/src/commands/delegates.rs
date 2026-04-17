use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum DelegatesCommand {
    List,
    Invite { delegate_pubkey: String },
    Revoke { delegate_pubkey: String },
}

pub(crate) async fn run(client: &ApiClient, command: DelegatesCommand) -> anyhow::Result<Value> {
    match command {
        DelegatesCommand::List => {
            let owner = client.required_owner()?;
            client
                .request(
                    Method::GET,
                    &format!("/v1/accounts/{owner}/delegates"),
                    None,
                    true,
                    false,
                )
                .await
        }
        DelegatesCommand::Invite { delegate_pubkey } => {
            let owner = client.required_owner()?;
            client
                .request(
                    Method::POST,
                    &format!("/v1/accounts/{owner}/delegates"),
                    Some(serde_json::json!({ "delegate_pubkey": delegate_pubkey })),
                    true,
                    false,
                )
                .await
        }
        DelegatesCommand::Revoke { delegate_pubkey } => {
            let owner = client.required_owner()?;
            client
                .request(
                    Method::DELETE,
                    &format!("/v1/accounts/{owner}/delegates/{delegate_pubkey}"),
                    None,
                    true,
                    false,
                )
                .await
        }
    }
}
