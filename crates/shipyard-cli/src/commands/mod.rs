mod accounts;
mod auth;
mod delegates;
mod devices;
mod dvm;
mod media;
mod proposals;
mod publish;
mod queues;
mod relays;

use anyhow::Context;
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde_json::Value;
use std::{fs, path::PathBuf};

use crate::{
    client::ApiClient,
    config::{config_path, Config},
};

#[derive(Debug, Parser)]
#[command(name = "shipyard", version, about = "Shipyard publishing CLI")]
pub(crate) struct Cli {
    #[arg(long, global = true)]
    pub(crate) json: bool,

    #[arg(long, global = true, env = "SHIPYARD_API_URL")]
    api_url: Option<String>,

    #[arg(long, global = true, env = "SHIPYARD_SESSION_TOKEN")]
    session_token: Option<String>,

    #[arg(long, global = true, env = "SHIPYARD_OWNER_PUBKEY")]
    owner_pubkey: Option<String>,

    #[arg(long, global = true, env = "SHIPYARD_CONFIG")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Auth {
        #[command(subcommand)]
        command: auth::AuthCommand,
    },
    Accounts {
        #[command(subcommand)]
        command: accounts::AccountsCommand,
    },
    Delegates {
        #[command(subcommand)]
        command: delegates::DelegatesCommand,
    },
    Queues {
        #[command(subcommand)]
        command: queues::QueuesCommand,
    },
    Relays {
        #[command(subcommand)]
        command: relays::RelaysCommand,
    },
    Propose(publish::ProposeArgs),
    Proposals {
        #[command(subcommand)]
        command: proposals::ProposalsCommand,
    },
    Schedule(publish::ScheduleArgs),
    SendNow(publish::SendNowArgs),
    Posts {
        #[command(subcommand)]
        command: publish::PostsCommand,
    },
    Dvm {
        #[command(subcommand)]
        command: dvm::DvmCommand,
    },
    Media {
        #[command(subcommand)]
        command: media::MediaCommand,
    },
    Status,
}

pub(crate) async fn run(cli: Cli) -> anyhow::Result<Value> {
    let config_path = config_path(cli.config)?;
    let mut config = Config::load(&config_path)?;
    let api_url = cli
        .api_url
        .or_else(|| config.api_url.clone())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    let session_token = cli.session_token.or_else(|| config.session_token.clone());
    let owner_pubkey = cli.owner_pubkey.or_else(|| config.default_account.clone());
    let client = ApiClient::new(api_url, session_token, owner_pubkey);

    match cli.command {
        Command::Status => {
            client
                .request(Method::GET, "/v1/status", None, false, false)
                .await
        }
        Command::Auth { command } => auth::run(&client, &mut config, &config_path, command).await,
        Command::Accounts { command } => {
            accounts::run(&client, &mut config, &config_path, command).await
        }
        Command::Delegates { command } => delegates::run(&client, command).await,
        Command::Queues { command } => queues::run(&client, command).await,
        Command::Relays { command } => relays::run(&client, command).await,
        Command::Propose(args) => publish::create_proposal(&client, args).await,
        Command::Proposals { command } => proposals::run(&client, command).await,
        Command::Schedule(args) => publish::schedule(&client, args).await,
        Command::SendNow(args) => publish::send_now(&client, args).await,
        Command::Posts { command } => publish::run_posts(&client, command).await,
        Command::Dvm { command } => dvm::run(&client, command).await,
        Command::Media { command } => media::run(command).await,
    }
}

pub(crate) fn read_json_file(path: PathBuf) -> anyhow::Result<Value> {
    let contents = fs::read_to_string(path).context("failed to read JSON file")?;
    serde_json::from_str(&contents).context("failed to parse JSON file")
}
