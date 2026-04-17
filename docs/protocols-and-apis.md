# Protocols And APIs

This document defines the externally visible contracts for the rebuilt service. Exact JSON field names can evolve during implementation, but the behavior and boundaries here are requirements.

## Authentication

Clients authenticate as Nostr pubkeys.

Supported clients:

- Web.
- Mobile.
- CLI.

Recommended backend auth:

- Signed Nostr event proof for login/session creation.
- Server verifies signature, pubkey, timestamp, domain or URL/method binding.
- Server issues a Shipyard session scoped to the signing pubkey.

Security requirements:

- Short validity window for login proofs.
- Session revocation support.
- Secure storage per client.
- Backend must reload authorization from database for account/delegate actions.

## Account API

### List Authorized Accounts

Returns accounts the current pubkey can act within.

Response shape:

```json
{
  "user_pubkey": "hex",
  "accounts": [
    {
      "owner_pubkey": "hex",
      "relationship": "owner|delegate",
      "can_propose": true,
      "can_sign": false
    }
  ]
}
```

Rules:

- The user's own pubkey is always present as owner.
- Delegated accounts are present only while active authorization exists.

### Invite Delegate

Owner creates server-side authorization.

Request:

```json
{
  "delegate_pubkey": "hex"
}
```

Rules:

- Caller must authenticate as `owner_pubkey`.
- Delegate pubkey must be valid.
- Duplicate active invites are idempotent.

### Revoke Delegate

Owner revokes authorization.

Rules:

- Existing unsigned proposals remain visible to owner.
- Delegate loses ability to create new proposals.
- Delegate should lose edit/delete permission for existing proposals once revoked unless implementation explicitly allows a grace period. Default: no grace period.

## Proposal API

### Create Proposal

Creates an unsigned candidate event for an owner.

Request:

```json
{
  "owner_pubkey": "hex",
  "unsigned_event": {},
  "trigger": "TIME|QUEUE",
  "publish_time": "ISO-8601 or null",
  "queue_id": "uuid or null"
}
```

Rules:

- Caller must be owner or active delegate.
- `unsigned_event.pubkey` must equal `owner_pubkey`.
- If trigger is `TIME`, publish time is required.
- If trigger is `QUEUE`, queue id is required and must belong to owner.
- Server stores state `PROPOSED`.

### Edit Own Proposal

Delegate can edit own `PROPOSED` items.

Editable fields:

- unsigned event content/tags.
- trigger.
- publish time.
- queue.

Rules:

- Only while state is `PROPOSED`.
- Only creator delegate or owner.
- Store a proposal revision.

### Delete Own Proposal

Delegate can cancel own `PROPOSED` item.

Result:

- State becomes `CANCELLED`, or record is soft-deleted with audit retained.

### Owner Review

Owner lists pending proposals for own account.

Filters:

- state.
- delegate pubkey.
- queue.
- publish time range.

### Owner Reject

Request:

```json
{
  "reason": "optional internal reason"
}
```

Result:

- State `REJECTED`.

### Owner Sign

The API should support two patterns:

1. Client signs final event and submits it.
2. Backend creates signing challenge/payload and client returns signed event.

Request:

```json
{
  "signed_event": {}
}
```

Rules:

- Caller must be owner.
- Signature must be valid.
- Signed event pubkey must equal owner pubkey.
- Signed event must correspond to the reviewed final content/time.
- On success, state becomes `SIGNED` then `SCHEDULED` or `PUBLISHING`.

### Batch Sign

Batch endpoint accepts multiple signed events for selected proposal ids.

Rules:

- Partial failure must be explicit per item.
- A bad signature for one item must not corrupt other items.

## Post/Schedule API

### Schedule Signed Event

For owner-authenticated direct scheduling.

Request:

```json
{
  "signed_event": {},
  "trigger": "TIME|QUEUE",
  "publish_time": "ISO-8601 or null",
  "queue_id": "uuid or null"
}
```

Rules:

- Signed event pubkey must equal authenticated owner or selected owner where caller can sign.
- Event signature must validate.
- Queue items may require event `created_at` to match assigned queue slot.

### List Publish Items

Returns backend items, not NIP drafts.

Filters:

- owner pubkey.
- state.
- queue.
- time range.

### Cancel Item

Allowed for unpublished items.

Rules:

- Owner can cancel any unpublished item.
- Delegate can cancel own `PROPOSED` item only.

### Retry Failed

Owner can retry failed items.

Rules:

- If failure is signing-related, item goes to `NEEDS_SIGNATURE`.
- If failure is relay-related and signed event remains valid, item can return to `SCHEDULED` or `PUBLISHING`.

## Queue API

### Create Queue

```json
{
  "name": "Daily",
  "description": "Optional",
  "cadence_seconds": 86400,
  "start_at": "ISO-8601"
}
```

Rules:

- Owner scoped.
- Cadence must be positive.
- Start time must be valid.

### Update Queue

Editable:

- name.
- description.
- cadence.
- start time.
- archived status.

Changing cadence/start time should not silently mutate signed scheduled events.

### List Queues

Return owner queues plus next slot preview.

### Delete/Archive Queue

Preferred behavior:

- Archive queues rather than hard delete if items exist.

## Relay Settings API

Fields:

- owner pubkey.
- publish relay URLs.

Rules:

- Validate URLs use `wss://` or `ws://` if local development allows `ws://`.
- Backend uses this list for publishing.
- If absent, UI should guide user to import from Nostr relay list.

## Blossom API/Client Contract

Most Blossom operations can be client-side through NDK.

Rules:

- Read `kind:10063`.
- Use first `server` tag as primary upload server.
- Fallback to `https://blossom.primal.net`.
- Upload with Blossom auth.
- Insert returned URL.

Server may expose a helper endpoint for diagnostics or server-side validation, but media bytes should not be proxied by Shipyard unless required by mobile platform constraints.

## NIP-37 Draft Contract

Drafts are Nostr events, not backend objects.

Client requirements:

- Draft kind `31234`.
- Encrypted content contains JSON draft event.
- `k` tag identifies target event kind.
- `d` tag identifies draft.
- Empty content means deletion.
- Publish to private content relays from kind `10013` when available.

## DVM Contract

The DVM surface remains compatible with legacy scheduling.

Required:

- Subscribe to kind `5905`.
- Accept clear or encrypted NIP-90-style request params.
- Accept `i` tags containing signed Nostr events to schedule.
- Accept relay targets as in the legacy/NDK scheduling flow.
- Schedule by the input event's `created_at`.
- Require scheduled input event signature.
- Publish job feedback kind `7000` with scheduled/error status.

Do not require new request tags.

Implementation may internally record:

- request event id.
- requester pubkey.
- encrypted flag.
- scheduled event ids.
- error messages.

## CLI Contract

The CLI talks to the same backend APIs and can also help create DVM requests.

Required command categories:

- auth.
- accounts.
- delegates.
- proposals.
- schedule.
- queues.
- media.
- relays.
- status.
- diagnostics.

All commands that mutate state must support:

- human output.
- `--json` machine output.
- clear non-zero exit codes.

## Error Model

API errors should return:

```json
{
  "error": {
    "code": "string",
    "message": "human readable",
    "details": {}
  }
}
```

Important codes:

- `unauthorized`
- `forbidden`
- `invalid_pubkey`
- `invalid_event`
- `invalid_signature`
- `delegate_not_authorized`
- `proposal_not_editable`
- `queue_not_found`
- `signing_required`
- `publish_failed`
- `dvm_request_invalid`

