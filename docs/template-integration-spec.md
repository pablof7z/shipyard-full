# NDK SvelteKit Template Integration Spec

## Purpose

Shipyard's new web app should start from `nostr-dev-kit/ndk-template-sveltekit-vercel`, but it must not inherit the template's Vercel deployment target as a product constraint.

The template is useful because it already solves Nostr app foundations:

- SvelteKit 2 and Svelte 5 app structure.
- NDK client setup.
- Session persistence.
- Signer login UX.
- Blossom-backed onboarding examples.
- SSR Nostr fetching patterns.
- SEO/social preview pattern.
- Clear code boundaries between NDK, server, routes, and components.

## Template Facts Observed

Template commit inspected: `907086a15a7ab8ee9b862552b2570d333be76009`.

Dependencies:

- `@nostr-dev-kit/ndk`
- `@nostr-dev-kit/svelte`
- `@nostr-dev-kit/sessions`
- `@nostr-dev-kit/blossom`
- `@sveltejs/kit`
- `svelte`
- `bits-ui`
- `sharp`

Current adapter:

- `@sveltejs/adapter-vercel`

Important files:

- `src/lib/ndk/client.ts`
- `src/lib/features/auth/AuthPanel.svelte`
- `src/lib/features/auth/LoginDialog.svelte`
- `src/lib/features/auth/auth.ts`
- `src/lib/onboarding.ts`
- `src/lib/server/nostr.ts`
- `src/lib/seo.ts`
- `src/routes/+layout.svelte`

## What To Keep

### NDK Client Boundary

Keep a dedicated NDK client module similar to `src/lib/ndk/client.ts`.

It should:

- Create the browser NDK instance.
- Configure default relay list.
- Configure session persistence.
- Monitor relevant Nostr lists:
  - relay lists
  - Blossom list `kind:10063`
  - private draft relay list `kind:10013` if supported by the NDK session layer or custom code
- Provide `ensureClientNdk`.

### Session UX

Keep login modes:

- Browser extension.
- Private key, if the product accepts this risk with explicit warnings.
- Remote signer/NIP-46.

The template's remote signer pairing UX is a better starting point than the legacy nsecbunker-only form.

### SSR And Client Split

Keep the template principle:

- Server routes fetch preview-critical Nostr data in `+page.server.ts`.
- Browser NDK subscriptions handle live session-aware UI.

Shipyard-specific examples:

- Event detail routes can SSR fetch target event and metadata.
- Public or shared proposal links, if added later, should SSR fetch safe public data.
- The scheduling cockpit itself is authenticated and can be client-heavy.

### Blossom Helpers

Keep and adapt:

- `DEFAULT_BLOSSOM_SERVER = 'https://blossom.primal.net'`
- Blossom server parsing and normalization.
- `NDKBlossomList` handling.
- Upload via `@nostr-dev-kit/blossom`.

Shipyard should remove old Satellite upload code entirely.

### Code Boundaries

Preserve the template's preferred boundaries:

- `src/routes`: route orchestration.
- `src/lib/server`: server-only Nostr and backend API helpers.
- `src/lib/ndk`: NDK integration, Nostr event builders, formatters, and registry primitives.
- `src/lib/components`: app-level presentation components.
- `src/lib/components/ui`: generic UI primitives.

## What To Replace

### Deployment Adapter

Replace `@sveltejs/adapter-vercel`.

Preferred first implementation:

- `@sveltejs/adapter-node`, containerized beside the Rust API or behind the same reverse proxy.

Alternative:

- static web build only if all authenticated API calls go to Rust backend and SSR requirements are removed or separately handled. This is not recommended as the first target because the template's SSR pattern is valuable.

### Product Surfaces

Remove or replace template example surfaces:

- Highlighter brand.
- Article feed.
- Comments sidebar.
- Highlights.
- Managed NIP-05 onboarding as a core journey.

Replace with Shipyard surfaces:

- Dashboard.
- Composer.
- NIP-37 draft list/editor.
- Scheduled items.
- Queues.
- Proposal inbox.
- Delegate management.
- Relay settings.
- Blossom settings.
- DVM access/help page.
- CLI download/help page.

### Backend Assumptions

The web app should not implement durable Shipyard business logic in SvelteKit API routes.

Instead:

- SvelteKit talks to `shipyard-api`.
- SvelteKit may have thin server load helpers.
- Rust backend owns persistence, authorization, state machine, and workers.

## Suggested Web App Structure

```text
src/lib/ndk/
  client.ts
  events/
  drafts/
  blossom/
  relays/

src/lib/api/
  client.ts
  auth.ts
  accounts.ts
  proposals.ts
  posts.ts
  queues.ts
  relays.ts

src/lib/features/
  auth/
  composer/
  drafts/
  proposals/
  queues/
  schedule/
  settings/
  media/

src/routes/
  +layout.svelte
  +page.svelte
  write/
  drafts/
  posts/
  proposals/
  queues/
  settings/
  event/[id]/
```

## Auth Integration

The template uses NDK session state. Shipyard also needs backend authentication.

Recommended flow:

1. User logs into browser NDK session.
2. Web client creates a backend auth proof using a signed Nostr event.
3. Backend verifies event and issues a Shipyard session.
4. Web stores backend session with secure cookie if SSR is used, or an appropriate token mechanism if app is fully client-side.
5. API requests include the backend session.
6. Backend always authorizes by session pubkey plus server-side account/delegate permissions.

The legacy custom login event can be replaced with a more standard HTTP auth proof, but the docs do not require changing the DVM protocol.

## Draft Integration

Drafts are not backend records.

Web draft module must:

- Create NIP-37 draft wrap events.
- Encrypt draft content to the signer.
- Publish draft wraps to private content relays from kind `10013` when available.
- Fall back to a user-visible default/private relay strategy if no private relay list exists.
- Load and decrypt draft wraps for the active user.
- Delete drafts by publishing blank-content draft wrap events.

The composer should be able to promote a draft into:

- signed scheduled item
- queued signed item
- delegated proposal

## Proposal Integration

The web app needs a clear active account model:

- `logged_in_pubkey`
- `active_owner_pubkey`
- list of authorized owner accounts from backend

If active owner differs from logged-in pubkey:

- Composer builds unsigned event with `pubkey = active_owner_pubkey`.
- Save action creates a proposal through backend API.
- UI must show proposal status, not scheduled status, until owner signs.

If active owner equals logged-in pubkey:

- Composer can sign locally and create scheduled items directly.
- Owner can also create own proposals if they choose to stage review, but default should be direct scheduling.

## Blossom Integration

Use template code as reference:

- `NDKBlossomList`
- `DEFAULT_BLOSSOM_SERVER`
- server parsing.
- `NDKBlossom.upload`.

Shipyard-specific media flow:

1. Resolve active signing user's Blossom server list.
2. Choose first server.
3. If no list, use `https://blossom.primal.net`.
4. Upload.
5. Insert returned URL into editor.
6. If owner and logged-in pubkey differ, the delegate's signer uploads media. The final event content can contain that URL.

## Styling And UX Direction

Do not preserve the legacy app's UI wholesale.

Use the template as a modern SvelteKit/NDK base, but design Shipyard as a focused publishing cockpit:

- Dense enough for queue/review workflows.
- Clear status labels.
- Strong distinction between drafts, proposals, scheduled, and published.
- Fast account switcher.
- Mobile-responsive from the start.

## Migration From Template

Implementation steps:

1. Fork/copy template.
2. Replace app name, copy, SEO, and example routes.
3. Replace Vercel adapter.
4. Add Shipyard API client.
5. Add authenticated layout and active account state.
6. Add draft wrap module.
7. Add Blossom media module.
8. Add composer.
9. Add queue and proposal screens.
10. Add owner signing UI.
11. Add CLI install/help docs route.

