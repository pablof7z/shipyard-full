# Shipyard CLI Skill

This skill lets an agent operate Shipyard — a Nostr post scheduling and publishing platform — through the `shipyard` Rust CLI. Agents use it to propose content to human owners, manage queues and relays, check publish state, and submit DVM scheduling requests.

> **Key safety rule:** Never ask for or store a human private key. Agents authenticate with their own pubkey and propose to the human owner pubkey. A post is not published until `state: "PUBLISHED"` appears in the response.

---

## Global Flags

All commands accept these flags:

| Flag | Env var | Purpose |
|---|---|---|
| `--json` | — | Machine-readable output (always use this) |
| `--api-url <url>` | `SHIPYARD_API_URL` | Override API base URL |
| `--session-token <tok>` | `SHIPYARD_SESSION_TOKEN` | Override session token |
| `--owner-pubkey <hex>` | `SHIPYARD_OWNER_PUBKEY` | Override owner pubkey |

---

## Auth

```bash
# Check current auth state
shipyard auth status --json

# Login with an existing session token
shipyard auth login --session-token <token> --json

# Login by submitting a pre-signed auth event (kind 27235)
shipyard auth login --event-json ./auth-event.json --json

# Logout
shipyard auth logout --json
```

---

## Accounts

```bash
# List accounts the session can act on
shipyard accounts list --json

# Set the default owner for subsequent commands
shipyard accounts use <owner-pubkey> --json
```

---

## Delegates

Delegates are pubkeys permitted to act on behalf of an owner.

```bash
shipyard delegates list --owner-pubkey <owner> --json
shipyard delegates invite <delegate-pubkey> --owner-pubkey <owner> --json
shipyard delegates revoke <delegate-pubkey> --owner-pubkey <owner> --json
```

---

## Queues

Queues define a posting cadence. Each queue slot is a time window for a scheduled post.

```bash
# List all queues for an owner
shipyard queues list --owner-pubkey <owner> --json

# Create a queue (cadence in seconds; 86400 = daily)
shipyard queues create \
  --name Daily \
  --cadence 86400 \
  --start 2026-04-18T10:00:00Z \
  --owner-pubkey <owner> --json

# Update a queue
shipyard queues update <queue-id> --name Weekly --cadence 604800 --json

# Get the next available slot time
shipyard queues next-slot <queue-id> --json

# Archive a queue (stops new slots)
shipyard queues archive <queue-id> --json
```

---

## Relays

```bash
# List configured relays
shipyard relays list --owner-pubkey <owner> --json

# Replace all relays
shipyard relays set wss://relay.damus.io wss://relay.primal.net --owner-pubkey <owner> --json

# Add a single relay
shipyard relays add wss://relay.example.com --owner-pubkey <owner> --json

# Remove a relay
shipyard relays remove wss://relay.example.com --owner-pubkey <owner> --json
```

---

## Propose (Agent → Human Owner)

Use `propose` when the agent cannot sign for the owner. The human receives the proposal and signs it.

```bash
# Show the human the content first, then create the proposal
shipyard propose \
  --to <human-owner-pubkey> \
  --content "Today's insight: Nostr is unstoppable." \
  --time 2026-04-18T10:00:00Z \
  --json
```

Returns a proposal ID. Track it with:

```bash
shipyard proposals list --owner-pubkey <owner> --json
```

### Proposal lifecycle management

```bash
# Delete a proposal (before signing)
shipyard proposals delete <id> --json

# Reject with a reason
shipyard proposals reject <id> --reason "Off-brand" --json

# Submit a signed event for a proposal (owner signs externally)
shipyard proposals sign <id> --event-json ./signed-event.json --json

# Batch-sign multiple proposals
shipyard proposals batch-sign --file ./batch-sign.json --json
```

---

## Schedule & Send

Use when the agent already has a signed Nostr event JSON.

```bash
# Schedule for a specific time
shipyard schedule --event-json ./signed-event.json --time 2026-04-18T10:00:00Z --json

# Publish immediately
shipyard send-now --event-json ./signed-event.json --json
```

---

## Posts (Publish Items)

```bash
# List scheduled/published posts for an owner
shipyard posts list --owner-pubkey <owner> --json

# Inspect a specific post — check for state: "PUBLISHED" before reporting success
shipyard posts show <id> --owner-pubkey <owner> --json

# Cancel a pending post
shipyard posts cancel <id> --json

# Retry a failed post
shipyard posts retry <id> --json
```

**A post is not published until `posts show` returns `state: "PUBLISHED"` with accepted relays.**

---

## DVM

The DVM interface allows scheduling via Nostr kind `5905` job requests.

```bash
# List incoming DVM scheduling requests for an owner
shipyard dvm requests --owner-pubkey <owner> --json
```

---

## Server Status

```bash
shipyard status --json
```

Returns API version and health info.
