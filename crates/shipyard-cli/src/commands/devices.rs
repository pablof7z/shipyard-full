use anyhow::{bail, Context};
use clap::Subcommand;
use reqwest::Method;
use serde_json::Value;
use shipyard_core::Pubkey;
use uuid::Uuid;

use crate::client::ApiClient;

#[derive(Debug, Subcommand)]
pub(crate) enum DevicesCommand {
    /// List registered device tokens for the authenticated session
    List,
    /// Register or refresh a device token
    Register {
        #[arg(long)]
        platform: String,
        #[arg(long)]
        token: String,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        enabled: Option<bool>,
    },
    /// Update the owner binding or enabled state for a device token
    Update {
        id: Uuid,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        enabled: Option<bool>,
    },
    /// Delete a device token
    Delete { id: Uuid },
}

pub(crate) async fn run(client: &ApiClient, command: DevicesCommand) -> anyhow::Result<Value> {
    match command {
        DevicesCommand::List => {
            client
                .request(Method::GET, "/v1/devices", None, true, false)
                .await
        }
        DevicesCommand::Register {
            platform,
            token,
            owner,
            enabled,
        } => {
            client
                .request(
                    Method::POST,
                    "/v1/devices",
                    Some(register_body(platform, token, owner, enabled)?),
                    true,
                    false,
                )
                .await
        }
        DevicesCommand::Update { id, owner, enabled } => {
            client
                .request(
                    Method::PATCH,
                    &format!("/v1/devices/{id}"),
                    Some(update_body(owner, enabled)?),
                    true,
                    false,
                )
                .await
        }
        DevicesCommand::Delete { id } => {
            client
                .request(
                    Method::DELETE,
                    &format!("/v1/devices/{id}"),
                    None,
                    true,
                    false,
                )
                .await
        }
    }
}

fn register_body(
    platform: String,
    token: String,
    owner: Option<String>,
    enabled: Option<bool>,
) -> anyhow::Result<Value> {
    if token.trim().is_empty() {
        bail!("device register requires a non-empty --token");
    }

    let mut body = serde_json::Map::new();
    body.insert("platform".to_string(), Value::String(platform));
    body.insert("token".to_string(), Value::String(token));
    if let Some(owner) = owner {
        body.insert("owner_pubkey".to_string(), owner_value(owner)?);
    }
    if let Some(enabled) = enabled {
        body.insert("enabled".to_string(), Value::Bool(enabled));
    }
    Ok(Value::Object(body))
}

fn update_body(owner: Option<String>, enabled: Option<bool>) -> anyhow::Result<Value> {
    let mut body = serde_json::Map::new();
    if let Some(owner) = owner {
        body.insert("owner_pubkey".to_string(), owner_value(owner)?);
    }
    if let Some(enabled) = enabled {
        body.insert("enabled".to_string(), Value::Bool(enabled));
    }
    if body.is_empty() {
        bail!("device update requires --owner or --enabled");
    }
    Ok(Value::Object(body))
}

fn owner_value(owner: String) -> anyhow::Result<Value> {
    Pubkey::parse(owner.clone()).context("invalid owner pubkey")?;
    Ok(Value::String(owner))
}

#[cfg(test)]
mod tests {
    use clap::Parser;
    use serde_json::json;
    use uuid::Uuid;

    use super::{register_body, update_body, DevicesCommand};

    #[derive(Debug, Parser)]
    struct Harness {
        #[command(subcommand)]
        command: DevicesCommand,
    }

    #[test]
    fn device_register_requires_token() {
        let result = Harness::try_parse_from(["shipyard", "register", "--platform", "ios"]);
        assert!(result.is_err());
    }

    #[test]
    fn device_update_requires_at_least_one_field() {
        let result = update_body(None, None);
        assert!(result.is_err());
    }

    #[test]
    fn device_register_body_matches_api_contract() {
        let body = register_body(
            "ios".to_string(),
            "apns-token".to_string(),
            Some(valid_owner()),
            Some(false),
        )
        .unwrap();

        assert_eq!(
            body,
            json!({
                "platform": "ios",
                "token": "apns-token",
                "owner_pubkey": valid_owner(),
                "enabled": false
            })
        );
    }

    #[test]
    fn device_update_body_matches_api_contract() {
        let body = update_body(Some(valid_owner()), Some(true)).unwrap();

        assert_eq!(
            body,
            json!({
                "owner_pubkey": valid_owner(),
                "enabled": true
            })
        );
    }

    #[test]
    fn device_delete_accepts_uuid() {
        let id = Uuid::new_v4();
        let parsed = Harness::try_parse_from(["shipyard", "delete", &id.to_string()]).unwrap();

        assert!(matches!(
            parsed.command,
            DevicesCommand::Delete { id: parsed_id } if parsed_id == id
        ));
    }

    fn valid_owner() -> String {
        "0000000000000000000000000000000000000000000000000000000000000001".to_string()
    }
}
