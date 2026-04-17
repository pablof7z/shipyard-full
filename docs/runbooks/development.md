# Development Runbook

## Bring Up The Stack

```bash
docker compose -f deploy/docker-compose.yml up --build
```

Services:

- API: `http://localhost:8080`
- Web: `http://localhost:3000`
- Postgres: `localhost:5432`

## Validate The Foundation

```bash
cargo fmt --all -- --check
cargo test --workspace
docker compose -f deploy/docker-compose.yml config
```

For the web app:

```bash
cd apps/web
npm install
npm run check
npm run build
```

Web auth configuration must match the API auth verifier:

- `PUBLIC_SHIPYARD_API_URL` points the browser to the API.
- `PUBLIC_SHIPYARD_AUTH_DOMAIN` must match `SHIPYARD_AUTH_DOMAIN`.
- `PUBLIC_SHIPYARD_AUTH_URL` must match `SHIPYARD_AUTH_URL`.

The Settings page supports NIP-07 browser signer login and manual kind `27235`
auth event login. The active owner pubkey is stored in browser local storage and
sent as `x-shipyard-owner-pubkey` for account-scoped routes.

## Operational Rules

- Publish workers must claim jobs with row locks and must record relay attempts.
- DVM service must preserve the external kind `5905` surface.
- API mutations must authorize against database state, not only session claims.
- Draft content must not be stored in Postgres.
- Private keys and signed auth payloads must not be logged.
- Device token registration is per authenticated user; account-scoped tokens must still pass account/delegate authorization.

## Worker Runtime

`shipyard-worker` polls `jobs` for ready work:

- Claims one job at a time with `FOR UPDATE SKIP LOCKED`.
- Supports `publish_event` and `retry_publish_event`.
- Moves publish items through `SCHEDULED -> PUBLISHING -> PUBLISHED`.
- Records every relay outcome in `publish_attempts`.
- Marks items `FAILED` with a clear failure code/message when no relays are configured or all relays fail.
- Publishes real Nostr `["EVENT", event]` frames over relay WebSockets and waits for matching `["OK", event_id, accepted, message]` responses.

## DVM Runtime

`shipyard-dvm` subscribes to configured relays from `SHIPYARD_DVM_RELAYS` and processes stored `dvm_requests` rows with `status = 'received'`:

- Sends a live `["REQ", subscription_id, {"kinds":[5905],"since":...}]` subscription to each relay.
- Inserts received kind `5905` events into `dvm_requests` idempotently.
- Parses legacy kind `5905` clear request tags.
- Reads signed `i` tag input events.
- Creates or updates scheduled `publish_items`.
- Enqueues `publish_event` jobs.
- Marks DVM requests `scheduled` or `error`.
- Signs kind `7000` feedback with `SHIPYARD_DVM_SECRET_KEY`.
- Publishes feedback as real Nostr `["EVENT", feedback]` frames and waits for relay `OK`.
- Decrypts encrypted NIP-90 params with NIP-04 using the requester pubkey and `SHIPYARD_DVM_SECRET_KEY`.
- For encrypted requests, encrypts the feedback tag payload back to the requester and leaves only `encrypted` and `p` tags visible.
