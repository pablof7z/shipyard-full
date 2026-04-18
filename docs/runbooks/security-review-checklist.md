# Security Review Checklist

Use this checklist before a Milestone 13 release and after any change touching
auth, authorization, event validation, DVM handling, or signing custody.

## Auth

- `POST /v1/auth/login` recomputes the Nostr event id before trusting it.
- Login verifies the Schnorr signature against the event pubkey.
- Login accepts only kind `27235`.
- Login requires the configured `domain`, `method`, and `u` tags.
- Login rejects stale or future timestamps outside the configured skew window.
- Session tokens are generated server-side, are unguessable, and expire.
- Logout revokes the active session.
- Authenticated routes reject missing, expired, or revoked sessions.
- Logs do not include bearer tokens, full signed auth payloads, private keys, or
  seed material.

## Delegate Authorization

- Every account-scoped route resolves the authenticated pubkey from the session,
  not from request body data.
- Owner-only mutations reject delegates.
- Delegate access is loaded from current database state on each request.
- Revoked delegates immediately lose access to owner accounts.
- Delegate-created proposals are scoped to authorized owner accounts.
- Delegates can edit or delete only their own proposals unless a future role
  explicitly expands that permission.
- Device token owner associations are checked against user ownership or active
  delegation.
- Audit events record actor pubkey, owner pubkey, action, resource type, and
  resource id without storing draft content or secrets.

## Signed Event Validation

- Signed event id is recomputed and must match the submitted `id`.
- Signed event signature is verified before scheduling or sending.
- Signed event pubkey must equal the selected owner pubkey.
- Queue-assigned signed events use the expected `created_at` for the assigned
  slot.
- Backend rejects signed events that would violate publish state transitions.
- Backend does not mutate signed event content, tags, `created_at`, or signature.
- Duplicate event ids cannot create duplicate publish items.
- Publish retries reuse the stored signed event and record every relay attempt.

## DVM Encrypted Handling

- Kind `5905` ingestion is idempotent by input event and DVM pubkey.
- DVM request parsing validates request kind, tags, target DVM pubkey, and input
  event references before scheduling.
- Encrypted NIP-90 params are decrypted only in the Rust DVM service.
- Decrypted tags or plaintext encrypted content are not written to logs.
- Feedback for encrypted requests exposes only encrypted feedback data and the
  requester `p` tag.
- DVM failure paths record clear failure codes/messages without leaking
  decrypted content.
- Stuck `processing` requests can be recovered without duplicate scheduling.

## Owner Private-Key Custody

- Backend API, worker, and DVM services never receive or store owner private
  keys.
- Owner signing happens in client-controlled signers, local CLI signers, or
  explicit remote signers chosen by the owner.
- The CLI stores owner private keys only when explicitly configured by the user.
- Agent workflows use their own pubkey for proposals and do not ask for human
  owner private keys.
- Release artifacts and logs do not include test keys beyond documented
  disposable fixtures.
- DVM service private key is scoped to DVM feedback signing only, not owner
  publishing.

## Review Evidence

Record the reviewer, Git SHA, date, and links to relevant test output. Any
unchecked item must have a tracked issue or release blocker decision.
