# Shipyard Reimplementation Specs

This directory is the product and technical specification package for rebuilding Shipyard from scratch.

The goal is not to port the legacy application line by line. The goal is to preserve the product concept and protocol compatibility that matter to users while rebuilding the service with a modern architecture:

- SvelteKit web application based on `nostr-dev-kit/ndk-template-sveltekit-vercel`
- Rust backend services and workers
- Rust `shipyard-cli`
- Native iOS and Android clients with shared Rust domain/client code where practical
- DVM access compatible with the existing kind `5905` scheduling surface
- Blossom-only media uploads
- Nostr draft wraps for drafts
- Server-authorized delegated proposal and owner signing workflows

## Source Snapshot

Legacy source inspected:

- Repository: `shipyard-previous`
- Commit: `8e830d072a653b610dc12f95c35be20376be5a91`
- Important areas:
  - `apps/web`: SvelteKit web app and API routes
  - `apps/backend`: polling publisher, NIP-46 signer, DVM listener
  - `shipyard`: shared Prisma schema and data helpers

Template source inspected:

- Repository: `https://github.com/nostr-dev-kit/ndk-template-sveltekit-vercel`
- Commit inspected in `/tmp`: `907086a15a7ab8ee9b862552b2570d333be76009`
- Important areas:
  - `src/lib/ndk/client.ts`: browser NDK setup with session persistence and monitored NDK lists
  - `src/lib/features/auth`: extension, private-key, and remote signer login UX
  - `src/lib/onboarding.ts`: Blossom server helpers and default `https://blossom.primal.net`
  - `src/lib/server/nostr.ts`: server-side Nostr fetching pattern for SSR
  - `svelte.config.js`: Vercel adapter in the template, to be replaced for Shipyard

Protocol references used:

- [NIP-37 Draft Wraps](https://github.com/nostr-protocol/nips/blob/master/37.md): `kind:31234` encrypted draft events and private content relay list `kind:10013`
- [NIP-90 DVM](https://github.com/nostr-protocol/nips/blob/master/90.md): request kinds `5000-5999`, result kinds `6000-6999`, feedback kind `7000`
- [Data vending machine kind `5905`](https://github.com/nostr-protocol/data-vending-machines/blob/master/kinds/5905.md): Nostr Event Publish Schedule
- [NIP-B7 Blossom media](https://nips.nostr.com/B7) and [Blossom BUD-03](https://github.com/hzrd149/blossom/blob/master/buds/03.md): Blossom media and user server list `kind:10063`

## Reading Order

1. [Product Spec](./product-spec.md)
2. [State Machine Spec](./state-machine-spec.md)
3. [Protocols and APIs](./protocols-and-apis.md)
4. [Technical Architecture Spec](./technical-architecture-spec.md)
5. [Implementation Plan](./implementation-plan.md)
6. [Template Integration Spec](./template-integration-spec.md)
7. [Mobile Spec](./mobile-spec.md)
8. [Shipyard CLI and Skill Spec](./shipyard-cli-and-skill-spec.md)
9. [Legacy System Analysis](./legacy-system-analysis.md)
10. [Legacy Technical Spec](./legacy-technical-spec.md)
11. [Out of Scope and Future](./out-of-scope-and-future.md)

## Runbooks

- [Development](./runbooks/development.md)
- [Postgres Migration, Backup, And Restore](./runbooks/postgres-migration-backup-restore.md)
- [Local Docker Compose E2E Smoke](./runbooks/local-compose-e2e-smoke.md)
- [Security Review Checklist](./runbooks/security-review-checklist.md)

## Locked Product Decisions

These decisions are requirements for the reimplementation:

- Preserve the DVM scheduling surface. The service must keep compatibility with the existing kind `5905` request and feedback behavior.
- Rust is the backend implementation approach. The API, worker, DVM service, shared domain logic, CLI, and mobile core are Rust. SvelteKit is the browser client and must not own durable backend behavior.
- Do not add analytics or performance feedback as a product feature.
- Use Blossom only for media uploads.
- Resolve Blossom upload servers from the user's Blossom server list. If none exists, default to `https://blossom.primal.net`.
- Use Nostr draft wraps for drafts. Shipyard's backend must not store durable drafts.
- Support delegated publishing through server-side invites. `pubkey1` can invite `pubkey2` to write for `pubkey1`.
- A delegate can select an authorized owner account, write as that account, and create unsigned proposals.
- Delegates can edit and delete their own proposals until the owner signs or rejects them.
- The owner can review, edit, reject, sign individually, or batch sign pending proposals.
- Do not show persistent product-level proposer or author credit after owner signing. Internal audit metadata is allowed.
- The CLI must support proposing events from its own pubkey to a target human pubkey for human review and signing.
- Agent support is limited to `SKILL.md` packaging/installing/using the Rust CLI. No extra agent workflow product layer.
- Use full workflow states internally instead of the legacy four-state model.
- The NDK SvelteKit template is a web-app foundation, not a deployment target. Shipyard production must run on infrastructure that supports backend services, workers, and long-lived relay connections.

## Glossary

- **Owner**: The pubkey that will sign and publish the final Nostr event.
- **Delegate**: A pubkey authorized by the owner in Shipyard's backend to propose posts for the owner.
- **Proposal**: An unsigned or not-owner-signed candidate event created for an owner account by a delegate, CLI, or other authorized pubkey.
- **Draft**: A client-side Nostr draft wrap, not a backend record.
- **Scheduled item**: A signed event stored in Shipyard's backend with a publish time or queue slot.
- **DVM**: Data vending machine. For Shipyard, the key external surface is kind `5905`, Nostr Event Publish Schedule.
- **Blossom server list**: A replaceable `kind:10063` event listing media servers for a user.
