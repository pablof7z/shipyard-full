# Shipyard CLI — Agent Reference

This skill lets an agent operate Shipyard — a Nostr post scheduling and publishing platform — through the `shipyard` Rust CLI. Agents use it to propose content to human owners, manage queues, relays, and devices, check publish state, and inspect DVM requests. Agents never touch the owner's private key.

The `shipyard` binary talks to the Shipyard API. All output is JSON when `--json` is passed.

---

## Core Use Cases for Agents

| Goal | What to do |
|---|---|
| Propose a post for a human to review & sign | `shipyard propose` |
| Schedule a post the agent has already signed | `shipyard schedule` |
| Check whether a post was actually published | `shipyard posts show` |
| Manage a timed posting queue | `shipyard queues *` |
| Invite another agent as a delegate | `shipyard delegates invite` |

---

## Quick-Start Workflow

### 1. Install

Download the binary for your platform, verify the SHA-256 from the release manifest, and
place it on `PATH`. See `SKILL.md` for the expected manifest shape.

### 2. Configure

```bash
mkdir -p ~/.config/shipyard
cat > ~/.config/shipyard/config.toml <<EOF
api_url        = "https://api.shipyard.example"
session_token  = "<session-token>"
default_account = "<owner-pubkey-hex>"
output         = "json"
EOF
```

Or use environment variables (useful in CI / agent home `.env`):

```bash
export SHIPYARD_API_URL=https://api.shipyard.example
export SHIPYARD_SESSION_TOKEN=<session-token>
export SHIPYARD_OWNER_PUBKEY=<owner-pubkey-hex>
```

### 3. Verify auth

```bash
shipyard auth status --json
```

Expected success shape:

```json
{ "authenticated": true, "pubkey": "<agent-pubkey>" }
```

---

## Example: Agent Creates a Proposal for a Human Owner

This is the primary agent flow when the agent cannot sign on behalf of the owner.

```bash
# 1. Draft content and show it to the human before submitting
CONTENT="Just shipped: Shipyard now supports batch signing. Thread below 🧵"

# 2. Create the proposal
shipyard propose \
  --to <owner-pubkey-hex> \
  --content "$CONTENT" \
  --time "2026-04-20T14:00:00Z" \
  --json
```

Response includes a `proposal_id`. Share this with the owner so they can review, edit,
and sign from their own client.

```bash
# 3. Check proposal status
shipyard proposals list --owner-pubkey <owner-pubkey-hex> --json
```

```bash
# 4. Wait for owner to sign, then verify publication
shipyard posts show <publish-item-id> --owner-pubkey <owner-pubkey-hex> --json
```

Only report success when the response contains `"state": "PUBLISHED"` **and** a non-empty
`accepted_relays` list. Do not assume publication from queue entry alone.

---

## Key Command Reference

### Auth

```bash
shipyard auth status --json
shipyard auth login --session-token <token> --json
shipyard auth login --event-json ./auth-event.json --json
shipyard auth logout --json
```

### Accounts & Delegates

```bash
shipyard accounts list --json
shipyard accounts use <owner-pubkey> --json

shipyard delegates list   --owner-pubkey <owner> --json
shipyard delegates invite <delegate-pubkey> --owner-pubkey <owner> --json
shipyard delegates revoke <delegate-pubkey> --owner-pubkey <owner> --json
```

### Proposals

```bash
shipyard propose \
  --to <owner> --content "..." --time 2026-04-20T14:00:00Z --json

shipyard proposals list   --owner-pubkey <owner> --json
shipyard proposals delete <id> --json
shipyard proposals reject <id> --reason "Not a fit" --json
shipyard proposals sign   <id> --event-json ./signed-event.json --json
shipyard proposals batch-sign --file ./batch-sign.json --json
```

### Scheduling (agent-signed events)

```bash
# Schedule for the signed event's created_at timestamp
shipyard schedule --event-json ./signed-event.json --json

# Schedule into a queue; created_at must match the assigned queue slot
shipyard schedule --event-json ./signed-event.json --queue <queue-id> --json
```

### Posts

```bash
shipyard posts list   --owner-pubkey <owner> --json
shipyard posts show   <id> --owner-pubkey <owner> --json
shipyard posts cancel <id> --json
shipyard posts retry  <id> --json
```

**A post is not published until `posts show` returns `state: "PUBLISHED"` with accepted relays.**

### Queues

```bash
shipyard queues list --owner-pubkey <owner> --json

shipyard queues create \
  --name "Daily" --cadence 86400 \
  --start 2026-04-18T10:00:00Z \
  --owner-pubkey <owner> --json

shipyard queues next-slot <queue-id> --json
shipyard queues update   <queue-id> --name "Weekly" --cadence 604800 --json
shipyard queues archive  <queue-id> --json
```

### Relays

```bash
shipyard relays list   --owner-pubkey <owner> --json
shipyard relays set    wss://relay.damus.io wss://relay.primal.net --owner-pubkey <owner> --json
shipyard relays add    wss://relay.example.com --owner-pubkey <owner> --json
shipyard relays remove wss://relay.example.com --owner-pubkey <owner> --json
```

### Devices

Device commands manage push notification device tokens for the authenticated session. Pass `--owner <pubkey>` on register or update to associate the token with a specific owner account.

```bash
shipyard devices list --json
shipyard devices register --platform ios --token <device-token> --owner <owner> --enabled true --json
shipyard devices update <device-id> --enabled false --json
shipyard devices update <device-id> --owner <owner> --json
shipyard devices delete <device-id> --json
```

### DVM / Status

The DVM interface lists stored Nostr kind `5905` job requests. DVM scheduling is handled by the long-running DVM service, not by a CLI endpoint.

```bash
shipyard status --json
shipyard dvm requests --owner-pubkey <owner> --json
```

---

## Safety Notes

- **Use your own pubkey.** Authenticate as the agent, not the human owner.
- **Never ask for or store a private key.** The human signs from their own client.
- **Propose, don't sign.** When acting on behalf of an owner, use `propose` so they retain control.
- **Show content first.** Display the post text to the human before calling `propose`.
- **Confirm publication explicitly.** Poll `posts show` until `state: "PUBLISHED"` — don't infer success from scheduling alone.
