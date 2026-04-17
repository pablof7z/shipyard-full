use anyhow::bail;
use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use std::path::PathBuf;

use crate::{client::ApiClient, config::Config};

#[derive(Debug, Subcommand)]
pub(crate) enum AuthCommand {
    Login {
        #[arg(long)]
        event_json: Option<PathBuf>,
        #[arg(long)]
        session_token: Option<String>,
    },
    Logout,
    Status,
}

pub(crate) async fn run(
    client: &ApiClient,
    config: &mut Config,
    config_path: &PathBuf,
    command: AuthCommand,
) -> anyhow::Result<Value> {
    match command {
        AuthCommand::Login {
            event_json,
            session_token,
        } => login(client, config, config_path, event_json, session_token).await,
        AuthCommand::Logout => logout(client, config, config_path).await,
        AuthCommand::Status => {
            client
                .request(Method::GET, "/v1/auth/session", None, true, false)
                .await
        }
    }
}

async fn login(
    client: &ApiClient,
    config: &mut Config,
    config_path: &PathBuf,
    event_json: Option<PathBuf>,
    session_token: Option<String>,
) -> anyhow::Result<Value> {
    if let Some(session_token) = session_token {
        config.session_token = Some(session_token.clone());
        config.save(config_path)?;
        Ok(serde_json::json!({
            "status": "stored",
            "session_token": session_token
        }))
    } else if let Some(event_json) = event_json {
        let event = super::read_json_file(event_json)?;
        let response = client
            .request(
                Method::POST,
                "/v1/auth/login",
                Some(serde_json::json!({ "event": event })),
                false,
                false,
            )
            .await?;
        config.session_token = response
            .get("session_token")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        config.save(config_path)?;
        Ok(response)
    } else {
        bail!("auth login requires --event-json or --session-token")
    }
}

async fn logout(
    client: &ApiClient,
    config: &mut Config,
    config_path: &PathBuf,
) -> anyhow::Result<Value> {
    let response = client
        .request(Method::POST, "/v1/auth/logout", None, true, false)
        .await?;
    config.session_token = None;
    config.save(config_path)?;
    Ok(response)
}
