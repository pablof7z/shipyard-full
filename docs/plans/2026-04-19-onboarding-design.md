# Onboarding Design

Date: 2026-04-19

## Goal

Wire `design/landing-page-v3.html` into the Svelte app, gate the app behind authentication, and add a lightweight onboarding flow that surfaces Shipyard's power features (queues, team, agent) without overwhelming the 90% of users who just want to schedule a post.

## Decisions

1. **Route structure.** Use SvelteKit route groups: `(marketing)` owns `/`; `(app)` owns everything else (`/write`, `/drafts`, `/scheduled`, `/queues`, `/proposals`, `/published`, `/settings`). URLs stay flat — groups are invisible to users.
2. **Auth guard.** Client-side check in `(app)/+layout.svelte`. Unauthed users get `goto('/')`. No server-side session cookies exist today; the API validates tokens per-request, so a client-side gate is sufficient.
3. **Login surface.** Modal overlay on the landing page, triggered by any "Sign in with Nostr" CTA. Content adapted from the NDK registry `login-compact.svelte` block (NIP-07 extension, nsec, ncryptsec, bunker://, nostrconnect:// QR, NIP-05).
4. **Post-login onboarding.** A single dismissible **welcome modal** on first dashboard visit. Shows four feature tiles (Schedule / Queue / Team / Agent) with a primary "Write your first post" CTA. Dismiss-once semantics via `localStorage`. Power features are discovered naturally via the existing sidebar and settings.
5. **Agent / SKILL.md.**
   - Static file at `apps/web/static/SKILL.md` — served as `https://{domain}/SKILL.md`. Content follows `agentskills.io/specification` frontmatter rules and reuses the existing `docs/shipyard-cli-and-skill-spec.md` copy.
   - A new section inside `/settings` surfaces the copy-paste prompt (`Read https://{domain}/SKILL.md and follow the instructions`) with a copy button. No new sidebar item — keeps the UI lean.

## Login flow

1. User clicks a landing-page CTA.
2. `LoginModal` opens (overlay, closable). Contains the credential picker from the registry block.
3. User picks a signer → NDK signer is configured locally (`ndk.$sessions.login(signer)` where applicable; otherwise a direct signer is constructed).
4. The modal signs a kind-27235 HTTP-auth event for the `POST /v1/auth/login` endpoint (domain + method + URL tags).
5. `shipyardApi.login(signedEvent)` returns a session token.
6. `writeShipyardSession({ token, ownerPubkey })` stores it; modal dispatches success.
7. Modal closes; `goto('/')` reloads the dashboard route which now passes the auth guard. Welcome modal triggers on mount if `shipyard.welcome_seen` is absent.

## Welcome modal

- **Trigger:** first render of `(app)/+page.svelte` (the dashboard) where `localStorage.getItem('shipyard.welcome_seen')` is null.
- **Content:** four tiles with icon + title + one-line description:
  - *Schedule posts* — write now, publish later. Primary CTA.
  - *Set up queues* — drip content on a cadence. Link to `/queues`.
  - *Post as a team* — invite delegates who propose, you sign. Link to `/settings#delegates`.
  - *Let an agent post for you* — copy a prompt for your agent. Link to `/settings#agents`.
- **Dismiss:** "Got it" button or overlay click. Sets `shipyard.welcome_seen = '1'`.

## Files to add

```
apps/web/src/routes/
├── (marketing)/
│   ├── +layout.svelte
│   └── +page.svelte              # ported landing-page-v3
└── (app)/
    └── +layout.svelte            # AppShell + auth guard

apps/web/src/lib/
├── components/onboarding/
│   ├── LoginModal.svelte
│   ├── WelcomeModal.svelte
│   └── loginState.svelte.ts      # opens/closes login modal from anywhere
└── styles/
    └── marketing.css             # landing-page-v3 styles extracted

apps/web/static/
└── SKILL.md                      # agent skill file
```

## Files to modify

- `apps/web/src/routes/+layout.svelte` — remove AppShell wrapper (moves to `(app)/+layout.svelte`). Root keeps only the global style import.
- All current routes (`+page.svelte`, `/write`, `/drafts`, `/scheduled`, `/queues`, `/proposals`, `/published`, `/settings`) move under `(app)/`.
- `(app)/settings/+page.svelte` — add an "Agents" section with the copy-prompt UI.
- `(app)/+page.svelte` (dashboard) — mount the `WelcomeModal`.

## Out of scope

- Server-side session cookies / SSR auth.
- Any changes to the API.
- Personalized SKILL.md (per-user tokens, pre-filled npubs).
- Checklist/progress tracking for feature adoption.
- Rebuilding the registry login block as shared `$lib/components/ui` primitives — we inline a Shipyard-shaped version for now.
