# Local Docker Compose E2E Smoke Runbook

This checklist validates the Milestone 13 local path across web, API, worker,
DVM, and Postgres.

## Current Automation Status

There is no fully executable E2E smoke test in the repository today. The current
Compose stack also does not include:

- a local Nostr relay container;
- deterministic owner/delegate signer fixtures;
- a browser automation test that drives NIP-07 signing;
- a CLI command that creates a valid signed owner event without external signer
  setup.

Until those fixtures exist, run the manual checklist below and capture command
output plus screenshots in the release notes.

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
curl -sS http://localhost:8080/v1/auth/login \
  -H 'content-type: application/json' \
  -d @tmp/auth-event.json
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

- Compose config command output.
- API, worker, and DVM log excerpts with secrets redacted.
- Screenshot or JSON output for login, relay config, queue, proposal, scheduled
  item, publish attempt, and DVM request views.
- Database row snapshots for `publish_items`, `jobs`, `publish_attempts`, and
  `dvm_requests`.
