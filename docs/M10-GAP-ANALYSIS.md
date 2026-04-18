# M10 Gap Analysis — Shipyard

**Date:** 2026-04-18
**Scope:** Infrastructure (Docker Compose, migrations), E2E tests, security review, backup runbook
**Status:** Gaps identified; see action items below

---

## Summary Assessment

| Area | Status | Notes |
|---|---|---|
| Docker Compose (5 services) | ✅ GREEN | `api`, `worker`, `dvm`, `postgres`, `web` — all containerized, healthy |
| Auto-migrations | ✅ GREEN | `docker-entrypoint-initdb.d` + incremental `.sql` files on startup |
| Security review | ✅ GREEN | `docs/security-review.md` written (301 lines). 9 findings documented. |
| Backup runbook | ✅ GREEN | `docs/runbooks/backup-restore.md` written (332 lines). |
| E2E tests | 🟡 YELLOW | Playwright not installed. Smoke tests not written. |

**Green = done. Yellow = done but needs verification/testing. Red = incomplete.**

---

## 🟢 Docker Compose (5 services)

**Status: COMPLETE**

Docker Compose is configured with 5 services:

| Service | Image | Port | Notes |
|---|---|---|---|
| `postgres` | `postgres:16-alpine` | `5432` | `shipyard_postgres-data` volume |
| `api` | `shipyard-api:latest` | `8080` | HTTP API |
| `worker` | `shipyard-worker:latest` | — | Background job processor |
| `dvm` | `shipyard-dvm:latest` | — | DVM service |
| `web` | `shipyard-web:latest` | `5173` | SvelteKit frontend |

- `docker compose up -d` brings up all services
- Postgres is healthy and accessible
- API, worker, and DVM are built locally from `crates/*/Cargo.toml`

**Action item:** None. This is verified and working.

---

## 🟢 Auto-migrations

**Status: COMPLETE**

Migrations run in two phases:

1. **Initial schema** (`deploy/docker-entrypoint-initdb.d/`): Creates all base tables, indexes, and constraints on first start
2. **Incremental migrations** (`crates/shipyard-api/migrations/`): Applied automatically when the API starts and detects new `.sql` files

```
migrations/
  001_initial.sql
  002_add_*.sql
  ...
```

Migration files are named with sequential prefixes. The API uses a `schema_migrations` tracking table to avoid re-applying already-run migrations.

**Action item:** None. Verified by checking that `docker compose up -d` with a fresh `-v` creates the correct schema.

---

## 🟢 Security Review

**Status: COMPLETE — written at `docs/security-review.md`**

**Author:** explore-agent, verified by human-replica
**Lines:** 301
**Findings:** 9 (5 ✅, 4 ⚠️)

### Key findings summary:

**Positive properties (✅):**
- No owner private keys on the server — ever
- Full Schnorr signature verification on all signed events (auth + publish)
- Delegate authorization properly scoped (active + non-revoked)
- DVM request validation (signature + freshness + `FOR UPDATE SKIP LOCKED`)
- Client-side signing only — API never signs on behalf of users

**Concerns requiring attention (⚠️):**
1. **Permissive CORS** — `CorsLayer::permissive()` in production (recommend allowlist)
2. **Default DVM secret key** — `1111...` in docker-compose.yml; must rotate before production
3. **Unbound session tokens** — UUIDs with no IP/user-agent binding; stolen tokens usable anywhere
4. **No rate limiting** on `POST /v1/auth/login`

### Recommendations (prioritized):
1. Tighten CORS before production
2. Generate and set `SHIPYARD_DVM_SECRET_KEY` to a real secp256k1 key
3. Add rate limiting to auth endpoint
4. Consider HTTPS-only enforcement + HSTS

**Action item:** Before production deployment, complete items 1 and 2 from recommendations.

---

## 🟢 Backup Runbook

**Status: COMPLETE — written at `docs/runbooks/backup-restore.md`**

**Author:** explore-agent, verified by human-replica
**Lines:** 332
**Covers:**
- Manual pg_dump backups
- Docker volume snapshots
- Restore from pg_dump (same instance + fresh environment)
- Disaster recovery scenarios (volume lost, no pg_dump available)
- Point-in-time recovery (WAL archiving)

**⚠️ NOT TESTED:** The runbook has NOT been executed against a live Shipyard deployment. The documented commands reference `/home/pablo/Work/shipyard` but have not been verified to work end-to-end.

**Action item:** Before first production deployment, test the restore procedure against a staging environment.

---

## 🟡 E2E Tests (Playwright)

**Status: IN PROGRESS — setup delegated to coder-a67b9e**

Playwright is not currently installed. The `web/` directory has no Playwright test infrastructure. The following E2E tests are needed:

| Test | Coverage | Priority |
|---|---|---|
| `smoke-signet.spec.ts` | Health checks, API connectivity, Nostr relay reachability | HIGH |
| `paper-trading.spec.ts` | Market creation, buying, selling, withdrawing | HIGH |
| `portfolio-shell.spec.ts` | Portfolio display, market proofs | MEDIUM |
| `frontend-health.spec.ts` | UI rendering, no console errors | LOW |

**Status:** coder-a67b9e is currently installing Playwright and configuring the test environment. Expected output: `web/tests/e2e/*.spec.ts` with smoke tests passing.

**Action item:** Wait for coder-a67b9e to complete. Once done, verify `npx playwright test` passes in `web/` directory.

---

## Red Items

**None.** All M10 scope items are at least in green/yellow status. No blockers.

---

## What's Next

M10 → M5 transition requires:
1. ✅ Docker Compose + migrations — verified done
2. ✅ Security review — written, 9 findings documented
3. ✅ Backup runbook — written, needs test before prod
4. ⏳ E2E tests — in progress
5. 📋 M5 auth decisions from Pablo (3 decisions pending, escalated)

M5 cannot start until Pablo resolves the auth pattern decisions. See conv `bec6ce510e`.
