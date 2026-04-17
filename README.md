# Shipyard

Shipyard is a Nostr publishing cockpit for drafts, media, queues, delegated proposals, owner signing, scheduled publishing, CLI automation, mobile clients, and legacy-compatible DVM scheduling.

This repository is the clean reimplementation described in `docs/`.

## Workspace

- `crates/shipyard-core`: shared domain types, state machine, validation, queue logic.
- `crates/shipyard-api`: Rust HTTP API for web, mobile, and CLI.
- `crates/shipyard-worker`: durable publish worker.
- `crates/shipyard-dvm`: long-lived kind `5905` DVM service.
- `crates/shipyard-cli`: Rust CLI binary named `shipyard`.
- `crates/shipyard-mobile-core`: shared Rust library for iOS and Android.
- `apps/web`: SvelteKit cockpit using the Shipyard design system.
- `migrations`: PostgreSQL schema and durable job queue.
- `mobile`: native iOS and Android app scaffolds.
- `skills/shipyard-cli`: agent skill package for installing and using the CLI.
- `deploy`: Docker Compose and image definitions.

## Local Development

```bash
docker compose -f deploy/docker-compose.yml up --build
```

Useful checks:

```bash
cargo fmt --all -- --check
cargo test --workspace
cd apps/web && npm install && npm run check && npm run build
```

The web app reads `PUBLIC_SHIPYARD_API_URL`, `PUBLIC_SHIPYARD_AUTH_DOMAIN`, and
`PUBLIC_SHIPYARD_AUTH_URL`. In Settings, users can sign in with a NIP-07 browser
signer or paste a pre-signed kind `27235` auth event, then choose the active
owner account for queues, relays, proposals, and publish history.

## Product Constraints

- Drafts are NIP-37 draft wraps, not backend records.
- Media uploads are Blossom only.
- Delegated proposal credit is audit-only and not public product attribution.
- DVM kind `5905` request and feedback compatibility is preserved.
- Backend services do not hold owner private keys.
