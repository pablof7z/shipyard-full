# Technical Architecture Spec

## Architecture Goals

The rebuilt Shipyard should be a durable publishing service with Nostr-native clients and protocol surfaces.

Primary goals:

- Preserve user-visible scheduling, queueing, relay publishing, and DVM access.
- Support server-authorized delegated proposal workflows.
- Support owner review/signing across web, mobile, and CLI.
- Use NIP-37 for drafts and Blossom for media.
- Use Rust for backend services and CLI.
- Use the NDK SvelteKit template as the web foundation.
- Run outside Vercel in an environment that supports long-lived services and workers.

## System Components

```text
shipyard-web
shipyard-api
shipyard-worker
shipyard-dvm
shipyard-cli
shipyard-mobile-core
shipyard-ios
shipyard-android
postgres
job queue
object/log/metrics backend
```

## Web Application

Base:

- `nostr-dev-kit/ndk-template-sveltekit-vercel`
- SvelteKit 2
- Svelte 5
- `@nostr-dev-kit/ndk`
- `@nostr-dev-kit/svelte`
- `@nostr-dev-kit/sessions`
- `@nostr-dev-kit/blossom`

Template patterns to keep:

- Separate server-side Nostr fetching from client live subscriptions.
- Use `createNDK` for browser NDK session state.
- Use session storage from `@nostr-dev-kit/sessions`.
- Support extension, private key, and remote signer login UX.
- Use monitored session events for relay, Blossom, and other user lists.
- Keep NDK integration isolated in `src/lib/ndk`.
- Keep route orchestration in `src/routes`.

Template assumptions to replace:

- Replace Vercel adapter with node or the deployment adapter chosen for Shipyard.
- Remove example Highlighter product surfaces.
- Replace publication/news UI with Shipyard publishing cockpit.
- Replace managed NIP-05 onboarding as a core flow unless separately approved.
- Add Shipyard API client modules.

## Rust Backend Services

### `shipyard-api`

Responsibilities:

- HTTP API for web, mobile, and CLI.
- Nostr-authenticated session handling.
- Account and delegate authorization.
- Proposal CRUD.
- Owner review/sign/reject operations.
- Queue CRUD.
- Scheduling operations.
- Relay settings.
- Blossom server discovery helper if clients need server-side support.
- Status and diagnostics.

The API should be stateless except for database and cache/job queue access.

Recommended stack:

- Rust stable.
- Axum or Actix Web.
- SQLx or Diesel.
- PostgreSQL.
- `nostr` Rust ecosystem crates for event validation/signature verification where mature.
- OpenAPI generation or a typed shared API schema.

### `shipyard-worker`

Responsibilities:

- Consume due publish jobs.
- Publish signed events to configured relays.
- Record relay results.
- Retry retryable relay failures.
- Advance state machine.
- Emit operational metrics.

The worker replaces legacy polling with explicit jobs.

### `shipyard-dvm`

Responsibilities:

- Maintain relay subscriptions for kind `5905`.
- Validate DVM requests.
- Store valid signed scheduled events.
- Send legacy-compatible feedback events.
- Record request audit metadata.

The DVM can run as its own Rust process or as a worker mode. It should be isolated enough that DVM load cannot starve the HTTP API.

### Signer Workflow

The backend should not hold owner private keys unless a future explicit custody model is approved.

Signing modes:

- Client-side signing in web/mobile.
- NIP-46 remote signing where the user explicitly connects a signer.
- CLI signing when the CLI has a signer.
- Owner review UI that signs final events before storing scheduled signed events.

The backend stores signed final events and unsigned/proposed candidate events, but it should not silently sign on behalf of users without an explicit signer connection and user-approved flow.

## Database

Use PostgreSQL.

Required tables or equivalent:

- `users`
- `accounts`
- `account_delegates`
- `publish_items`
- `queues`
- `relay_settings`
- `proposal_revisions`
- `publish_attempts`
- `dvm_requests`
- `audit_events`

### `users`

Represents a pubkey that has logged into Shipyard.

Fields:

- `pubkey`
- `created_at`
- `last_seen_at`

### `accounts`

Represents a pubkey Shipyard can manage records for.

Fields:

- `pubkey`
- `created_at`
- `updated_at`

### `account_delegates`

Server-side invite/authorization record.

Fields:

- `owner_pubkey`
- `delegate_pubkey`
- `created_by_pubkey`
- `status`
- `created_at`
- `revoked_at`

Rules:

- Owner can create and revoke.
- Delegate can create proposals while authorization is active.

### `publish_items`

Core state machine table.

Fields:

- `id`
- `owner_pubkey`
- `created_by_pubkey`
- `state`
- `trigger`
- `unsigned_event_json`
- `signed_event_json`
- `event_id`
- `publish_time`
- `queue_id`
- `published_at`
- `published_to`
- `failure_code`
- `failure_message`
- `created_at`
- `updated_at`

### `proposal_revisions`

Stores revisions for audit and review.

Fields:

- `id`
- `publish_item_id`
- `edited_by_pubkey`
- `event_json`
- `reason`
- `created_at`

This is internal audit metadata, not public proposer credit.

### `publish_attempts`

Records relay publishing attempts.

Fields:

- `id`
- `publish_item_id`
- `attempt`
- `relay_url`
- `status`
- `error`
- `created_at`

### `dvm_requests`

Records DVM request processing.

Fields:

- `id`
- `request_event_id`
- `request_pubkey`
- `encrypted`
- `raw_request_event`
- `status`
- `error`
- `created_at`

## Job Queue

Use a durable job queue, not periodic database scanning.

Acceptable implementations:

- PostgreSQL-backed queue with row locks and `SKIP LOCKED`.
- Dedicated queue service if deployment already includes one.

Required job types:

- `publish_event`
- `retry_publish_event`
- `expire_signature_request`
- `process_dvm_request`
- `send_notification`

Due publish jobs should be scheduled when an item enters `SCHEDULED`.

## Event Validation

Validate at service boundaries:

- Pubkey format.
- Event JSON shape.
- Event id matches event contents when signed.
- Signature validity when signed.
- Signed event pubkey equals owner pubkey.
- Publish time and event `created_at` consistency.
- Delegate authorization for proposals.
- Owner authorization for signing/rejection/cancellation.

## Relay Publishing

Publishing flow:

1. Worker loads due `SCHEDULED` item and locks it.
2. Move to `PUBLISHING`.
3. Resolve relay list for owner.
4. Publish signed event to relays.
5. Record relay outcomes.
6. If any relay accepts, move to `PUBLISHED`.
7. If none accept, retry according to policy or move to `FAILED`.

Relay list source:

- Shipyard account relay settings.
- If absent, imported or discovered Nostr write relays can be offered to the user but should not be silently assumed for backend publishing without clear UX.

## Deployment

The system cannot be Vercel-only because it requires:

- DVM relay subscriptions.
- Long-lived worker processes.
- Durable scheduled jobs.
- Backend API service.
- Nostr relay publishing workers.

Deployment should support:

- Containerized services.
- Postgres.
- Persistent job queue.
- Secret management.
- Logs, metrics, traces.
- Zero-downtime deploys for API.
- Graceful worker shutdown.

Examples:

- Fly.io with Postgres.
- Kubernetes.
- Nomad.
- Railway/Render-like platforms if they support workers and long-running websocket processes.

## Observability

Operational observability is required but not a user analytics feature.

Track:

- DVM requests received, scheduled, failed.
- Publish jobs due, completed, failed.
- Relay acceptance/error counts.
- Queue latency.
- Signer errors.
- API error rates.
- Worker lock/retry counts.

Do not expose engagement analytics as product features.

## Security

Requirements:

- Never commit private keys.
- Store service keys in secret manager.
- Hash or encrypt sensitive signer connection material where possible.
- Prefer client-side signing.
- Validate all Nostr events server-side.
- Scope every mutation by authenticated pubkey and active account/delegate permission.
- Log audit events for invites, revocations, proposal edits, owner signatures, rejections, cancellations, and publish failures.
- Use secure cookies or token storage appropriate to each client.

