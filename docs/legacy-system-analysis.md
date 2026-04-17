# Legacy System Analysis

This document describes what `shipyard-previous` actually does from a product perspective. It is based on direct source inspection of commit `8e830d072a653b610dc12f95c35be20376be5a91`.

## High-Level Product

Legacy Shipyard is a Nostr scheduling tool and writing client. Its own README describes it as "a nostr scheduling tool" and the successor to Nostrit.

User-facing capabilities present in source:

- Nostr login through browser extension and some NIP-46 support.
- Write kind `1` notes.
- Save local/server drafts through the legacy `PostStatus.DRAFT` flow.
- Schedule posts at a specific time.
- Send a post "now" by scheduling it a few seconds in the future.
- Create queues with cadence and start time.
- Add posts to queues.
- Repost scheduled or existing events.
- Reply to events from feeds.
- View own feed and mentions through NDK subscriptions.
- Manage relay list for publishing.
- Connect an nsecbunker/NIP-46 signer for remote signing.
- Upload media to Satellite using a signed event.
- Run a backend worker that signs, publishes, and listens as a scheduling DVM.

The legacy product is more a proof of concept than a complete managed service. Several flows are partially implemented, inconsistent, or insecure for production.

## Monorepo Shape

The repo is a pnpm/turbo monorepo:

- `apps/web`: SvelteKit 1/Svelte 4 app.
- `apps/backend`: Node backend worker.
- `shipyard`: shared package with Prisma schema and database helpers.
- `packages`: git submodule for `@kind0/ui-common` and NDK packages. The clone has this submodule uninitialized, so behavior from those packages is inferred from imports and usage.

The web app and backend both use the same Prisma database.

## Web App

The web app is the main user interface.

Important routes:

- `/`: landing page and login entry point.
- `/write`: primary composer.
- `/posts`: all posts with filters for scheduled, published, and drafts.
- `/posts/queue/[id]`: queue-specific post list and inline composer.
- `/queues/new`: queue creation form.
- `/feeds/me`: user's own Nostr feed, with inline scheduled replies.
- `/feeds/mentions`: mentions feed, with inline scheduled replies.
- `/settings/relays`: relay settings.
- `/settings/keys`: local and remote signer settings.
- `/settings/wallet`: placeholder wallet settings.
- `/e/[id]`: event/thread page.

Important API routes:

- `POST /api/login`: validate signed login event and return JWT.
- `GET /api/user`: return current user and authorized accounts.
- `POST /api/account/[pubkey]`: switch active account by issuing a new JWT.
- `GET /api/account/settings`: return relay settings.
- `PUT /api/account/settings`: update relay settings.
- `PUT /api/account/nip46`: connect an nsecbunker/NIP-46 signer.
- `GET /api/posts`: list account posts.
- `POST /api/posts`: create post.
- `PUT /api/posts/[id]`: update post.
- `DELETE /api/posts/[id]`: delete post.
- `GET /api/queues`: list queues.
- `POST /api/queues`: create queue.

## Authentication

Legacy login uses a signed Nostr event:

- Kind `27235`.
- Content: `"Sign to verify your account."`
- Tag: `["domain", hostname]`

The server validates:

- Signature with `nostr-tools`.
- Timestamp within three minutes.
- Domain equals `shipyard.pub` in production or `localhost` otherwise.

On success:

- It upserts `Account` and `User` rows for the signing pubkey.
- It creates a `UserAccount` join row.
- It signs a JWT with `JWT_ACCESS_SECRET`.
- The JWT stores `userPubkey` and `accountPubkey`.

Limitations:

- It is not NIP-98 HTTP auth. It is a custom login event that happens to use the same kind number.
- Cookie handling is simple and does not show secure/httpOnly/sameSite hardening.
- The route returns raw error objects in some cases.
- Session payload is trusted from JWT without reloading user/account from the database.

## Account Switching And NIP-46

Legacy supports multiple accounts through `UserAccount`:

- A user can have access to multiple account pubkeys.
- The account dropdown calls `POST /api/account/[pubkey]`.
- Server checks the join table and returns a JWT with the selected `accountPubkey`.

NIP-46 linking:

- `PUT /api/account/nip46` accepts a `connectionString`.
- The route creates an `NDKNip46Signer` on `wss://relay.nsecbunker.com`.
- It waits for the signer to be ready.
- It stores the local signer's private key in `Account.nip46Pk`.
- If the remote signer pubkey differs from the logged-in user, it creates a `UserAccount` grant.

Product implication:

- Legacy already points toward delegated/multi-account publishing, but it does not model invites, writer permissions, review, or owner approval. It stores a remote signer key that lets the backend later request signatures.

## Composer

The composer creates `NDKEvent` objects:

- Kind defaults to `1`.
- Content comes from the editor.
- Pubkey is active account pubkey.
- Reply tags are added from referenced events.
- For time scheduling, `created_at` is set to publish time.
- For drafts, `created_at` is deleted.
- If active account equals the logged-in user, the browser signs immediately.
- If active account differs, the event is converted to a raw Nostr event without a signature.

Save modes:

- Send now.
- Schedule by time.
- Add to queue.
- Save draft.

The "send now" path is implemented as a time trigger with publish time set to roughly ten seconds in the future.

## Drafts

Legacy drafts are backend `Post` rows with status `DRAFT` plus local editor persistence:

- `editorContent` is persisted in local storage.
- Saving a draft creates a backend post.
- The backend schema has `PostStatus.DRAFT`.

This is not the desired new behavior. The new product must use NIP-37 draft wraps and avoid durable backend drafts.

## Queues

Legacy queues have:

- `name`
- `description`
- `cadence`
- `startAt`
- `accountPubkey`

UI creates cadence in hours, converts to minutes, and API converts to seconds. The shared helper treats `Queue.cadence` as seconds.

Queue scheduling:

- `calculateNextQueueItemTime` finds the last post in a queue by `publishTime`.
- If a last item exists, next slot is `lastPublishTime + cadence`.
- Otherwise, next slot is `queue.startAt`.
- It advances until the slot is in the future.
- It rounds to the next cadence slot from `startAt`.

When a scheduled item is put in a queue:

- The helper calculates the next publish time.
- It sets event `created_at` to that time.
- It clears `sig`.
- It stores status `NEEDS_SIGNATURE`.

Deleting queue posts attempts to shift later posts forward into the freed slot and clears their signatures. That is dangerous because it mutates future scheduled events and invalidates signatures. The new spec should not silently do this to signed events.

## Publishing Backend

The backend worker is a Node process.

Startup:

- Reads `settings.json` from the working directory.
- Requires relays, `dryMode`, and optionally a DVM private key.
- Creates an NDK instance with explicit relays.
- Connects NDK.
- Starts DVM if configured.
- Runs signing and publishing every five seconds.

Publishing:

- Finds posts with `publishedAt = null` and `publishTime <= now`.
- Filters to raw events with non-empty `sig`.
- Loads account by event pubkey.
- Uses account relay list if present.
- Publishes with NDK.
- If at least one relay accepts, sets `publishedAt` and `publishedTo`.

Limitations:

- No durable job queue.
- No retry metadata.
- No structured failure state.
- No locking around concurrent workers.
- No distinction between due-but-not-signed and failed.
- Published status enum is not updated by the publisher; `publishedAt` is the real marker.

## Signing Backend

The backend signs posts with status `NEEDS_SIGNATURE`.

It uses:

- `wss://relay.nsecbunker.com`
- `Account.nip46Pk` as the local signer private key
- `NDKNip46Signer` targeting the account pubkey

For each `NEEDS_SIGNATURE` post:

1. Create NDK event from raw event.
2. Find account by event pubkey.
3. Require `account.nip46Pk`.
4. Connect to remote signer with a ten second timeout.
5. Call `event.sign(remoteSigner)`.
6. Store signed raw event.
7. Set status `SCHEDULED`.

Limitations:

- Remote signer keys are stored in the database.
- Timeout handling logs but does not create an actionable user state.
- Signer cache is process-local.
- Signing runs in the same loop as publishing.

## DVM

Legacy DVM is implemented in `apps/backend/src/dvm.ts`.

Behavior:

- Uses the configured backend private key as DVM signer.
- Subscribes to kind `5905` events since `now - 60`.
- For each request, validates input tags.
- Supports encrypted requests by decrypting content and parsing tags from decrypted JSON.
- Reads `i` tags as JSON-encoded Nostr events.
- Requires each scheduled event to have a signature.
- Reads a `relays` tag as target relay list.
- Ensures a user/account exists for each event pubkey.
- Creates a scheduled post:
  - status `SCHEDULED`
  - trigger `TIME`
  - raw event from the input event
  - author/account pubkey from the input event pubkey
  - publish time from input event `created_at`
  - raw scheduling request stored as `dvmScheduleEvent`
- Publishes DVM feedback with status `scheduled`.
- If request was encrypted, feedback content is encrypted and tags are reduced to encrypted marker and `p` tag.
- On error, sends feedback with status `error`.

Product implication:

- This is the most important external compatibility surface. The new service can be much more robust internally, but must keep kind `5905` behavior compatible.

## Relays

Legacy stores per-account `relayList` as a string array.

Settings UI:

- Fetches user's Nostr relay list via NDK.
- Uses the account setting if present.
- Lets the user add/remove relay URLs.
- Validates `ws:` or `wss:` in the client.

Publishing:

- Backend publishes to account relay list if present.
- Otherwise publishes through the worker NDK's relay pool.

## Media Uploads

Legacy uploads media to Satellite:

- Creates kind `22242` event with content `Authorize Upload`.
- Signs it in the browser.
- Sends file to `https://api.satellite.earth/v1/media/item?auth=...`.
- Inserts returned `json.url`.

New product must replace this with Blossom-only upload.

## Feeds And Reposts

Legacy has lightweight Nostr client behavior:

- `/feeds/me` subscribes to kind `1` authored by active account.
- `/feeds/mentions` subscribes to kind `1` events tagging active account.
- Inline reply forms can create scheduled replies.
- `RepostMenu` creates kind `6` repost events, either from an existing scheduled post or an NDK event.

The rebuilt product should preserve the ability to schedule replies and reposts, but it does not need to reproduce the old feed UI exactly.

## PWA

Legacy web uses `@vite-pwa/sveltekit`, static icons, and a manifest.

This made Shipyard installable as a PWA, but the new product has explicit native mobile requirements. PWA support can remain useful for web but is not the mobile strategy.

## Known Legacy Defects And Risks

These should not be copied:

- Backend polling every five seconds instead of a durable job queue.
- Weak or incomplete authorization comments in post update/delete routes.
- Delete route calls shared `deletePost` without verifying account ownership before deletion.
- Sorting posts by `Date.getDate()` instead of timestamp in some UI code.
- Queue deletion shifts later events and clears signatures, which can surprise users.
- NIP-46 local signer private keys stored directly in account rows.
- `settings.json` contains a private DVM key in the repo.
- Inconsistent Prisma versions between packages.
- Shared helper `getAllPosts` calls `db.posts.findMany`, likely wrong model name.
- Old `+page.server.ts` form action appears stale and mismatched with current `createPost` signature.
- Drafts are backend rows instead of NIP-37 draft wraps.
- Wallet settings are placeholder only.
- Media upload is hardcoded to Satellite.

