# Implementation Plan

This plan implements the product with a Rust backend architecture.

Backend means all authoritative server-side behavior: persistence, authorization, queue assignment, publish state transitions, relay publishing, DVM processing, device registration, and audit-critical validation. Those components must be Rust. SvelteKit is the browser client and UI runtime. TypeScript may orchestrate browser NDK sessions, client signing, and API calls, but it must not own durable Shipyard backend logic.

Each milestone has narrow deliverables and success/fail criteria. A milestone is not complete until every success criterion passes.

## Milestone 1: Rust Workspace And Service Shells

Deliverables:

- Rust workspace with crates for `shipyard-core`, `shipyard-api`, `shipyard-worker`, `shipyard-dvm`, `shipyard-cli`, and `shipyard-mobile-core`.
- SvelteKit web app scaffold.
- PostgreSQL migration directory.
- Docker Compose and production Dockerfiles for API, worker, DVM, web, and Postgres.
- CI workflows for Rust format, lint, tests, web check/build, and CLI release builds.

Success criteria:

- `cargo fmt --all -- --check` passes.
- `cargo test --workspace` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cd apps/web && npm run check && npm run build` passes.
- `docker compose -f deploy/docker-compose.yml config` renders valid service definitions.

Fail criteria:

- Any durable backend behavior is implemented only in SvelteKit routes or Node services.
- Workspace crates cannot build independently.
- Docker Compose cannot render configuration.

## Milestone 2: Core Domain And Protocol Primitives

Deliverables:

- Strong domain types for pubkeys, publish states, triggers, queues, publish items, accounts, and API errors.
- Publish state transition rules.
- Queue next-slot calculation.
- Nostr event parsing, event ID calculation, signature validation, and signing helpers.
- NIP-04 helper for DVM compatibility where required.
- DVM kind `5905` request parsing and kind `7000` feedback construction.

Success criteria:

- Unit tests cover valid and invalid state transitions.
- Unit tests cover queue slot calculation from start time, current time, and latest assigned slot.
- Unit tests cover event ID/signature behavior and DVM parsing/feedback behavior.
- Invalid core inputs return typed errors, not stringly internal states.

Fail criteria:

- Publish states can be advanced without explicit transition checks.
- Queue slots are computed ad hoc in API or UI code instead of the shared Rust core.
- DVM compatibility parsing lives only in TypeScript.

## Milestone 3: Database Schema And Persistence Contracts

Deliverables:

- PostgreSQL schema for users, accounts, delegates, sessions, queues, relay settings, publish items, proposal revisions, publish attempts, DVM requests, jobs, audit events, and device tokens.
- Constraints for non-empty pubkeys, positive queue cadence, publish state invariants, job attempt bounds, and device platforms.
- Indexes for session lookup, account delegation, publish item state/due queries, queue slots, DVM request lookup, and job claiming.

Success criteria:

- A new Postgres database can initialize from migrations without manual steps.
- Schema prevents impossible scheduled/published items without signed events.
- Schema supports efficient worker job claiming and account-scoped item lists.

Fail criteria:

- Durable drafts are stored as backend rows.
- Publish attempts or DVM requests cannot be audited after processing.
- Device tokens are not scoped to authenticated users.

## Milestone 4: Rust API Auth, Accounts, And Delegation

Deliverables:

- Nostr-authenticated session login with signed kind `27235` auth proof.
- Logout and session inspection.
- Account listing for owner and delegated accounts.
- Delegate invite, list, and revoke endpoints.
- Owner header handling for account-scoped operations.

Success criteria:

- Login recomputes event ID and verifies Schnorr signature.
- Login rejects wrong kind, stale timestamp, wrong domain, wrong method, and wrong URL tag.
- Delegates can see authorized owner accounts.
- Revoked delegates lose account access.

Fail criteria:

- API trusts client-provided pubkeys without verifying session identity.
- Authorization is stored only in browser/local client state.
- Delegates can sign or manage owner-only settings.

## Milestone 5: Rust API Scheduling, Proposals, Queues, Relays, And Devices

Deliverables:

- Queue create/list/update/archive endpoints.
- Queue next-slot endpoint using shared Rust queue logic.
- Proposal create/list/edit/delete/reject/sign/batch-sign endpoints.
- Publish item list/schedule/cancel/retry endpoints.
- Relay get/update endpoints.
- DVM request list endpoint.
- Device token list/register/update/delete endpoints.

Success criteria:

- Account-scoped endpoints require authenticated ownership or active delegation.
- Owner-only mutations reject delegates.
- Proposal signing validates signed event owner, ID, signature presence, and publish time.
- Batch signing returns per-item success or API error without corrupting other items.
- Device token registration validates platform and user ownership.

Fail criteria:

- Queue slot logic differs between API and core tests.
- A failed item in batch signing aborts or mutates unrelated valid items incorrectly.
- Device endpoints allow users to mutate another user's token.

## Milestone 6: Durable Rust Publish Worker

Deliverables:

- Worker process that claims due jobs with row locks and `SKIP LOCKED`.
- Relay publishing over WebSocket using Nostr `EVENT` frames.
- Matching relay `OK` handling.
- Publish attempt recording per relay.
- State transitions from scheduled to publishing to published or failed.
- Retry path for failed items.

Success criteria:

- Concurrent workers do not process the same job.
- Every relay attempt is recorded with status and error detail.
- Items only move to `PUBLISHED` after at least one configured relay accepts the event.
- Items fail with clear codes/messages when no relays are configured or all relays reject/fail.

Fail criteria:

- Worker polls publish items directly without durable jobs.
- Relay failures are swallowed without audit rows.
- Unsigned events can be published.

## Milestone 7: Rust DVM Service

Deliverables:

- Long-lived relay subscriptions for kind `5905`.
- Idempotent storage of received DVM request events.
- Clear and encrypted request parsing.
- Signed input event validation.
- Creation of scheduled publish items and jobs.
- Kind `7000` feedback signing and publishing.
- Encrypted feedback for encrypted requests.

Success criteria:

- Existing kind `5905` clients can schedule without changing request shape.
- Duplicate DVM events do not create duplicate publish items.
- Invalid input produces error feedback and stored request error state.
- DVM service failure cannot starve HTTP API handling.

Fail criteria:

- DVM requires new version tags or HTTP API usage.
- Encrypted request feedback leaks private request tags publicly.
- DVM processing is implemented in the web app.

## Milestone 8: SvelteKit Web Client

Deliverables:

- Dashboard backed by Rust API data.
- Settings for session, active owner, relays, delegates, and browser signer login.
- Write flow for proposals and signed event scheduling.
- Queues page with create, update, archive, and next-slot calculation.
- Proposal review page with sign, reject, delete, and batch sign.
- Scheduled, published, and DVM request views.
- Clear error states for API failures.

Success criteria:

- Web app uses typed API client modules to call Rust API.
- Web signing uses browser signer/NIP-07 or other client signer UX; backend does not receive owner private keys.
- `npm run check` and `npm run build` pass.
- UI does not store durable drafts in Shipyard backend.

Fail criteria:

- SvelteKit implements authoritative queue assignment, authorization, worker behavior, or DVM processing.
- Signing UX asks users to send owner private keys to backend.
- API errors are hidden behind generic UI failure states.

## Milestone 9: Rust CLI And Agent Skill

Deliverables:

- Rust `shipyard` CLI with JSON output for automation.
- Auth login/logout/status.
- Account selection and listing.
- Delegate management.
- Queue list/create/update/archive/next-slot.
- Relay list/set/add/remove.
- Proposal create/list/delete/reject/sign/batch-sign.
- Schedule signed events using the signed event's `created_at`, plus post list/show/cancel/retry.
- DVM request listing.
- `SKILL.md` that instructs agents to use their own pubkey and propose to human owners.

Success criteria:

- CLI commands map to the same Rust API used by web/mobile.
- `shipyard schedule --event-json` derives publish time from the signed event's `created_at`.
- CLI exits non-zero on failure.
- `--json` returns stable machine-readable output.
- Skill safety rules prohibit requesting or storing a human private key.

Fail criteria:

- CLI bypasses backend authorization.
- Agent skill creates a separate product workflow or privileged agent state.
- CLI requires TypeScript runtime services to function.

## Milestone 10: Drafts And Blossom Media

Deliverables:

- Web draft module using NIP-37 draft wraps.
- Mobile draft design and shared helper behavior where practical.
- Draft list, load, save, and delete flows through Nostr relays.
- Blossom server list resolution from kind `10063`.
- Blossom upload flow for web and mobile.
- CLI media upload where signer access is available.

Success criteria:

- Backend stores no durable draft content.
- Draft deletion publishes the NIP-37 blank-content deletion form.
- Blossom fallback is `https://blossom.primal.net` only when no server list exists.
- Upload errors distinguish signer, server selection, upload, and response parsing failures.

Fail criteria:

- Backend `DRAFT` rows are reintroduced.
- Satellite or generic media providers are added.
- Drafts become local-only without Nostr draft wrap sync.

## Milestone 11: Native Mobile Clients And Shared Rust Mobile Core

Deliverables:

- iOS and Android project scaffolds.
- Shared Rust mobile core for domain models, validation, and API client behavior where practical.
- Login/signing design.
- Compose, drafts, Blossom upload, queues, proposals, owner signing, publish status, and device registration.
- Offline-friendly local editing cache that does not replace NIP-37 canonical drafts.

Success criteria:

- Mobile can register/update/delete device tokens through Rust API.
- Mobile can create proposals and review/sign owner proposals.
- Mobile uses NIP-37 for durable drafts.
- Shared Rust code reduces duplicated validation without compromising native UX.

Fail criteria:

- Mobile becomes only a web wrapper.
- Local cache is treated as canonical durable draft storage.
- Mobile implements divergent state transition rules.

## Milestone 12: Notifications And Operational Visibility

Deliverables:

- Notification job creation for pending signatures and publish failures.
- Device-token targeting by user and optional owner account.
- Publish failure and DVM error visibility in API/web/CLI.
- Structured logs with no private keys or signed auth payloads.
- Metrics for job processing, relay attempts, DVM processing, and API errors.

Success criteria:

- Notification jobs are durable and retryable.
- Users can disable or remove device tokens.
- Operational errors can be traced from publish item to job to relay attempt.

Fail criteria:

- Notification dispatch is best-effort only from web clients.
- Logs include private keys, auth proof payloads, or sensitive encrypted request content.
- Publish failures cannot be diagnosed from persisted records.

## Milestone 13: Deployment, Release, And Hardening

Deliverables:

- Production Docker images for Rust services and web.
- Database migration runbook.
- CLI release artifacts and checksums for supported platforms.
- Security review of auth, delegate authorization, signed event validation, and DVM encrypted handling.
- End-to-end smoke tests against local Docker Compose.
- Backup/restore guidance for Postgres.

Success criteria:

- Fresh environment can run API, worker, DVM, web, and Postgres from documented commands.
- CLI release artifacts are reproducible and checksum verified.
- E2E smoke test covers login, relay config, queue creation, proposal, signing, scheduling, worker publish attempt, and DVM request ingestion.
- Security review finds no owner private-key custody path in backend.

Fail criteria:

- Deployment relies on a Node backend for durable behavior.
- Migrations require manual database editing.
- Release artifacts are published without checksums.
- E2E publish and DVM flows are untested.
