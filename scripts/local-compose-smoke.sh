#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/deploy/docker-compose.yml"
PROJECT="${SHIPYARD_SMOKE_PROJECT:-shipyard-smoke-$$}"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/shipyard-smoke.XXXXXX")"

API_PORT="${SHIPYARD_SMOKE_API_PORT:-18080}"
WEB_PORT="${SHIPYARD_SMOKE_WEB_PORT:-13000}"
POSTGRES_PORT="${SHIPYARD_SMOKE_POSTGRES_PORT:-15432}"
API_URL="http://127.0.0.1:${API_PORT}"
WEB_URL="http://127.0.0.1:${WEB_PORT}"
AUTH_URL="http://localhost:${API_PORT}/v1/auth/login"
OWNER_SECRET="${SHIPYARD_SMOKE_OWNER_SECRET:-2222222222222222222222222222222222222222222222222222222222222222}"

export SHIPYARD_API_HOST_PORT="$API_PORT"
export SHIPYARD_WEB_HOST_PORT="$WEB_PORT"
export SHIPYARD_POSTGRES_HOST_PORT="$POSTGRES_PORT"
export SHIPYARD_AUTH_DOMAIN="localhost"
export SHIPYARD_AUTH_URL="$AUTH_URL"
export PUBLIC_SHIPYARD_API_URL="http://localhost:${API_PORT}"
export PUBLIC_SHIPYARD_AUTH_DOMAIN="localhost"
export PUBLIC_SHIPYARD_AUTH_URL="$AUTH_URL"
export SHIPYARD_DVM_RELAYS="${SHIPYARD_DVM_RELAYS:-ws://127.0.0.1:9}"
export SHIPYARD_DVM_TICK_SECONDS="${SHIPYARD_DVM_TICK_SECONDS:-1}"
export SHIPYARD_WORKER_TICK_SECONDS="${SHIPYARD_WORKER_TICK_SECONDS:-1}"
export SHIPYARD_WORKER_BASE_BACKOFF_SECONDS="${SHIPYARD_WORKER_BASE_BACKOFF_SECONDS:-1}"

cd "$ROOT_DIR"

dc() {
  docker compose -p "$PROJECT" -f "$COMPOSE_FILE" "$@"
}

log() {
  printf '\n[%s] %s\n' "$(date +%H:%M:%S)" "$*"
}

fail() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

cleanup() {
  local status=$?
  if [ "$status" -ne 0 ]; then
    printf '\nSmoke failed. Recent service logs follow.\n' >&2
    dc logs --tail=80 api worker dvm web >&2 || true
  fi
  if [ "${SHIPYARD_SMOKE_KEEP_STACK:-0}" = "1" ]; then
    printf 'Keeping compose project %s for inspection.\n' "$PROJECT" >&2
  else
    dc down -v --remove-orphans >/dev/null 2>&1 || true
  fi
  rm -rf "$TMP_DIR"
  exit "$status"
}
trap cleanup EXIT

require_command() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

json_field() {
  node -e '
const fs = require("fs");
const obj = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const path = process.argv[2].split(".");
let value = obj;
for (const key of path) value = value?.[key];
if (value === undefined || value === null) process.exit(2);
process.stdout.write(typeof value === "object" ? JSON.stringify(value) : String(value));
' "$1" "$2"
}

write_wrapped_event() {
  node -e '
const fs = require("fs");
const event = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
const field = process.argv[2];
process.stdout.write(JSON.stringify({ [field]: event }));
' "$1" "$2" >"$3"
}

write_queue_request() {
  node -e '
process.stdout.write(JSON.stringify({
  name: "Smoke Queue",
  description: "Local Compose smoke queue",
  cadence_seconds: 3600,
  start_at: new Date(Number(process.argv[1]) * 1000).toISOString()
}));
' "$1" >"$2"
}

write_proposal_request() {
  node -e '
const fs = require("fs");
const event = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
event.id = null;
event.sig = null;
process.stdout.write(JSON.stringify({
  owner_pubkey: process.argv[2],
  unsigned_event: event,
  trigger: "QUEUE",
  queue_id: process.argv[3]
}));
' "$1" "$2" "$3" >"$4"
}

write_send_now_request() {
  node -e '
const fs = require("fs");
const event = JSON.parse(fs.readFileSync(process.argv[1], "utf8"));
process.stdout.write(JSON.stringify({
  signed_event: event,
  trigger: "SEND_NOW",
  publish_time: null,
  queue_id: null
}));
' "$1" >"$2"
}

wait_for_http() {
  local url=$1
  local label=$2
  for _ in $(seq 1 90); do
    if curl -fsS "$url" >/dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  fail "$label did not become ready at $url"
}

psql_scalar() {
  dc exec -T postgres psql -U shipyard -d shipyard -tA -c "$1" | tr -d '[:space:]'
}

for command in cargo curl docker node; do
  require_command "$command"
done

log "Validating Docker Compose configuration"
dc config >/dev/null

log "Building smoke fixture"
cargo build -q -p shipyard-cli --bin shipyard-smoke-fixture
FIXTURE="$ROOT_DIR/target/debug/shipyard-smoke-fixture"

log "Starting local smoke stack as project ${PROJECT}"
dc down -v --remove-orphans >/dev/null 2>&1 || true
dc up --build -d postgres api worker dvm web

log "Waiting for API and web"
wait_for_http "$API_URL/healthz" "API"
wait_for_http "$WEB_URL/" "web"

OWNER_PUBKEY="$("$FIXTURE" pubkey "$OWNER_SECRET")"
NOW="$(date +%s)"
AUTH_EVENT="$TMP_DIR/auth-event.json"
AUTH_REQUEST="$TMP_DIR/auth-request.json"
LOGIN_RESPONSE="$TMP_DIR/login-response.json"

log "Logging in with deterministic test owner"
"$FIXTURE" auth "$OWNER_SECRET" "$NOW" localhost "$AUTH_URL" >"$AUTH_EVENT"
write_wrapped_event "$AUTH_EVENT" event "$AUTH_REQUEST"
curl --fail-with-body -sS -X POST "$API_URL/v1/auth/login" \
  -H 'content-type: application/json' \
  --data-binary "@$AUTH_REQUEST" >"$LOGIN_RESPONSE"

SESSION_TOKEN="$(json_field "$LOGIN_RESPONSE" session_token)"
LOGIN_PUBKEY="$(json_field "$LOGIN_RESPONSE" user_pubkey)"
[ "$LOGIN_PUBKEY" = "$OWNER_PUBKEY" ] || fail "login pubkey did not match fixture pubkey"

AUTH_HEADERS=(
  -H "authorization: Bearer ${SESSION_TOKEN}"
  -H "x-shipyard-owner-pubkey: ${OWNER_PUBKEY}"
  -H "content-type: application/json"
)

log "Checking authenticated session"
curl --fail-with-body -sS "$API_URL/v1/auth/session" "${AUTH_HEADERS[@]}" >"$TMP_DIR/session.json"

log "Configuring local failing relay for worker attempt coverage"
curl --fail-with-body -sS -X PUT "$API_URL/v1/relays" \
  "${AUTH_HEADERS[@]}" \
  -d '{"relay_urls":["ws://127.0.0.1:9"]}' >"$TMP_DIR/relays.json"

log "Creating queue and verifying next slot"
QUEUE_START_EPOCH=$((NOW + 3600))
write_queue_request "$QUEUE_START_EPOCH" "$TMP_DIR/queue-request.json"
curl --fail-with-body -sS -X POST "$API_URL/v1/queues" \
  "${AUTH_HEADERS[@]}" \
  --data-binary "@$TMP_DIR/queue-request.json" >"$TMP_DIR/queue-response.json"
QUEUE_ID="$(json_field "$TMP_DIR/queue-response.json" id)"
curl --fail-with-body -sS "$API_URL/v1/queues/$QUEUE_ID/next-slot" \
  "${AUTH_HEADERS[@]}" >"$TMP_DIR/next-slot.json"

log "Creating and signing queued proposal"
PROPOSAL_SIGNED_EVENT="$TMP_DIR/proposal-signed-event.json"
PROPOSAL_REQUEST="$TMP_DIR/proposal-request.json"
PROPOSAL_RESPONSE="$TMP_DIR/proposal-response.json"
SIGN_REQUEST="$TMP_DIR/sign-request.json"
SIGN_RESPONSE="$TMP_DIR/sign-response.json"
"$FIXTURE" note "$OWNER_SECRET" "$QUEUE_START_EPOCH" "Shipyard local smoke queued proposal" \
  >"$PROPOSAL_SIGNED_EVENT"
write_proposal_request "$PROPOSAL_SIGNED_EVENT" "$OWNER_PUBKEY" "$QUEUE_ID" "$PROPOSAL_REQUEST"
curl --fail-with-body -sS -X POST "$API_URL/v1/proposals" \
  "${AUTH_HEADERS[@]}" \
  --data-binary "@$PROPOSAL_REQUEST" >"$PROPOSAL_RESPONSE"
PROPOSAL_ID="$(json_field "$PROPOSAL_RESPONSE" id)"
write_wrapped_event "$PROPOSAL_SIGNED_EVENT" signed_event "$SIGN_REQUEST"
curl --fail-with-body -sS -X POST "$API_URL/v1/proposals/$PROPOSAL_ID/sign" \
  "${AUTH_HEADERS[@]}" \
  --data-binary "@$SIGN_REQUEST" >"$SIGN_RESPONSE"
SIGNED_STATE="$(json_field "$SIGN_RESPONSE" state)"
[ "$SIGNED_STATE" = "SCHEDULED" ] || fail "proposal did not become SCHEDULED"

log "Creating due send-now item and waiting for worker publish attempt"
SEND_EVENT="$TMP_DIR/send-event.json"
SEND_REQUEST="$TMP_DIR/send-request.json"
SEND_RESPONSE="$TMP_DIR/send-response.json"
SEND_CREATED_AT="$(date +%s)"
"$FIXTURE" note "$OWNER_SECRET" "$SEND_CREATED_AT" "Shipyard local smoke send-now" >"$SEND_EVENT"
write_send_now_request "$SEND_EVENT" "$SEND_REQUEST"
curl --fail-with-body -sS -X POST "$API_URL/v1/publish-items/send-now" \
  "${AUTH_HEADERS[@]}" \
  --data-binary "@$SEND_REQUEST" >"$SEND_RESPONSE"
SEND_ITEM_ID="$(json_field "$SEND_RESPONSE" id)"

ATTEMPTS="0"
for _ in $(seq 1 60); do
  ATTEMPTS="$(psql_scalar "select count(*) from publish_attempts where publish_item_id = '$SEND_ITEM_ID';")"
  if [ "${ATTEMPTS:-0}" -gt 0 ]; then
    break
  fi
  sleep 1
done
[ "${ATTEMPTS:-0}" -gt 0 ] || fail "worker did not record a publish attempt"

log "Checking DVM request endpoint and service process"
curl --fail-with-body -sS "$API_URL/v1/dvm/requests" "${AUTH_HEADERS[@]}" >"$TMP_DIR/dvm-requests.json"
DVM_RUNNING="$(dc ps --status running --services dvm)"
[ "$DVM_RUNNING" = "dvm" ] || fail "DVM service is not running"

printf '\nLocal Compose smoke passed.\n'
printf 'API: %s\n' "$API_URL"
printf 'Web: %s\n' "$WEB_URL"
printf 'Owner pubkey: %s\n' "$OWNER_PUBKEY"
printf 'Queued proposal: %s\n' "$PROPOSAL_ID"
printf 'Send-now item: %s (%s worker attempt rows)\n' "$SEND_ITEM_ID" "$ATTEMPTS"
