---
name: shipyard-cli
description: Use when an agent needs to install, configure, or operate Shipyard through the Rust shipyard CLI, especially to propose Nostr posts to a human owner, inspect proposal status, schedule signed posts, or work with queues, relays, media, and DVM-compatible scheduling. Agents must use their own pubkey and must not ask for or store a human private key.
---

# Shipyard CLI

Use this skill to operate Shipyard through `shipyard-cli`.

## Safety Rules

- Do not ask for a human private key.
- Authenticate with the agent's own pubkey unless the user explicitly provides a separate signer workflow.
- Propose to the human owner pubkey when you cannot sign for that owner.
- Do not claim a post is published until status shows owner signing and worker publish success.
- Use `--json` for machine-readable commands.
- Show the human the content before creating a proposal.

## Install

1. Fetch the release manifest from the configured Shipyard release source.
2. Select the artifact matching the current OS and architecture.
3. Verify the artifact SHA-256 from the manifest.
4. Install the `shipyard` binary into a directory on `PATH`.

Expected manifest shape:

```json
{
  "version": "x.y.z",
  "artifacts": [
    {
      "target": "aarch64-apple-darwin",
      "url": "https://...",
      "sha256": "..."
    }
  ]
}
```

## Configure

Default config path:

```text
~/.config/shipyard/config.toml
```

Minimum config:

```toml
api_url = "https://api.shipyard.example"
session_token = "<session token>"
default_account = "<owner pubkey>"
output = "json"
```

The CLI also accepts environment overrides:

```bash
SHIPYARD_API_URL=https://api.shipyard.example
SHIPYARD_SESSION_TOKEN=<session-token>
SHIPYARD_OWNER_PUBKEY=<owner-pubkey>
```

## Core Workflow For Agents

Check auth:

```bash
shipyard auth status --json
```

Store a session token if one was created through another signer flow:

```bash
shipyard auth login --session-token <session-token> --json
```

Or submit a signed auth event JSON file to the API:

```bash
shipyard auth login --event-json ./auth-event.json --json
```

List available accounts:

```bash
shipyard accounts list --json
```

Create a proposal to a human owner:

```bash
shipyard propose --to <human-owner-pubkey> --content "<post text>" --time "2026-04-18T10:00:00Z" --json
```

Check proposal status:

```bash
shipyard proposals list --json
```

Do not report publication until:

```bash
shipyard posts show <publish-item-id> --owner-pubkey <owner> --json
```

returns `state: "PUBLISHED"` and includes accepted relays.

## Implemented Commands

```bash
shipyard status --json
shipyard auth login --session-token <token> --json
shipyard auth login --event-json <path> --json
shipyard auth logout --json
shipyard auth status --json
shipyard accounts list --json
shipyard accounts use <owner-pubkey> --json
shipyard delegates list --owner-pubkey <owner> --json
shipyard delegates invite <delegate-pubkey> --owner-pubkey <owner> --json
shipyard delegates revoke <delegate-pubkey> --owner-pubkey <owner> --json
shipyard queues list --owner-pubkey <owner> --json
shipyard queues create --name Daily --cadence 86400 --start 2026-04-18T10:00:00Z --owner-pubkey <owner> --json
shipyard queues update <queue-id> --name Daily --cadence 86400 --json
shipyard queues next-slot <queue-id> --json
shipyard queues archive <queue-id> --json
shipyard relays list --owner-pubkey <owner> --json
shipyard relays set wss://relay.damus.io wss://relay.primal.net --owner-pubkey <owner> --json
shipyard relays add wss://relay.example.com --owner-pubkey <owner> --json
shipyard relays remove wss://relay.example.com --owner-pubkey <owner> --json
shipyard propose --to <owner> --content "..." --time 2026-04-18T10:00:00Z --json
shipyard proposals list --owner-pubkey <owner> --json
shipyard proposals delete <id> --json
shipyard proposals reject <id> --reason "Not a fit" --json
shipyard proposals sign <id> --event-json ./signed-event.json --json
shipyard proposals batch-sign --file ./batch-sign.json --json
shipyard schedule --event-json ./signed-event.json --time 2026-04-18T10:00:00Z --json
shipyard send-now --event-json ./signed-event.json --json
shipyard posts list --owner-pubkey <owner> --json
shipyard posts show <id> --owner-pubkey <owner> --json
shipyard posts cancel <id> --json
shipyard posts retry <id> --json
shipyard dvm requests --owner-pubkey <owner> --json
```
