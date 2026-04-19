# Product Spec

## Product Definition

Shipyard is a Nostr publishing cockpit. It lets users write notes, manage publishing queues, schedule events, delegate proposal creation, review and sign pending work, publish to chosen relays, and access the same scheduling service through web, native mobile, CLI, and the existing DVM protocol.

The rebuilt product should feel like one coherent publishing system across all clients:

- Web app for full writing, review, queue, and account management
- Native mobile app for writing, drafts, media, schedule review, and owner signing
- CLI for automation, human scripting, delegated proposals, and agent-assisted use through `SKILL.md`
- DVM endpoint for Nostr-native scheduling access

The product is not an analytics dashboard, growth tool, ad platform, or generalized agent publishing system.

## Implementation Approach

Shipyard uses a Rust backend architecture.

Authoritative server-side product behavior must live in Rust services:

- HTTP API for web, mobile, CLI, and automation clients.
- Durable publish worker.
- DVM relay listener and kind `5905` processor.
- Shared domain/state-machine crate.
- Rust CLI.
- Shared Rust mobile core where it reduces duplicate validation or client behavior.

The SvelteKit web app remains the primary browser client, but it must not become the authoritative backend. SvelteKit can serve UI, hold client-side NDK/session behavior, and call Shipyard APIs. Persistence, authorization, queue assignment, publish state transitions, relay publishing, DVM processing, and audit-critical validation belong to Rust services.

This boundary exists because Shipyard is protocol-heavy and operationally durable:

- Nostr event validation, event IDs, signatures, DVM parsing, encryption helpers, and relay publishing need strict and repeatable behavior.
- Queue scheduling and publish state transitions must reject invalid states at service boundaries.
- Workers and relay listeners are long-lived network services that need predictable runtime behavior.
- The same domain logic should be reusable by API, worker, DVM, CLI, and native mobile bindings where practical.

TypeScript is used for browser UI, NDK integration, client signing UX, and SvelteKit route orchestration. It is not used for Shipyard's durable backend services.

## Core User Jobs

Shipyard must support these user jobs:

- Write a short Nostr note.
- Save a work-in-progress draft using Nostr draft wraps.
- Upload media through Blossom and insert the resulting URL into a note.
- Schedule a signed note for a specific time.
- Put a note into a publishing queue.
- Manage one or more queues with names, descriptions, cadence, and start time.
- See scheduled, pending, failed, cancelled, and published items.
- Publish as the user's own pubkey.
- Publish as an owner account for which the current pubkey has Shipyard authorization.
- Invite another pubkey to propose posts for an owner account.
- Propose posts to an owner account without having the owner's key.
- Review, edit, reject, sign, or batch sign proposed posts as the owner.
- Connect remote signers through NIP-46 where supported.
- Configure relay publishing targets.
- Schedule through the DVM surface used by existing NDK clients.
- Use `shipyard-cli` for the same scheduling and proposal primitives.

## Users And Roles

### Owner

An owner is the pubkey that signs the final event. Owner capabilities:

- Manage own account settings.
- Configure relay targets.
- Configure or use Blossom server list.
- Invite and revoke delegates.
- Review pending proposals for their account.
- Edit proposals before signing.
- Sign one proposal.
- Batch sign multiple proposals.
- Reject proposals.
- Schedule signed events.
- Cancel unpublished scheduled events.

### Delegate

A delegate is a pubkey granted server-side permission to propose events for an owner.

Delegate capabilities:

- Select own pubkey or any authorized owner pubkey as the active publishing identity.
- Compose proposals for the selected owner account.
- Place proposals into a queue or explicit publish time.
- Edit own proposals before owner signing.
- Delete own proposals before owner signing.
- See the status of own proposals.

Delegate limitations:

- Cannot sign as the owner.
- Cannot publish owner events directly.
- Cannot edit or delete proposals created by other delegates unless separately authorized by future product work.
- Is not shown as persistent product-level author/proposer credit after owner signing.

### CLI Or Agent Pubkey

The CLI authenticates as a Nostr pubkey. It can act as a delegate if the target owner has invited that pubkey.

An agent using `SKILL.md` is not a separate product actor. It is just an operator of `shipyard-cli`.

## Writing And Drafts

Drafts are entirely Nostr-based.

Shipyard clients must use NIP-37 draft wraps:

- Draft event kind: `31234`
- Draft content: JSON-stringified draft event encrypted to the signer with NIP-44
- Required tags include `d` and `k`
- Empty draft content signals deletion
- Private content relay preference is discovered through kind `10013` where available

Backend rules:

- The backend must not store durable drafts.
- No backend `DRAFT` rows for unscheduled writing.
- Backend records begin when a user schedules, queues, proposes, or otherwise submits a publishable item to Shipyard.

Client UX requirements:

- Drafts sync through Nostr where the signer and relays allow it.
- Draft list should be visible in web and mobile.
- Draft deletion should publish the draft deletion form from NIP-37, not just clear local storage.
- Local storage can be used only as an ephemeral editing buffer or crash recovery cache.

## Media Uploads

Shipyard uses Blossom only.

Upload flow:

1. Determine the acting user's Blossom server list from kind `10063`.
2. Use the first server in the list as the primary upload target.
3. If no list exists, use `https://blossom.primal.net`.
4. Upload through Blossom-compatible authorization.
5. Insert the returned media URL into the editor.

Client requirements:

- Web should reuse the template's Blossom helpers where possible.
- Mobile must use the same server resolution rules.
- CLI must support media upload if it can access a file and signer.
- The UI must expose enough error detail to tell users whether the failure came from signing, server selection, upload, or response parsing.

Out of scope:

- Satellite upload.
- Generic provider abstraction.
- Media hosting marketplace.

## Scheduling

Scheduling means storing a signed event for future publishing.

User-visible scheduling modes:

- **Time**: store an owner-signed event and publish it at the event's `created_at`.
- **Queue**: assign an event to the next available slot in a named queue, then require the owner to sign the final event with `created_at` equal to that slot.

Rules:

- A scheduled item must eventually contain a valid signed Nostr event with the final `created_at`.
- For direct time scheduling, `publish_time` is derived from the signed event's `created_at`; clients must not send a separate scheduling timestamp that can diverge from the event.
- If an event has no valid owner signature, it cannot publish.
- Queue scheduling may change `created_at` to match the assigned slot.
- Any `created_at` change invalidates an existing signature and moves the item back to signature workflow.
- Publishing immediately is not a Shipyard scheduling workflow. A user or client that wants to publish now should publish directly to relays instead of routing through Shipyard scheduling.
- Unpublished scheduled items can be cancelled.
- Published items cannot be deleted from relays by Shipyard unless a future deletion feature is explicitly added.

## Queues

Queues are account-scoped publishing schedules.

Fields:

- `id`
- `owner_pubkey`
- `name`
- `description`
- `cadence_seconds`
- `start_at`
- `created_at`
- `updated_at`
- optional `archived_at`

Behavior:

- The next slot is the first cadence-aligned time after `max(now, last_unpublished_or_published_queue_slot)`.
- Adding an item to a queue assigns the next slot and sets event `created_at` to that slot.
- Deleting or cancelling an unsigned queue proposal can free the slot.
- Deleting or cancelling a signed scheduled queue item should not silently mutate already signed later events. The product should prefer leaving later signed items unchanged and showing gaps over invalidating signatures unexpectedly.
- If the owner explicitly rebases a queue, affected unsigned/proposed items can be recalculated. Signed events require owner confirmation and re-signing.

## Delegated Proposal Workflow

Delegation is a core product feature.

Invite flow:

1. Owner enters a delegate pubkey in Shipyard.
2. Backend creates a server-side authorization record.
3. Delegate sees owner account in account switcher after logging in.
4. Owner can revoke the authorization at any time.

Proposal flow:

1. Delegate selects owner account as active publishing identity.
2. Delegate writes content and chooses time or queue.
3. Client builds an unsigned Nostr event whose `pubkey` is the owner pubkey.
4. Backend stores the item as `PROPOSED`.
5. Delegate can edit or delete the proposal until owner acts.
6. Owner reviews proposals.
7. Owner may edit content, tags, time, queue, and media URLs.
8. Owner signs the final event.
9. Signed item moves into scheduled/publishing lifecycle.
10. Owner may reject instead of signing.

Attribution:

- Product UI should not show permanent proposer credit after owner signing.
- Internal audit records may track `proposed_by`, timestamps, and changes for security and debugging.

## Owner Review And Signing

Owner review is the approval gate for delegated work.

Owner UI requirements:

- Pending proposal inbox.
- Filters by queue, time, delegate pubkey, and status.
- Inline preview of final Nostr event.
- Edit before signing.
- Reject with optional internal reason.
- Sign one proposal.
- Batch sign selected proposals.
- Clear warnings when signing will publish soon.

Signing requirements:

- Web supports extension, private-key, and NIP-46 signing according to the template capabilities.
- Mobile supports platform-appropriate native signing and remote signing.
- CLI can sign only when it has an authorized signer for the owner pubkey.
- If owner edits a proposal, the owner signs the edited event, not the original proposal.

## DVM Access

The DVM surface must remain compatible.

Legacy-compatible behavior:

- Listen for request kind `5905`.
- Accept one or more `i` tags containing events to schedule.
- Support encrypted requests as currently implemented by NDK scheduling clients.
- Accept relay targets through the existing relays tag behavior.
- Store valid signed input events for future publishing.
- Send job feedback when scheduled.
- Send error feedback on invalid input.

Product constraints:

- Do not require new version tags.
- Do not change the accepted request shape.
- Do not require clients to use Shipyard's HTTP API.
- The DVM should be robust internally, but externally compatible.

## Account And Relay Settings

Account settings are scoped to the active owner pubkey:

- Relay publish list.
- Delegates.
- Signing connection state.
- Blossom server list display and refresh.

Relay behavior:

- Publishing should use the account's configured relay list.
- If no Shipyard-specific relay list exists, clients should offer importing write relays from the user's Nostr relay list.
- Backend publishing must record which relays accepted the event for operational visibility.

## Mobile Product

Mobile is first-class and must support:

- Login/signing.
- Nostr draft wraps.
- Compose/edit.
- Blossom upload from camera, photo library, and files.
- Queue selection and schedule time selection.
- Proposal inbox.
- Owner review and signing.
- Push or local notifications for pending signatures and publish failures.
- Offline-friendly draft editing through NIP-37 plus local editing cache.

## CLI Product

The CLI is a first-class interface.

Core capabilities:

- Authenticate as a Nostr pubkey.
- Select active owner account.
- List authorized accounts.
- Propose an event to a target owner pubkey.
- Schedule a signed event.
- Manage queues.
- List statuses.
- Sign or reject proposals if authenticated as owner.
- Upload media through Blossom.
- Emit JSON for automation.

The CLI must be designed for use by both humans and agents. The agent integration is limited to a `SKILL.md` that installs and uses the CLI.
