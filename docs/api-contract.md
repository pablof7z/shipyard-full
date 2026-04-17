# Shipyard API Contract

The Rust API is versioned under `/v1`.

## Error Body

```json
{
  "code": "delegate_not_authorized",
  "message": "You're not authorized to propose for this account.",
  "details": {},
  "request_id": "req_..."
}
```

## Required Endpoint Groups

- `POST /v1/auth/login`
- `POST /v1/auth/logout`
- `GET /v1/auth/session`
- `GET /v1/accounts`
- `GET /v1/accounts/{owner_pubkey}/delegates`
- `POST /v1/accounts/{owner_pubkey}/delegates`
- `DELETE /v1/accounts/{owner_pubkey}/delegates/{delegate_pubkey}`
- `GET /v1/queues`
- `POST /v1/queues`
- `PATCH /v1/queues/{id}`
- `GET /v1/queues/{id}/next-slot`
- `POST /v1/queues/{id}/archive`
- `GET /v1/proposals`
- `POST /v1/proposals`
- `PATCH /v1/proposals/{id}`
- `DELETE /v1/proposals/{id}`
- `POST /v1/proposals/{id}/reject`
- `POST /v1/proposals/{id}/sign`
- `POST /v1/proposals/batch-sign`
- `GET /v1/publish-items`
- `POST /v1/publish-items/schedule`
- `POST /v1/publish-items/send-now`
- `POST /v1/publish-items/{id}/cancel`
- `POST /v1/publish-items/{id}/retry`
- `GET /v1/relays`
- `PUT /v1/relays`
- `GET /v1/dvm/requests`
- `GET /v1/devices`
- `POST /v1/devices`
- `PATCH /v1/devices/{id}`
- `DELETE /v1/devices/{id}`

Every account-scoped endpoint must use the authenticated pubkey plus database authorization.

`GET /v1/queues/{id}/next-slot` returns the next cadence-aligned slot based on the queue start, cadence, current time, and latest non-cancelled queued publish time.

Device endpoints are authenticated to the logged-in user:

- `GET /v1/devices` lists that user's registered device tokens.
- `POST /v1/devices` accepts `{ "platform": "ios" | "android", "token": "...", "owner_pubkey": null | "<pubkey>", "enabled": true }`.
- `PATCH /v1/devices/{id}` updates `enabled` and the optional account association.
- `DELETE /v1/devices/{id}` removes a token owned by the logged-in user.

## Implemented Publishing Workflow

Implemented routes:

- `GET /v1/proposals`
- `POST /v1/proposals`
- `PATCH /v1/proposals/{id}`
- `DELETE /v1/proposals/{id}`
- `POST /v1/proposals/{id}/reject`
- `POST /v1/proposals/{id}/sign`
- `POST /v1/proposals/batch-sign`
- `GET /v1/publish-items`
- `POST /v1/publish-items/schedule`
- `POST /v1/publish-items/send-now`
- `POST /v1/publish-items/{id}/cancel`
- `POST /v1/publish-items/{id}/retry`
- `GET /v1/relays`
- `PUT /v1/relays`
- `GET /v1/dvm/requests`

Signing a proposal or scheduling a signed event stores the signed event, moves the item to `SCHEDULED`, and inserts a `publish_event` job.
Batch signing accepts up to 50 proposal/signature pairs and returns a per-item result with either the scheduled publish item or an API error body.
`GET /v1/dvm/requests` returns the latest 100 DVM requests for the active owner pubkey.

## Implemented Auth Contract

`POST /v1/auth/login` accepts:

```json
{
  "event": {
    "id": "<event id>",
    "pubkey": "<64-char hex pubkey>",
    "created_at": 1776432000,
    "kind": 27235,
    "tags": [
      ["domain", "localhost"],
      ["method", "POST"],
      ["u", "http://localhost:8080/v1/auth/login"]
    ],
    "content": "Sign in to Shipyard.",
    "sig": "<schnorr signature>"
  }
}
```

Server behavior:

- Recomputes the Nostr event id.
- Verifies Schnorr signature against the event pubkey.
- Requires `kind: 27235`.
- Requires timestamp within 3 minutes.
- Requires configured domain, method, and URL tags.
- Upserts the user/account for the signing pubkey.
- Returns a UUID `session_token`.

Authenticated requests use:

```text
Authorization: Bearer <session_token>
```

Owner-scoped routes default to the logged-in pubkey. To operate on another account where the session pubkey has active delegate access, clients send:

```text
x-shipyard-owner-pubkey: <owner pubkey>
```
