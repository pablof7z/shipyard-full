use anyhow::{bail, Context};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shipyard_core::{NostrEvent, Pubkey};
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[derive(Debug, Parser)]
#[command(name = "shipyard", version, about = "Shipyard publishing CLI")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,

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
        command: AuthCommand,
    },
    Accounts {
        #[command(subcommand)]
        command: AccountsCommand,
    },
    Delegates {
        #[command(subcommand)]
        command: DelegatesCommand,
    },
    Queues {
        #[command(subcommand)]
        command: QueuesCommand,
    },
    Relays {
        #[command(subcommand)]
        command: RelaysCommand,
    },
    Propose(ProposeArgs),
    Proposals {
        #[command(subcommand)]
        command: ProposalsCommand,
    },
    Schedule(ScheduleArgs),
    SendNow(SendNowArgs),
    Posts {
        #[command(subcommand)]
        command: PostsCommand,
    },
    Dvm {
        #[command(subcommand)]
        command: DvmCommand,
    },
    Status,
}

#[derive(Debug, Subcommand)]
enum AuthCommand {
    Login {
        #[arg(long)]
        event_json: Option<PathBuf>,
        #[arg(long)]
        session_token: Option<String>,
    },
    Logout,
    Status,
}

#[derive(Debug, Subcommand)]
enum AccountsCommand {
    List,
    Use { owner_pubkey: String },
}

#[derive(Debug, Subcommand)]
enum DelegatesCommand {
    List,
    Invite { delegate_pubkey: String },
    Revoke { delegate_pubkey: String },
}

#[derive(Debug, Subcommand)]
enum QueuesCommand {
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

#[derive(Debug, Subcommand)]
enum RelaysCommand {
    List,
    Set { relays: Vec<String> },
    Add { relay: String },
    Remove { relay: String },
}

#[derive(Debug, Parser)]
struct ProposeArgs {
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

#[derive(Debug, Subcommand)]
enum ProposalsCommand {
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

#[derive(Debug, Parser)]
struct ScheduleArgs {
    #[arg(long)]
    event_json: PathBuf,
    #[arg(long)]
    time: Option<DateTime<Utc>>,
    #[arg(long)]
    queue: Option<Uuid>,
}

#[derive(Debug, Parser)]
struct SendNowArgs {
    #[arg(long)]
    event_json: PathBuf,
}

#[derive(Debug, Subcommand)]
enum PostsCommand {
    List,
    Show { id: Uuid },
    Cancel { id: Uuid },
    Retry { id: Uuid },
}

#[derive(Debug, Subcommand)]
enum DvmCommand {
    Requests,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let json_output = cli.json;
    match run(cli).await {
        Ok(output) => print_output(json_output, &output),
        Err(error) => {
            if json_output {
                eprintln!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "code": "cli_error",
                        "message": error.to_string()
                    }))
                    .unwrap_or_else(|_| "{\"code\":\"cli_error\"}".to_string())
                );
            } else {
                eprintln!("Shipyard CLI: {error}");
            }
            std::process::exit(1);
        }
    }
}

async fn run(cli: Cli) -> anyhow::Result<Value> {
    let config_path = config_path(cli.config)?;
    let mut config = Config::load(&config_path)?;
    let api_url = cli
        .api_url
        .or_else(|| config.api_url.clone())
        .unwrap_or_else(|| "http://localhost:8080".to_string());
    let session_token = cli.session_token.or_else(|| config.session_token.clone());
    let owner_pubkey = cli.owner_pubkey.or_else(|| config.default_account.clone());
    let client = ApiClient {
        http: reqwest::Client::new(),
        api_url,
        session_token,
        owner_pubkey,
    };

    match cli.command {
        Command::Status => {
            client
                .request(Method::GET, "/v1/status", None, false, false)
                .await
        }
        Command::Auth { command } => match command {
            AuthCommand::Login {
                event_json,
                session_token,
            } => {
                if let Some(session_token) = session_token {
                    config.session_token = Some(session_token.clone());
                    config.save(&config_path)?;
                    Ok(serde_json::json!({
                        "status": "stored",
                        "session_token": session_token
                    }))
                } else if let Some(event_json) = event_json {
                    let event = read_json_file(event_json)?;
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
                    config.save(&config_path)?;
                    Ok(response)
                } else {
                    bail!("auth login requires --event-json or --session-token")
                }
            }
            AuthCommand::Logout => {
                let response = client
                    .request(Method::POST, "/v1/auth/logout", None, true, false)
                    .await?;
                config.session_token = None;
                config.save(&config_path)?;
                Ok(response)
            }
            AuthCommand::Status => {
                client
                    .request(Method::GET, "/v1/auth/session", None, true, false)
                    .await
            }
        },
        Command::Accounts { command } => match command {
            AccountsCommand::List => {
                client
                    .request(Method::GET, "/v1/accounts", None, true, false)
                    .await
            }
            AccountsCommand::Use { owner_pubkey } => {
                Pubkey::parse(owner_pubkey.clone()).context("invalid owner pubkey")?;
                config.default_account = Some(owner_pubkey.clone());
                config.save(&config_path)?;
                Ok(serde_json::json!({
                    "status": "active_account_saved",
                    "owner_pubkey": owner_pubkey
                }))
            }
        },
        Command::Delegates { command } => match command {
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
        },
        Command::Queues { command } => match command {
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
            } => update_queue(&client, id, name, cadence, start, description).await,
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
        },
        Command::Relays { command } => match command {
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
            RelaysCommand::Add { relay } => update_relays(&client, RelayMutation::Add(relay)).await,
            RelaysCommand::Remove { relay } => {
                update_relays(&client, RelayMutation::Remove(relay)).await
            }
        },
        Command::Propose(args) => create_proposal(&client, args).await,
        Command::Proposals { command } => match command {
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
                        Some(serde_json::json!({ "signed_event": read_json_file(event_json)? })),
                        true,
                        false,
                    )
                    .await
            }
            ProposalsCommand::BatchSign { file } => {
                let value = read_json_file(file)?;
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
        },
        Command::Schedule(args) => {
            let trigger = trigger_parts(args.time, args.queue, false)?;
            client
                .request(
                    Method::POST,
                    "/v1/publish-items/schedule",
                    Some(serde_json::json!({
                        "signed_event": read_json_file(args.event_json)?,
                        "trigger": trigger.name,
                        "publish_time": trigger.publish_time,
                        "queue_id": trigger.queue_id
                    })),
                    true,
                    false,
                )
                .await
        }
        Command::SendNow(args) => {
            client
                .request(
                    Method::POST,
                    "/v1/publish-items/send-now",
                    Some(serde_json::json!({
                        "signed_event": read_json_file(args.event_json)?,
                        "trigger": "SEND_NOW",
                        "publish_time": null,
                        "queue_id": null
                    })),
                    true,
                    false,
                )
                .await
        }
        Command::Posts { command } => match command {
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
        },
        Command::Dvm { command } => match command {
            DvmCommand::Requests => {
                client
                    .request(Method::GET, "/v1/dvm/requests", None, true, true)
                    .await
            }
        },
    }
}

async fn create_proposal(client: &ApiClient, args: ProposeArgs) -> anyhow::Result<Value> {
    let owner = Pubkey::parse(args.to.clone()).context("invalid owner pubkey")?;
    let content = match (args.content, args.file) {
        (Some(content), None) => content,
        (None, Some(path)) => fs::read_to_string(path).context("failed to read proposal file")?,
        (Some(_), Some(_)) => bail!("use either --content or --file, not both"),
        (None, None) => bail!("proposal requires --content or --file"),
    };
    let trigger = trigger_parts(args.time, args.queue, false)?;
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

fn trigger_parts(
    time: Option<DateTime<Utc>>,
    queue: Option<Uuid>,
    send_now: bool,
) -> anyhow::Result<TriggerParts> {
    if send_now {
        return Ok(TriggerParts {
            name: "SEND_NOW",
            publish_time: None,
            queue_id: None,
        });
    }
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

#[derive(Debug)]
struct TriggerParts {
    name: &'static str,
    publish_time: Option<DateTime<Utc>>,
    queue_id: Option<Uuid>,
}

#[derive(Debug)]
struct ApiClient {
    http: reqwest::Client,
    api_url: String,
    session_token: Option<String>,
    owner_pubkey: Option<String>,
}

impl ApiClient {
    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
        auth: bool,
        owner_header: bool,
    ) -> anyhow::Result<Value> {
        let url = format!("{}{}", self.api_url.trim_end_matches('/'), path);
        let mut request = self.http.request(method, url);
        if auth {
            let token = self
                .session_token
                .as_deref()
                .context("not authenticated; run shipyard auth login")?;
            request = request.bearer_auth(token);
        }
        if owner_header {
            if let Some(owner_pubkey) = &self.owner_pubkey {
                request = request.header("x-shipyard-owner-pubkey", owner_pubkey);
            }
        }
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await.context("request failed")?;
        let status = response.status();
        if status == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::json!({ "status": "ok" }));
        }

        let value = response
            .json::<Value>()
            .await
            .context("invalid JSON response")?;
        if !status.is_success() {
            bail!("{}", value);
        }
        Ok(value)
    }

    fn required_owner(&self) -> anyhow::Result<&str> {
        self.owner_pubkey
            .as_deref()
            .context("owner pubkey required; use --owner-pubkey or shipyard accounts use")
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    api_url: Option<String>,
    session_token: Option<String>,
    default_account: Option<String>,
    output: Option<String>,
}

impl Config {
    fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let contents = fs::read_to_string(path).context("failed to read config")?;
        toml::from_str(&contents).context("failed to parse config")
    }

    fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).context("failed to create config directory")?;
        }
        let contents = toml::to_string_pretty(self).context("failed to serialize config")?;
        fs::write(path, contents).context("failed to write config")
    }
}

fn config_path(path: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(path) = path {
        return Ok(path);
    }
    let base = dirs::config_dir().context("could not find config directory")?;
    Ok(base.join("shipyard").join("config.toml"))
}

fn read_json_file(path: PathBuf) -> anyhow::Result<Value> {
    let contents = fs::read_to_string(path).context("failed to read JSON file")?;
    serde_json::from_str(&contents).context("failed to parse JSON file")
}

fn print_output(json_output: bool, output: &Value) {
    if json_output {
        println!(
            "{}",
            serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
        );
    } else if let Some(status) = output.get("status").and_then(Value::as_str) {
        println!("Shipyard CLI: {status}");
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
        );
    }
}
