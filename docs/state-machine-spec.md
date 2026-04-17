# State Machine Spec

## Goals

The legacy app used a small state model: `DRAFT`, `NEEDS_SIGNATURE`, `SCHEDULED`, and `PUBLISHED`. The new system needs a fuller internal workflow because delegated proposals, owner review, retries, cancellations, and failures are core product behavior.

The user interface can group states into simple labels, but the backend must store precise states.

## Entities

The state machine applies to backend publish records. Nostr draft wraps are not backend records and are outside this state machine.

Primary entity:

```text
PublishItem
```

Important fields:

- `id`
- `owner_pubkey`
- `created_by_pubkey`
- `state`
- `raw_event`
- `unsigned_event`
- `publish_time`
- `queue_id`
- `trigger`
- `signed_event_id`
- `published_at`
- `published_to`
- `failure_code`
- `failure_message`
- `created_at`
- `updated_at`

`created_by_pubkey` is audit metadata. It must not create persistent product-level author/proposer credit after owner signing.

## States

### `PROPOSED`

An authorized delegate, CLI pubkey, or owner-created workflow has submitted an event candidate for an owner account, but the owner has not signed it.

Properties:

- May have `unsigned_event`.
- Must have `owner_pubkey`.
- Must have `created_by_pubkey`.
- May have `publish_time` or `queue_id`.
- Must not be published.

Allowed actions:

- Delegate who created it can edit.
- Delegate who created it can delete/cancel.
- Owner can edit.
- Owner can reject.
- Owner can sign.

### `REJECTED`

Owner rejected a proposal.

Properties:

- Must keep enough audit data to explain what was rejected.
- Must not publish.
- Should be hidden from normal scheduled views by default.

Allowed actions:

- Owner can archive or delete record according to retention policy.
- Delegate cannot restore.

### `NEEDS_SIGNATURE`

The item is intended for scheduling or publishing, but the final event needs an owner signature.

Common causes:

- Owner edited a proposal but has not signed.
- Queue slot changed and invalidated an existing signature.
- Backend imported an unsigned event from a UI flow.

Allowed actions:

- Owner can sign.
- Owner can edit.
- Owner can cancel.

### `SIGNED`

The final event is signed by `owner_pubkey`, but the item has not yet been admitted to the scheduler.

This can be a short-lived internal state used after signing and before queue/time validation completes.

Allowed actions:

- Backend validates event id, signature, kind, tags, publish time, and account authorization.
- Backend moves to `SCHEDULED` or `PUBLISHING`.
- Owner can cancel before publishing starts.

### `SCHEDULED`

A signed event is waiting for `publish_time`.

Properties:

- Must contain a valid signed Nostr event.
- `raw_event.pubkey` must equal `owner_pubkey`.
- `raw_event.created_at` must match the intended publish time unless the event kind explicitly requires different semantics.
- Must have `publish_time`.

Allowed actions:

- Owner can cancel.
- Owner can edit only by creating a new unsigned/signature-required revision.
- Scheduler moves to `PUBLISHING` when due.

### `PUBLISHING`

The worker is actively publishing the event to relays.

Allowed actions:

- Workers can record partial relay results.
- Workers can move to `PUBLISHED` if at least one configured relay accepts the event.
- Workers can move to `FAILED` after retry policy is exhausted.

User actions:

- UI should not allow destructive edits while publishing.
- Owner may request cancellation, but cancellation is best effort if relay publish has already started.

### `PUBLISHED`

At least one target relay accepted the event.

Properties:

- Must have `published_at`.
- Must record `published_to`.
- Must preserve final signed raw event.

Allowed actions:

- No delete from Shipyard's publish history by default.
- Future deletion support would require explicit Nostr deletion event behavior and is out of scope for this spec.

### `FAILED`

Shipyard could not complete signing, scheduling, or publishing.

Required fields:

- `failure_code`
- `failure_message`
- `failed_at`
- Retry metadata if applicable

Examples:

- `relay_publish_failed`
- `invalid_signature`
- `signer_timeout`
- `no_relays_configured`
- `queue_slot_invalid`
- `dvm_request_invalid`

Allowed actions:

- Owner can retry if the failure is retryable.
- Owner can edit and re-sign if the event is invalid.
- Owner can cancel.

### `CANCELLED`

An unpublished item was cancelled.

Allowed cancellations:

- Owner can cancel any unpublished item for their account.
- Delegate can cancel own `PROPOSED` item before owner action.

Not allowed:

- Cancelling a `PUBLISHED` item does not remove the event from relays and should not be represented as cancellation.

## Transitions

```text
PROPOSED -> REJECTED
PROPOSED -> NEEDS_SIGNATURE
PROPOSED -> SIGNED
PROPOSED -> CANCELLED

NEEDS_SIGNATURE -> SIGNED
NEEDS_SIGNATURE -> CANCELLED

SIGNED -> SCHEDULED
SIGNED -> PUBLISHING
SIGNED -> FAILED
SIGNED -> CANCELLED

SCHEDULED -> PUBLISHING
SCHEDULED -> NEEDS_SIGNATURE
SCHEDULED -> CANCELLED

PUBLISHING -> PUBLISHED
PUBLISHING -> FAILED

FAILED -> NEEDS_SIGNATURE
FAILED -> SCHEDULED
FAILED -> PUBLISHING
FAILED -> CANCELLED
```

## Actor Permissions

Owner:

- Can view all states for their account.
- Can edit proposals and signature-needed items.
- Can reject proposals.
- Can sign proposals and signature-needed items.
- Can cancel unpublished items.
- Can retry failed items.

Delegate:

- Can view own proposals for owner accounts where authorization still exists.
- Can edit own proposals while state is `PROPOSED`.
- Can cancel own proposals while state is `PROPOSED`.
- Cannot sign as owner.
- Cannot mutate items after owner signs, rejects, or edits into owner-controlled workflow.

Backend worker:

- Can advance scheduler and publishing states.
- Can record failures and relay outcomes.
- Cannot invent user approvals.

DVM service:

- Can create scheduled records from valid DVM requests.
- Must not change the external DVM request surface.

## UI Grouping

The UI may group internal states:

- Drafts: NIP-37 draft wraps, not backend rows
- Pending review: `PROPOSED`
- Needs signature: `NEEDS_SIGNATURE`
- Scheduled: `SIGNED`, `SCHEDULED`
- Publishing: `PUBLISHING`
- Published: `PUBLISHED`
- Attention needed: `FAILED`
- Done/hidden: `REJECTED`, `CANCELLED`

## Queue Recalculation Rules

Queue changes must not surprise users by invalidating signatures silently.

Rules:

- Recalculate slots freely for unsigned `PROPOSED` and `NEEDS_SIGNATURE` items.
- Do not automatically change `created_at` for signed `SCHEDULED` items.
- If a signed queue item needs a new slot, move it to `NEEDS_SIGNATURE` only after explicit owner confirmation.
- Deleting a proposed queue item frees its slot for later unsigned items.
- Cancelling a signed scheduled item can leave a gap.

