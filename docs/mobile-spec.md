# Mobile Spec

## Product Goal

The mobile app is a first-class Shipyard client, not just a wrapped website. It should support writing, Nostr drafts, Blossom media, queue management, delegated proposals, owner review/signing, and publish status monitoring.

The implementation should share Rust code where it reduces duplication without compromising native UX.

## Platforms

Required:

- iOS native frontend.
- Android native frontend.
- Shared Rust library for domain models, API client, validation, and possibly Nostr helpers.

Frontend technology can be selected later, but the product requirements are fixed.

## Shared Rust Core

The shared core should include:

- API client.
- Auth proof construction helpers where possible.
- Nostr event validation helpers.
- State machine types.
- Queue slot preview logic.
- Proposal and publish item models.
- Blossom server selection logic.
- CLI-compatible serialization types.

Platform-specific code should handle:

- Secure key storage.
- Native signer integrations.
- Push notifications.
- File picker/camera/photos.
- Background execution constraints.
- UI.

## Authentication And Signing

Mobile must support:

- Secure local key storage if user imports or creates a key.
- Remote signer/NIP-46 where feasible.
- Platform signer integrations where available, such as Android signer apps.
- Read-only/proposal mode where the app can propose but not sign for an owner.

Signing UX:

- Signing should be explicit for owner review.
- Batch signing must show count, affected account, and earliest publish time.
- If signing uses a remote signer, show pending and failure states clearly.

## Drafts

Drafts use NIP-37.

Mobile must:

- Create encrypted draft wraps.
- List drafts from private content relays.
- Decrypt drafts locally.
- Support offline editing cache.
- Publish changes when connectivity returns.
- Delete drafts through blank-content draft wrap events.

The mobile app may keep a local editing cache for offline use, but canonical durable drafts are Nostr draft events.

## Composer

Mobile composer must support:

- Plain text notes.
- Reply tags.
- Repost scheduling where available.
- Media insertion through Blossom.
- Time selection.
- Queue selection.
- Proposal creation for delegated owner accounts.

The composer should make the active publishing identity obvious. If the user is proposing for another pubkey, the UI must indicate that owner signature is required.

## Blossom Media

Mobile upload flow:

1. Resolve user's Blossom server list kind `10063`.
2. Use first server.
3. Fallback to `https://blossom.primal.net`.
4. Upload selected media with Blossom auth.
5. Insert returned URL into the draft/composer.

Media sources:

- Camera.
- Photo library.
- Files.
- Share sheet.

Failure states:

- no signer.
- no server list and fallback failed.
- upload rejected.
- network failure.
- invalid response.

## Queues

Mobile must support:

- List queues.
- Create queue.
- Edit queue basics.
- Show next slot preview.
- Add a draft/proposal/signed item to queue.
- View queue-specific scheduled items.

Advanced queue rebasing is not a mobile-specific requirement, but mobile must display when an item needs signature because queue timing changed.

## Delegated Proposals

Mobile must support both sides:

Delegate:

- View authorized owner accounts.
- Switch active account.
- Create proposal.
- Edit own proposed item before owner action.
- Delete own proposed item before owner action.
- See proposal status.

Owner:

- Pending proposal inbox.
- Preview final event.
- Edit proposal.
- Reject proposal.
- Sign proposal.
- Batch sign selected proposals.

No persistent public proposer credit is shown after owner signing.

## Notifications

Useful notification types:

- Owner has pending proposals.
- Proposal was signed.
- Proposal was rejected.
- Scheduled publish failed.
- Remote signing timed out.
- Item is about to publish soon if user opted in.

Implementation notes:

- Push notifications require backend device token registration.
- Local notifications can be used for locally known scheduled times.
- Notification settings must be per device and per account.

Implemented backend registration contract:

- `GET /v1/devices`
- `POST /v1/devices`
- `PATCH /v1/devices/{id}`
- `DELETE /v1/devices/{id}`

Device rows are owned by the authenticated user and may optionally be associated with an owner account after account/delegate authorization.

## Offline Behavior

Offline support should prioritize writing:

- Draft editing works offline through local cache.
- Draft sync resumes when online.
- Proposal submission requires backend connectivity.
- Scheduling signed events requires backend connectivity.
- Blossom upload requires network.

Conflicts:

- If the same draft was edited on multiple devices, show conflict resolution instead of overwriting silently.
- Backend publish items are authoritative once created.

## Mobile Security

Requirements:

- Use platform secure storage for secrets.
- Do not log private keys or signed auth payloads.
- Require explicit confirmation before importing private keys.
- Allow logout and local data wipe.
- Separate local signer identity from active owner account.
- Clearly label delegated proposal mode.

## Acceptance Criteria

Mobile v1 is complete when a user can:

- Login.
- Write and save a NIP-37 draft.
- Upload an image through Blossom.
- Schedule a signed note.
- Create and manage queues.
- Invite is managed on web or mobile, but mobile can use existing delegated access.
- Propose a queued or timed post for an owner account.
- Review and sign proposals as owner.
- See publish success/failure state.
