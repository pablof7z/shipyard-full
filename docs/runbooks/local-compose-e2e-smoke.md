# Local Docker Compose E2E Smoke Runbook

This checklist validates the Milestone 13 local path across web, API, worker,
DVM, and Postgres.

## Automated Smoke Script

Run the executable smoke script before release candidates and after changes to
auth, queues, proposals, worker publishing, or Compose wiring:

```bash
scripts/local-compose-smoke.sh
```

The script creates an isolated Docker Compose project with non-default host
ports by default:

- Web: `http://127.0.0.1:13000`
- API: `http://127.0.0.1:18080`
- Postgres: `127.0.0.1:15432`

It validates:

- Compose configuration.
- API and web boot.
- deterministic signed kind `27235` auth for a test-only owner key.
- relay settings round-trip.
- queue creation and next-slot lookup.
- queued proposal creation and owner signing into `SCHEDULED`.
- due `send-now` item creation and at least one worker `publish_attempt` row.
- authenticated DVM request API availability and DVM service process startup.

Useful options:

```bash
SHIPYARD_SMOKE_KEEP_STACK=1 scripts/local-compose-smoke.sh
SHIPYARD_SMOKE_API_PORT=28080 SHIPYARD_SMOKE_WEB_PORT=23000 scripts/local-compose-smoke.sh
SHIPYARD_SMOKE_PROJECT=shipyard-smoke-debug scripts/local-compose-smoke.sh
```

The script intentionally uses `ws://127.0.0.1:9` as a local failing relay so the
worker path can be exercised without publishing test notes to public relays. It
therefore asserts durable publish attempts, not successful relay acceptance.

The repository still does not include a local Nostr relay fixture or browser
automation for NIP-07 signing. Until those exist, DVM request ingestion and the
browser signer path remain manual checks.

## Bring Up The Stack

```bash
docker compose -f deploy/docker-compose.yml config
docker compose -f deploy/docker-compose.yml up --build
```

Expected local services:

- Web: `http://localhost:3000`
- API: `http://localhost:8080`
- Postgres: `localhost:5432`
- Worker: background service
- DVM: background service using `SHIPYARD_DVM_RELAYS`

To avoid port conflicts, override host ports:

```bash
SHIPYARD_API_HOST_PORT=18080 \
SHIPYARD_WEB_HOST_PORT=13000 \
SHIPYARD_POSTGRES_HOST_PORT=15432 \
SHIPYARD_AUTH_URL=http://localhost:18080/v1/auth/login \
PUBLIC_SHIPYARD_API_URL=http://localhost:18080 \
PUBLIC_SHIPYARD_AUTH_URL=http://localhost:18080/v1/auth/login \
docker compose -f deploy/docker-compose.yml up --build
```

Health checks:

```bash
curl -i http://localhost:8080/v1/auth/session
docker compose -f deploy/docker-compose.yml ps
docker compose -f deploy/docker-compose.yml logs --tail=100 api worker dvm
```

## Test Identities

Use two test-only Nostr identities:

- Owner pubkey: `<owner-pubkey>`
- Delegate pubkey: `<delegate-pubkey>`

Never use production owner private keys in local smoke tests. The backend must
receive signed auth events and signed publish events only.

## Login

1. Open `http://localhost:3000/settings`.
2. Sign in with a NIP-07 browser signer, or paste a pre-signed kind `27235`
   auth event matching:
   - `kind`: `27235`
   - `domain`: `localhost`
   - `method`: `POST`
   - `u`: `http://localhost:8080/v1/auth/login`
   - timestamp within 3 minutes
3. Confirm the owner account is visible in Settings.

Manual API alternative:

```bash
curl -sS -X POST http://localhost:8080/v1/auth/login \
  -H 'content-type: application/json' \
  -d @tmp/auth-request.json
```

Record the returned `session_token`:

```bash
export SHIPYARD_SESSION='<session-token>'
export SHIPYARD_OWNER='<owner-pubkey>'
```

Success criteria:

- Login returns a session token.
- `GET /v1/auth/session` succeeds with `Authorization: Bearer`.
- No private key or full signed auth payload appears in service logs.

## Relay Config

Use test relays only:

```bash
curl -sS -X PUT http://localhost:8080/v1/relays \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER" \
  -H 'content-type: application/json' \
  -d '{"relay_urls":["wss://relay.damus.io","wss://relay.primal.net"]}'
```

Verify:

```bash
curl -sS http://localhost:8080/v1/relays \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER"
```

Success criteria:

- Relay list round-trips for the active owner.
- Logs do not include credentials or unrelated session data.

## Queue Creation

Create a queue in the web UI or by API:

```bash
curl -sS -X POST http://localhost:8080/v1/queues \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER" \
  -H 'content-type: application/json' \
  -d '{
    "name":"Smoke Queue",
    "description":"Milestone 13 smoke queue",
    "cadence_seconds":3600,
    "start_at":"<future-iso-time>"
  }'
```

Verify next slot:

```bash
curl -sS http://localhost:8080/v1/queues/<queue-id>/next-slot \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER"
```

Success criteria:

- Queue appears in `GET /v1/queues`.
- Next slot is cadence aligned and later than or equal to the queue start.

## Proposal

If testing delegation, invite the delegate from the owner session first, then log
in as the delegate and create the proposal. If testing owner-only flow, create a
proposal as the owner.

```bash
curl -sS -X POST http://localhost:8080/v1/proposals \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER" \
  -H 'content-type: application/json' \
  -d '{
    "owner_pubkey":"<owner-pubkey>",
    "unsigned_event":{
      "pubkey":"<owner-pubkey>",
      "created_at":1776504000,
      "kind":1,
      "tags":[],
      "content":"Shipyard local smoke proposal"
    },
    "trigger":"QUEUE",
    "queue_id":"<queue-id>"
  }'
```

Success criteria:

- Proposal appears in `GET /v1/proposals`.
- Owner can see delegated proposal.
- Revoked or missing delegate authorization is rejected.

## Signing And Scheduling

Sign the proposed event with the owner signer. The signed event must include the
owner pubkey, valid event id, valid Schnorr signature, and `created_at` expected
by queue scheduling.

```bash
curl -sS -X POST http://localhost:8080/v1/proposals/<proposal-id>/sign \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER" \
  -H 'content-type: application/json' \
  -d @tmp/sign-proposal-request.json
```

`tmp/sign-proposal-request.json` must wrap the signed event:

```json
{
  "signed_event": {
    "id": "<event-id>",
    "pubkey": "<owner-pubkey>",
    "created_at": 1776504000,
    "kind": 1,
    "tags": [],
    "content": "Shipyard local smoke proposal",
    "sig": "<signature>"
  }
}
```

Success criteria:

- API returns a scheduled publish item.
- `publish_items.state` is `SCHEDULED`.
- A `publish_event` job exists for the item.

Inspect state:

```bash
docker compose -f deploy/docker-compose.yml exec postgres psql -U shipyard -d shipyard \
  -c "select id,state,trigger,publish_time,event_id from publish_items order by created_at desc limit 5;"
docker compose -f deploy/docker-compose.yml exec postgres psql -U shipyard -d shipyard \
  -c "select kind,state,payload,last_error from jobs order by created_at desc limit 5;"
```

## Worker Publish Attempt

The worker should claim due `publish_event` jobs and write `publish_attempts`.
Use a near-future publish time or `send-now` flow when exercising this step.

```bash
docker compose -f deploy/docker-compose.yml logs -f worker
docker compose -f deploy/docker-compose.yml exec postgres psql -U shipyard -d shipyard \
  -c "select relay_url,status,error,created_at from publish_attempts order by created_at desc limit 10;"
```

Success criteria:

- Worker logs show job claiming without panic.
- Each configured relay gets an attempt row.
- Publish item reaches `PUBLISHED` when at least one relay accepts, or `FAILED`
  with a clear failure code/message when all relays fail.

## DVM Request Ingestion

The DVM service listens to relays from `SHIPYARD_DVM_RELAYS` for kind `5905`
requests. Because Compose does not provide a local relay or DVM request fixture,
this step currently requires an external test relay and a separately signed kind
`5905` request addressed to the configured DVM pubkey.

`scripts/local-compose-smoke.sh` validates that the DVM service starts and that
the authenticated `/v1/dvm/requests` API is available. It does not prove relay
ingestion.

Manual verification:

```bash
docker compose -f deploy/docker-compose.yml logs -f dvm
docker compose -f deploy/docker-compose.yml exec postgres psql -U shipyard -d shipyard \
  -c "select request_event_id,dvm_pubkey,status,failure_code,created_at from dvm_requests order by created_at desc limit 10;"
curl -sS http://localhost:8080/v1/dvm/requests \
  -H "authorization: Bearer $SHIPYARD_SESSION" \
  -H "x-shipyard-owner-pubkey: $SHIPYARD_OWNER"
```

Success criteria:

- Kind `5905` request is inserted idempotently.
- Valid request transitions from `pending` through processing to `succeeded`,
  or to `failed` with a clear failure code/message.
- Duplicate request ingestion does not create duplicate publish items.
- Encrypted requests do not expose decrypted content in logs.

## Release Evidence

Attach the following to the release checklist:

- `scripts/local-compose-smoke.sh` output.
- Compose config command output.
- API, worker, and DVM log excerpts with secrets redacted.
- Screenshot or JSON output for login, relay config, queue, proposal, scheduled
  item, publish attempt, and DVM request views.
- Database row snapshots for `publish_items`, `jobs`, `publish_attempts`, and
  `dvm_requests`.
