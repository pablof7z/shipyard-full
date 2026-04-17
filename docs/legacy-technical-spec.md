# Legacy Technical Spec

This document records the technical behavior of the old implementation so engineers can understand what is being replaced.

## Runtime Components

### `apps/web`

SvelteKit app with:

- Svelte 4.
- SvelteKit 1.
- Tailwind/Daisy-style classes.
- `@kind0/ui-common` components.
- NDK packages from the uninitialized `packages` submodule.
- Svelte Query for client API state.
- Prisma client for server API routes.

The web process owns both browser UI and HTTP API routes.

### `apps/backend`

Node worker with:

- NDK relay connection.
- DVM listener.
- periodic signer pass.
- periodic publisher pass.

It reads local `settings.json`, not environment-based service config.

### `shipyard`

Shared package with:

- Prisma schema.
- exported Prisma models/types.
- enums `PostStatus` and `PostTrigger`.
- helper functions for posts, queues, user accounts, queue slot calculation.

## Database Schema

Current Prisma models:

```text
Post
User
Queue
Account
UserAccount
```

### `Post`

Fields:

- `id`
- `createdAt`
- `updatedAt`
- `rawEvent`
- `status`
- `trigger`
- `dvmScheduleEvent`
- `taggedEventId`
- `parentPostId`
- `publishTime`
- `publishedAt`
- `publishedTo`
- `accountPubkey`
- `authorPubkey`
- `queueId`

Relations:

- `accountPubkey -> Account.pubkey`
- `authorPubkey -> User.pubkey`
- optional `queueId -> Queue.id`
- optional self relation for dependencies

### `User`

Fields:

- `pubkey`

Relations:

- posts authored
- account memberships through `UserAccount`

### `Account`

Fields:

- `pubkey`
- `relayList`
- `nip46Pk`

Relations:

- posts
- queues
- members

### `Queue`

Fields:

- `id`
- `name`
- `description`
- `cadence`
- `createdAt`
- `updatedAt`
- `startAt`
- `accountPubkey`

### `UserAccount`

Join table:

- `userPubkey`
- `accountPubkey`

Composite primary key.

## Legacy Enums

```text
PostStatus:
  DRAFT
  PUBLISHED
  SCHEDULED
  NEEDS_SIGNATURE

PostTrigger:
  TIME
  QUEUE
  ADVANCED
```

`ADVANCED` exists but is disabled in UI.

`PUBLISHED` exists but the publisher mostly relies on `publishedAt`.

## Post Creation Helper

Shared helper: `shipyard/src/database/posts/create.ts`.

Input:

- status
- trigger
- tagged event id
- parent post id
- author pubkey
- account pubkey
- raw event
- publish time
- queue id

Behavior:

- Connects author and account.
- Stores raw event JSON.
- If trigger is `TIME` and status is `SCHEDULED` but raw event has no signature, status becomes `NEEDS_SIGNATURE`.
- If trigger is `QUEUE`, loads the queue, calculates next queue time, mutates event `created_at`, clears `sig`, and stores status `NEEDS_SIGNATURE`.
- If no trigger or draft-like behavior, removes publish time.

Important consequence:

- Queue entries generally require a signature after slot assignment because slot assignment mutates `created_at`.

## Queue Time Helper

Helper: `shipyard/src/queues/utils/calculateTime.ts`.

`calculateNextQueueItemTime(queue, db)`:

- Finds latest post in the queue by `publishTime`.
- Uses latest publish time plus cadence, or queue start time if none.
- Advances while time is in the past.
- Rounds to next cadence slot from queue start.

The helper assumes cadence is stored in seconds and converts to milliseconds internally.

## Delete Helper

Helper: `shipyard/src/database/posts/delete.ts`.

Behavior:

- Throws if post does not exist.
- Throws if already published.
- Non-queue posts are deleted directly.
- Queue posts cause later queue posts to be shifted into earlier slots.
- Later shifted posts have `created_at` changed, `sig` cleared, status set to `NEEDS_SIGNATURE`.

Risk:

- This invalidates signatures and changes intended publish times as a side effect of deletion.

## Web API Contracts

### `POST /api/login`

Request:

```json
{ "event": "<raw nostr event>" }
```

Response:

```json
{ "jwt": "<token>" }
```

Validation:

- event exists.
- signature valid.
- created_at within 3 minutes.
- domain tag matches current domain.

Creates account/user if missing.

### `GET /api/user`

Requires session.

Returns:

```json
{
  "pubkey": "<user pubkey>",
  "accounts": ["<account objects>"]
}
```

### `POST /api/account/[pubkey]`

Requires session and `UserAccount` grant.

Returns a new JWT with selected `accountPubkey`.

### `GET /api/posts`

Requires session.

Returns posts where `accountPubkey` equals session active account, ordered by `createdAt desc`.

### `POST /api/posts`

Requires session.

Request maps to `createPost`:

- `event`
- `status`
- `taggedEventId`
- `parentPostId`
- `publishTime`
- `queueId`
- `trigger`

Uses session `userPubkey` as author and session `accountPubkey` as account.

### `PUT /api/posts/[id]`

Requires session.

Loads existing post and checks account pubkey matches session.

Updates supplied fields:

- raw event
- status
- publish time
- queue id
- trigger

### `DELETE /api/posts/[id]`

Requires session but route does not verify account ownership before calling `deletePost`.

### `GET /api/queues`

Requires session.

Returns queues for active account.

### `POST /api/queues`

Requires session.

Creates queue with:

- name
- description
- cadence
- start time
- active account pubkey

### `GET/PUT /api/account/settings`

Currently covers relay list only.

### `PUT /api/account/nip46`

Connects nsecbunker/NIP-46 signer and stores local signer private key.

## Backend Worker Loop

`apps/backend/src/index.ts`:

1. Load `settings.json`.
2. Build NDK with explicit relay URLs.
3. Connect NDK.
4. Start DVM if settings include `dvm.key`.
5. Call `signEvents`.
6. Call `publishEvents`.
7. Repeat every five seconds.

### Publish Query

```text
where:
  publishedAt: null
  publishTime <= now
```

Then in memory:

- raw event exists
- raw event has non-empty `sig`

### Sign Query

```text
where:
  status = NEEDS_SIGNATURE
include:
  account
```

Each post is sent through backend NIP-46 signing.

## DVM Details

`apps/backend/src/dvm.ts`.

Startup:

- Creates `NDKPrivateKeySigner` from configured DVM private key.
- Sets `ndk.signer`.
- Subscribes to `{ kinds: [5905], since: now - 60 }`.

Validation:

- If request has `encrypted` tag, call `event.decrypt()` and parse decrypted content as tags.
- Else use event tags.
- For each `i` tag, parse `tag[1]` as JSON into `NDKEvent`.
- Require parsed event to have `sig`.
- For `relays`, take `tag.slice(1)`.
- Ignore `encrypted` and `p`.
- Log unknown tags.

Scheduling:

- Ensure user/account exists for scheduled event pubkey.
- Create scheduled time post with `publishTime` from scheduled event `created_at`.
- Store the raw request as `dvmScheduleEvent`.

Feedback:

- Success feedback is `NDKDVMJobFeedback` with status `scheduled`.
- It tags the request as job.
- If encrypted request, content becomes JSON string of tags, then encrypted to request author, tags become `[["encrypted"], ["p", event.pubkey]]`.
- Error feedback tags include `["status", "error", message]`.

## Template Dependencies Not Available Locally

The legacy repo expects a `packages` submodule:

```text
packages
packages/ndk/ndk
packages/ndk/ndk-cache-redis
packages/ndk/ndk-cache-dexie
packages/ndk/ndk-svelte
packages/ndk/ndk-svelte-components
```

The cloned repo has the submodule uninitialized. Analysis of those dependencies is based on import names and usage in Shipyard source.

## Production Gaps

The legacy architecture is not production-safe for the new product:

- No transactional state machine.
- No durable job queue.
- No row-level locking around due jobs.
- No retry policy.
- No secure secret management shown.
- No audit trail for delegated work.
- No mobile support.
- No CLI.
- No NIP-37 drafts.
- No Blossom-only media flow.
- No robust owner review/signing workflow.

