# Shipyard — Agent Conventions

## Architecture Mandate (from `docs/implementation-plan.md`)

The backend is **Rust only**. All authoritative server-side behavior belongs in Rust crates:
persistence, authorization, queue assignment, publish state transitions, relay publishing,
DVM processing, device registration, and audit-critical validation.

SvelteKit/TypeScript is the UI layer only. TypeScript may orchestrate browser NDK sessions,
client signing, and API calls — but it must not own durable Shipyard backend logic.

## File Size Rules

- **No file may exceed 500 LOC.**
- **Ideally, no file exceeds 300 LOC.**
- When a file approaches 300 LOC, begin planning how to split it.
- Breaking up files takes priority over adding new features.

## Module Boundaries (Rust)

Each crate has a clear domain:

| Crate | Responsibility |
|---|---|
| `shipyard-core` | Domain types, state machine, queue logic, Nostr primitives, DVM parsing |
| `shipyard-api` | HTTP API endpoints and request handling |
| `shipyard-worker` | Durable publish worker with job claiming |
| `shipyard-dvm` | Long-lived kind `5905` DVM service |
| `shipyard-cli` | CLI binary — thin wrapper over API |
| `shipyard-mobile-core` | Shared Rust lib for mobile clients |

## General Rules

- All Rust code: `cargo fmt`, `cargo clippy -D warnings`, `cargo test`
- All web code: `bun run check && bun run build`
- Docker Compose must validate: `docker-compose config`
- No backend behavior in TypeScript/SvelteKit
- No owner private keys ever touch the backend
- After every significant modification to a service, immediately redeploy or restart that service.
- Every milestone has explicit success/fail criteria — a milestone is not done until all pass
