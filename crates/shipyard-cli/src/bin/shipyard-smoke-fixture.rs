use anyhow::{bail, Context};
use shipyard_core::{pubkey_from_secret_hex, NostrEvent};

fn main() -> anyhow::Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    match args.first().map(String::as_str) {
        Some("pubkey") => print_pubkey(&args[1..]),
        Some("auth") => print_auth_event(&args[1..]),
        Some("note") => print_note_event(&args[1..]),
        Some(command) => bail!("unknown command: {command}"),
        None => bail!("usage: shipyard-smoke-fixture <pubkey|auth|note> ..."),
    }
}

fn print_pubkey(args: &[String]) -> anyhow::Result<()> {
    if args.len() != 1 {
        bail!("usage: shipyard-smoke-fixture pubkey <secret-hex>");
    }
    let pubkey = pubkey_from_secret_hex(&args[0]).context("invalid secret hex")?;
    println!("{}", pubkey.as_str());
    Ok(())
}

fn print_auth_event(args: &[String]) -> anyhow::Result<()> {
    if args.len() != 4 {
        bail!("usage: shipyard-smoke-fixture auth <secret-hex> <created-at> <domain> <url>");
    }

    let secret_hex = &args[0];
    let created_at = parse_timestamp(&args[1])?;
    let domain = args[2].clone();
    let url = args[3].clone();
    let tags = vec![
        vec!["domain".to_string(), domain],
        vec!["method".to_string(), "POST".to_string()],
        vec!["u".to_string(), url],
    ];

    print_signed_event(secret_hex, created_at, 27_235, tags, String::new())
}

fn print_note_event(args: &[String]) -> anyhow::Result<()> {
    if args.len() != 3 {
        bail!("usage: shipyard-smoke-fixture note <secret-hex> <created-at> <content>");
    }

    let secret_hex = &args[0];
    let created_at = parse_timestamp(&args[1])?;
    print_signed_event(secret_hex, created_at, 1, Vec::new(), args[2].clone())
}

fn print_signed_event(
    secret_hex: &str,
    created_at: i64,
    kind: u64,
    tags: Vec<Vec<String>>,
    content: String,
) -> anyhow::Result<()> {
    let pubkey = pubkey_from_secret_hex(secret_hex).context("invalid secret hex")?;
    let mut event = NostrEvent::unsigned(pubkey, created_at, kind, tags, content);
    event
        .sign_with_secret_hex(secret_hex)
        .context("failed to sign event")?;
    println!("{}", serde_json::to_string(&event)?);
    Ok(())
}

fn parse_timestamp(value: &str) -> anyhow::Result<i64> {
    value
        .parse::<i64>()
        .with_context(|| format!("invalid unix timestamp: {value}"))
}
