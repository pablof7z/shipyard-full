use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum DvmCommand {
    Requests,
}

pub(crate) async fn run(client: &ApiClient, command: DvmCommand) -> anyhow::Result<Value> {
    match command {
        DvmCommand::Requests => {
            client
                .request(Method::GET, "/v1/dvm/requests", None, true, true)
                .await
        }
    }
}
