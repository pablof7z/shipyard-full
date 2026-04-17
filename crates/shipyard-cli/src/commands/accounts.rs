use anyhow::Context;
use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use shipyard_core::Pubkey;
use std::path::PathBuf;

use crate::{client::ApiClient, config::Config};

#[derive(Debug, Subcommand)]
pub(crate) enum AccountsCommand {
    List,
    Use { owner_pubkey: String },
}

pub(crate) async fn run(
    client: &ApiClient,
    config: &mut Config,
    config_path: &PathBuf,
    command: AccountsCommand,
) -> anyhow::Result<Value> {
    match command {
        AccountsCommand::List => {
            client
                .request(Method::GET, "/v1/accounts", None, true, false)
                .await
        }
        AccountsCommand::Use { owner_pubkey } => {
            Pubkey::parse(owner_pubkey.clone()).context("invalid owner pubkey")?;
            config.default_account = Some(owner_pubkey.clone());
            config.save(config_path)?;
            Ok(serde_json::json!({
                "status": "active_account_saved",
                "owner_pubkey": owner_pubkey
            }))
        }
    }
}
