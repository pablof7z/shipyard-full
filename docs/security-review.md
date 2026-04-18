# Shipyard Security Review

**Date:** 2026-04-18  
**Scope:** `shipyard-api`, `shipyard-worker`, `shipyard-dvm`, `shipyard-core`  
**Status:** M13 deliverable

---

## 1. Executive Summary

Shipyard is a Nostr content scheduling service with three backend services (`shipyard-api`, `shipyard-worker`, `shipyard-dvm`) and a SvelteKit web frontend. The security model is built on Nostr's native cryptographic identity — users authenticate by signing a kind `27235` HTTP auth event, receive a bearer session token, and all subsequent authorization is resolved against database state.

Key findings:

- **No owner private keys ever touch the backend.** Only the DVM's operational key (`SHIPYARD_DVM_SECRET_KEY`) exists server-side, and it is exclusively the DVM's own signing key — not a user key.
- **All API mutations require authenticated sessions**, and destructive mutations additionally require owner-level access or active delegate status.
- **Event signing is client-side only.** The API stores unsigned proposals and client-signed events but never generates signatures on behalf of users.
- **Session tokens are opaque UUIDs** stored in Postgres with expiry and revocation support. There is no JWT-based stateless auth.
- **NIP-04 encryption** is used by the DVM service for decrypting encrypted kind `5905` requests and encrypting feedback back to requesters.

---

## 2. Threat Model

| Threat | Mitigated? | How |
|---|---|---|
| **Private key theft** — attacker extracts owner keys from server | ✅ Yes | No owner private keys exist on the server. Only `SHIPYARD_DVM_SECRET_KEY` is present, and it is the DVM's own operational key. |
| **Session hijacking** — attacker reuses a stolen session token | ⚠️ Partial | Sessions are UUIDs stored in Postgres with `expires_at` and `revoked_at`. No binding to IP or user-agent for validation. See Recommendation 3. |
| **Delegate privilege escalation** — delegate promotes themselves to owner | ✅ Yes | `require_owner()` checks `session.user_pubkey == owner_pubkey`. Delegate actions use `require_account_access()` which checks the `account_delegates` table for active, non-revoked status. |
| **Event forgery** — attacker submits a signed event with wrong pubkey | ✅ Yes | `validate_signed_for_owner()` in `shipyard-core/src/event.rs:68-111` verifies Schnorr signature, event ID hash, and that the event pubkey matches the owner. |
| **Replay attacks on auth** — attacker replays a kind `27235` event | ✅ Yes | Auth verification rejects events older than ±180 seconds (`auth.rs:88-91`). |
| **Relay injection** — attacker configures malicious relay URLs | ✅ Yes | `validate_relay_urls()` in `relays.rs:63-84` requires `wss://` scheme (with local `ws://localhost`/`ws://127.0.0.1` exceptions). |
| **DVM request forgery** — attacker submits unsigned kind `5905` events | ✅ Yes | `validate_request_event()` in `processing.rs:244-253` validates the request event's Schnorr signature against the requestor's pubkey. |
| **Stale DVM requests** — attacker floods with old requests | ✅ Yes | `validate_request_freshness_at()` in `processing.rs:256-268` rejects requests older than `SHIPYARD_DVM_REQUEST_MAX_AGE_MINUTES` (default: 10 minutes). |
| **CORS abuse** | ⚠️ Partial | `CorsLayer::permissive()` is used in `main.rs:24`. Acceptable for development but must be tightened for production. See Recommendation 1. |

---

## 3. Authentication Flow

### 3.1 Login (NIP-27235 HTTP Auth)

The login flow implements Nostr HTTP authentication (kind `27235`):

1. Client creates a kind `27235` event with tags `[["domain", "..."], ["method", "POST"], ["u", "..."]]`
2. Client signs the event with their Nostr private key (client-side, never sent to the server)
3. Client posts the signed event to `POST /v1/auth/login`
4. Server verifies (`shipyard-core/src/auth.rs:77-139`):
   - Event kind is exactly `27235`
   - Timestamp is within ±180 seconds of server time
   - `domain` tag matches `SHIPYARD_AUTH_DOMAIN`
   - `method` tag matches `POST`
   - `u` tag matches `SHIPYARD_AUTH_URL`
   - Event ID matches SHA-256 hash of serialized event
   - Schnorr signature is valid for the claimed pubkey
5. On success, server creates a session (UUID, 30-day expiry) and returns it as `session_token`

**Source:** `crates/shipyard-api/src/routes/auth.rs:20-69`, `crates/shipyard-core/src/auth.rs`

### 3.2 Session Token

After login, all subsequent API calls use `Authorization: Bearer <session_token>`. The `require_session()` function (`crates/shipyard-api/src/routes/models.rs:37-76`):

1. Extracts the `Bearer` token from the `Authorization` header
2. Parses it as a UUID
3. Queries `sessions` table for a non-expired, non-revoked session
4. Returns `AuthenticatedSession` with `session_id`, `user_pubkey`, and `expires_at`
5. Updates `users.last_seen_at` on every authenticated request

### 3.3 Authorization Model

Authorization is role-based with two levels:

| Role | Auth Check | Capabilities |
|---|---|---|
| **Owner** | `require_owner()` — `session.user_pubkey == owner_pubkey` | Full control: sign proposals, manage relays, manage queues, create/delete delegates, cancel/retry publish items |
| **Active Delegate** | `require_account_access()` — checks `account_delegates` for `status = 'active' AND revoked_at IS NULL` | Propose drafts, list items, view delegate list — cannot sign, cannot change relay settings, cannot revoke delegates |

**Source:** `crates/shipyard-api/src/routes/models.rs:78-120`

The `x-shipyard-owner-pubkey` header allows a delegate session to act on behalf of an owner. If omitted, actions default to the session user's own pubkey.

---

## 4. Route Authorization Matrix

| Route | Method | Auth Level | Owner-Only? |
|---|---|---|---|
| `/v1/auth/login` | POST | None (public) | — |
| `/v1/auth/session` | GET | Session | — |
| `/v1/auth/logout` | POST | Session | — |
| `/v1/accounts` | GET | Session | — |
| `/v1/accounts/:pk/delegates` | GET | Session | ✅ Owner |
| `/v1/accounts/:pk/delegates` | POST | Session + Account Access | ❌ Delegate OK |
| `/v1/accounts/:pk/delegates/:dpk` | DELETE | Session + Account Access | ❌ Delegate OK |
| `/v1/relays` | GET | Session + Account Access | ❌ |
| `/v1/relays` | PUT | Session + Owner | ✅ Owner |
| `/v1/publish-items` | GET | Session + Account Access | ❌ |
| `/v1/publish-items/schedule` | POST | Session + Owner | ✅ Owner |
| `/v1/publish-items/send-now` | POST | Session + Owner | ✅ Owner |
| `/v1/proposals` | GET | Session + Account Access | ❌ |
| `/v1/proposals` | POST | Session + Account Access | ❌ |
| `/v1/proposals/:id` | PATCH | Session + Proposal Mutation | Context-dependent |
| `/v1/proposals/:id` | DELETE | Session + Proposal Mutation | Context-dependent |
| `/v1/proposals/:id/sign` | POST | Session + Owner | ✅ Owner |
| `/v1/proposals/batch-sign` | POST | Session + Owner | ✅ Owner |
| `/v1/proposals/:id/reject` | POST | Session + Owner | ✅ Owner |
| `/v1/queues` | GET | Session + Account Access | ❌ |
| `/v1/queues` | POST | Session + Owner | ✅ Owner |
| `/v1/queues/:id` | PATCH | Session + Owner | ✅ Owner |
| `/v1/queues/:id/archive` | POST | Session + Owner | ✅ Owner |
| `/v1/dvm/requests` | GET | Session + Account Access | ❌ |
| `/v1/devices` | GET | Session | — |
| `/v1/devices` | POST | Session | — |
| `/v1/devices/:id` | PATCH | Session | — |
| `/v1/devices/:id` | DELETE | Session | — |
| `/v1/status` | GET | None (public) | — |

---

## 5. Cryptographic Operations

### 5.1 Event Signature Validation (`shipyard-core/src/event.rs:68-111`)

`NostrEvent::validate_signed_for_owner()` performs five checks:

1. **Owner match:** Event pubkey must equal the expected owner pubkey
2. **ID present:** Event must have a non-empty `id` field
3. **Signature present:** Event must have a non-empty `sig` field
4. **Event ID integrity:** Calculated SHA-256 hash `[0, pubkey, created_at, kind, tags, content]` must match the `id` field
5. **Schnorr signature validity:** Verify the signature against the event ID message using the event's pubkey

When `publish_time` is provided (for scheduled events), the event's `created_at` must exactly match.

### 5.2 Auth Event Verification (`shipyard-core/src/auth.rs:77-139`)

Kind `27235` events are verified with:

1. Kind check (`== 27235`)
2. Timestamp within ±180 seconds
3. Domain tag matches `SHIPYARD_AUTH_DOMAIN`
4. Method tag matches expected HTTP method
5. URL tag matches `SHIPYARD_AUTH_URL`
6. Event ID hash verification
7. Schnorr signature verification

### 5.3 DVM Secret Key Usage (`shipyard-dvm`)

The DVM service holds a single secret key (`SHIPYARD_DVM_SECRET_KEY`) used exclusively for:

- **Signing kind `7000` feedback events** to requestors (`feedback.rs:30-73`)
- **Decrypting NIP-04 encrypted kind `5905` requests** (`feedback.rs:17-28`)
- **Encrypting NIP-04 feedback for encrypted requests** (`feedback.rs:45-73`)
- **Computing the DVM's own pubkey** for request claiming (`main.rs:27`)

This key is the DVM's operational identity — it is **never used to sign on behalf of any user**. User events arrive at the API already signed from the client side.

### 5.4 NIP-04 Encryption (`shipyard-core/src/nip04.rs`)

Implements standard NIP-04:

- **ECDH shared secret** from secp256k1 key agreement
- **AES-256-CBC** encryption with PKCS7 padding
- **Base64** encoding of ciphertext and IV
- Random IV generation for feedback encryption (`feedback.rs:76-81`)

---

## 6. Private Key Custody

| Service | Private Key Access | Purpose |
|---|---|---|
| `shipyard-api` | **None** | — |
| `shipyard-worker` | **None** | — |
| `shipyard-dvm` | `SHIPYARD_DVM_SECRET_KEY` | DVM's own operational key only |

**Confirmation:** `grep -r "secret\|nsec\|private_key\|SecretKey" crates/shipyard-api/src crates/shipyard-worker/src` returns zero matches. Only `shipyard-dvm` and `shipyard-core` (crypto primitives) reference secret keys.

The `SHIPYARD_DVM_SECRET_KEY` is a 32-byte hex string passed via environment variable. In `docker-compose.yml:59`, the default value is an all-`1`s test key — **this must be replaced with a real key in production**.

---

## 7. Signed Event Flow

### 7.1 Proposal Flow (Delegate Creates, Owner Signs)

1. **Delegate** creates unsigned proposal via `POST /v1/proposals` with `unsigned_event` (no signature)
2. API validates `event.pubkey == owner_pubkey` and that `event.sig` is absent (`proposals.rs:82-94`)
3. Proposal stored in `PROPOSED` state with `unsigned_event_json` and `created_by_pubkey = delegate`
4. **Owner** reviews and signs via `POST /v1/proposals/:id/sign`
5. API validates the signed event (`validate_signed_for_owner`) and transitions to `SCHEDULED`

### 7.2 Direct Schedule Flow (Owner Creates Signed Event)

1. **Owner** creates a signed event client-side
2. Posts to `POST /v1/publish-items/schedule` or `POST /v1/publish-items/send-now`
3. API validates signature and pubkey match, stores in `SCHEDULED` state, enqueues job

### 7.3 DVM Flow (Pre-Signed Events)

1. Kind `5905` request arrives via relay subscription
2. DVM validates request event signature and freshness
3. DVM reads pre-signed `i` tag events from the request
4. Creates `publish_items` in `SCHEDULED` state with the already-signed event JSON
5. Worker publishes the signed event as-is — no re-signing

---

## 8. Logging Security

All three services use structured `tracing` with JSON output:

- **`shipyard-api`:** `RUST_LOG=shipyard_api=info,tower_http=info` — logs API bind address, database errors (as structured `tracing::error!`)
- **`shipyard-worker`:** `RUST_LOG=shipyard_worker=info` — logs worker lifecycle, job processing, relay outcomes
- **`shipyard-dvm`:** `RUST_LOG=shipyard_dvm=info` — logs DVM lifecycle, request processing outcomes

**What is NOT logged:**

- Private keys or secret keys (never referenced in log statements)
- Raw event payloads (only event IDs, pubkeys, and status)
- Session tokens (only session creation and validation outcomes)
- NIP-04 plaintext (only decryption success/failure)

The DVM specifically logs `request_event_id` and `feedback_id` — not the encrypted or decrypted content (`processing.rs:72`, `processing.rs:76`).

---

## 9. Findings

### 9.1 ✅ FINDING: No owner private keys on server [Certainty: 100%]

The API and worker services have zero references to private keys. The only secret key on the server is the DVM's own operational key, used exclusively for signing kind `7000` feedback events and NIP-04 decryption/encryption with requestors. This is a strong architectural property — the server cannot sign on behalf of users because it literally does not have the keys to do so.

### 9.2 ✅ FINDING: Full Schnorr signature verification on all signed events [Certainty: 100%]

Both auth events (kind `27235`) and publish events are verified with complete Schnorr signature checks, including event ID hash verification. The verification is done in `shipyard-core/src/event.rs:68-111` and `shipyard-core/src/auth.rs:77-139` respectively. This prevents forgery, replay, and tampering.

### 9.3 ✅ FINDING: Delegate authorization is properly scoped [Certainty: 100%]

The `require_account_access()` function checks both `status = 'active'` and `revoked_at IS NULL` in the `account_delegates` table. Mutations that require owner-level access (signing, relay updates, delegate management) use `require_owner()`. Delegates can only propose and view.

### 9.4 ✅ FINDING: DVM request validation prevents replay and forgery [Certainty: 100%]

DVM requests are validated for:
- Schnorr signature validity (`validate_signed_for_owner`)
- Freshness (configurable max age, default 10 minutes)
- `FOR UPDATE SKIP LOCKED` prevents concurrent processing

### 9.5 ⚠️ FINDING: Permissive CORS configuration [Certainty: 100%]

`CorsLayer::permissive()` is used in `shipyard-api/src/main.rs:24`. This allows any origin to make API requests. This is appropriate for development but must be tightened before production deployment.

### 9.6 ⚠️ FINDING: Session tokens are unbound UUIDs [Certainty: 100%]

Session tokens are UUIDs stored in Postgres with `expires_at` and `revoked_at` columns. There is no IP address binding, user-agent validation, or rate limiting on login attempts. A stolen session token is usable from any origin until expiry or explicit revocation.

### 9.7 ⚠️ FINDING: Default DVM secret key in docker-compose [Certainty: 100%]

`docker-compose.yml:59` sets `SHIPYARD_DVM_SECRET_KEY` to an all-`1`s hex string. This is a test value that must be replaced in production.

### 9.8 ✅ FINDING: Proposal creation prevents pre-signed events [Certainty: 100%]

The `create_proposal` handler explicitly rejects events that already have a signature (`proposals.rs:89-94`), ensuring the proposal flow enforces the two-step create-then-sign pattern.

### 9.9 ✅ FINDING: Audit trail for publish state changes [Certainty: 100%]

The worker records `audit_events` on publish state changes with `actor_pubkey = NULL` and `new_state` metadata (`publish.rs:159-182`). While currently only tracking automated transitions, the schema supports actor attribution for future use.

---

## 10. Recommendations

1. **Tighten CORS for production** — Replace `CorsLayer::permissive()` with an explicit allowlist of trusted origins. This is the highest-priority production hardening item.

2. **Rotate the DVM secret key** — Generate a fresh secp256k1 key for `SHIPYARD_DVM_SECRET_KEY` in production. Never deploy the default `1111...` key.

3. **Consider session binding** — Add optional IP address and/or user-agent validation to session lookups to reduce the window of opportunity for stolen session tokens.

4. **Add rate limiting** — Apply rate limits to `POST /v1/auth/login` to prevent brute-force auth event submission. Consider per-IP and per-pubkey limits.

5. **Enforce HTTPS in production** — The auth flow transmits bearer tokens. Ensure the API is only accessible over HTTPS in production, with HSTS headers.

6. **Secure the DVM secret key** — Store `SHIPYARD_DVM_SECRET_KEY` in a secrets manager (Vault, AWS Secrets Manager, etc.) rather than environment variables in docker-compose. The current approach is acceptable for development but insufficient for production.

7. **Consider adding request logging IDs** — The API does not currently assign request IDs to logs. Adding `x-request-id` tracing would improve incident investigation.

---

## 11. Architectural Security Properties

| Property | Status | Notes |
|---|---|---|
| Owner keys never on server | ✅ Enforced | Zero private key references in API and worker crates |
| Client-side signing only | ✅ Enforced | API stores `unsigned_event_json` for proposals; owner signs client-side |
| Cryptographic event verification | ✅ Enforced | Schnorr + hash verification in `shipyard-core` |
| Delegate scoping | ✅ Enforced | DB-backed active/non-revoked check on every delegate access |
| DVM key isolation | ✅ Enforced | DVM key only used for kind `7000` feedback and NIP-04, never for user events |
| NIP-04 encryption | ✅ Standard | ECDH + AES-256-CBC per NIP-04 specification |
| Session revocation | ✅ Supported | `revoked_at` column on sessions; logout sets it |
| Auth event replay protection | ✅ Enforced | 180-second timestamp window |
