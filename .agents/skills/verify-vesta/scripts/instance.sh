#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
STATE_ROOT="${VESTA_VERIFY_STATE_ROOT:-${TMPDIR:-/tmp}/vesta-verify-${UID:-$(id -u)}}"
EVIDENCE_ROOT="${VESTA_VERIFY_EVIDENCE_ROOT:-$ROOT/artifacts/verification/vesta}"
ACTION="${1:-}"
RUN_ID="${2:-}"

fail() {
  printf 'verify-vesta: %s\n' "$*" >&2
  exit 1
}

[[ -n "$RUN_ID" && "$RUN_ID" =~ ^[a-zA-Z0-9][a-zA-Z0-9._-]*$ ]] || {
  printf 'Usage: %s {start|doctor|url|cleanup} RUN_ID\n' "$0" >&2
  exit 2
}

RUN_DIR="$STATE_ROOT/$RUN_ID"
EVIDENCE_DIR="$EVIDENCE_ROOT/$RUN_ID"
BIN="$ROOT/target/debug/vesta"

require_run() {
  [[ -d "$RUN_DIR" && -f "$RUN_DIR/run-id" ]] || fail "run '$RUN_ID' does not exist"
  [[ "$(cat "$RUN_DIR/run-id")" == "$RUN_ID" ]] || fail "run marker does not match '$RUN_ID'"
}

is_our_process() {
  local pid="$1"
  [[ "$pid" =~ ^[0-9]+$ && -r "/proc/$pid/cmdline" && -r "/proc/$pid/environ" ]] || return 1
  tr '\0' ' ' <"/proc/$pid/cmdline" | grep -Fq -- "$BIN" || return 1
  tr '\0' '\n' <"/proc/$pid/environ" | grep -Fxq -- "VESTA_VERIFY_RUN_ID=$RUN_ID"
}

port_is_owned() {
  local pid="$1" port="$2" listeners
  listeners="$(ss -H -ltnp "sport = :$port" 2>/dev/null || true)"
  [[ -n "$listeners" && "$listeners" == *"pid=$pid,"* ]]
}

copy_server_log() {
  if [[ -s "$RUN_DIR/server.log" ]]; then
    mkdir -p "$EVIDENCE_DIR"
    cp "$RUN_DIR/server.log" "$EVIDENCE_DIR/server.log"
  fi
}

stop_recorded_process() {
  local pid="$1" attempt=0
  if is_our_process "$pid"; then
    if ! kill -TERM "$pid" 2>/dev/null && is_our_process "$pid"; then
      fail "could not send SIGTERM to this Vesta run (PID $pid)"
    fi
    while kill -0 "$pid" 2>/dev/null && ((attempt < 100)); do
      sleep 0.1
      ((attempt += 1))
    done
    if is_our_process "$pid"; then
      if ! kill -KILL "$pid" 2>/dev/null && is_our_process "$pid"; then
        fail "could not stop this Vesta run (PID $pid)"
      fi
      sleep 0.1
    fi
  elif kill -0 "$pid" 2>/dev/null; then
    fail "PID $pid no longer identifies this Vesta run; refusing to signal it"
  fi
}

check_http_ready() {
  local url="$1"
  python3 - "$url" <<'PY'
import json
import sys
import urllib.error
import urllib.request

base = sys.argv[1]

def get(path):
    with urllib.request.urlopen(base + path, timeout=3) as response:
        return response.status, response.read()

for path in ("/api/health", "/api/auth/status", "/settings", "/static/ui/app.css", "/static/ui/app.js"):
    status, body = get(path)
    if status != 200 or not body:
        raise SystemExit(f"{path}: expected a non-empty HTTP 200 response, got {status}")

health = json.loads(get("/api/health?detailed=true")[1])
if health.get("status") != "success" or not health.get("data", {}).get("version"):
    raise SystemExit("detailed health response did not include the running Vesta version")

try:
    urllib.request.urlopen(base + "/api/settings", timeout=3)
except urllib.error.HTTPError as error:
    if error.code != 401:
        raise SystemExit(f"unauthenticated settings request returned HTTP {error.code}, expected 401")
else:
    raise SystemExit("unauthenticated settings request unexpectedly succeeded")
PY
}

start() {
  [[ ! -e "$RUN_DIR" ]] || fail "scratch run path already exists: $RUN_DIR"
  [[ ! -e "$EVIDENCE_DIR" ]] || fail "evidence path already exists: $EVIDENCE_DIR; choose a new run ID"
  command -v bun >/dev/null || fail "bun is required to build the frontend"
  command -v cargo >/dev/null || fail "cargo is required to build Vesta"
  command -v python3 >/dev/null || fail "python3 is required for readiness checks"
  python3 -c 'import tomllib' >/dev/null 2>&1 || fail "Python 3.11 or newer is required for TOML verification"
  command -v ss >/dev/null || fail "ss is required to verify port ownership"
  command -v curl >/dev/null || fail "curl is required for readiness checks"

  umask 077
  mkdir -p "$STATE_ROOT" "$EVIDENCE_DIR"
  mkdir -m 700 "$RUN_DIR"
  printf '%s\n' "$RUN_ID" >"$RUN_DIR/run-id"
  mkdir -p "$RUN_DIR/data"

  on_start_error() {
    local status=$?
    trap - ERR
    if [[ -s "$RUN_DIR/pid" ]]; then
      local failed_pid
      failed_pid="$(cat "$RUN_DIR/pid")"
      if kill -0 "$failed_pid" 2>/dev/null; then
        if ! is_our_process "$failed_pid"; then
          printf 'verify-vesta: failed start left PID %s unverified; preserving scratch state and refusing to signal it\n' "$failed_pid" >&2
          exit "$status"
        fi
        stop_recorded_process "$failed_pid"
      fi
    fi
    if ! copy_server_log; then
      printf 'verify-vesta: could not preserve server.log; retaining scratch at %s\n' "$RUN_DIR" >&2
      exit "$status"
    fi
    if [[ -s "$RUN_DIR/build.log" ]] && ! cp "$RUN_DIR/build.log" "$EVIDENCE_DIR/build.log"; then
      printf 'verify-vesta: could not preserve build.log; retaining scratch at %s\n' "$RUN_DIR" >&2
      exit "$status"
    fi
    rm -rf -- "$RUN_DIR"
    exit "$status"
  }
  trap on_start_error ERR

  if [[ ! -d "$ROOT/node_modules" ]]; then
    (cd "$ROOT" && bun install --frozen-lockfile) >"$EVIDENCE_DIR/build.log" 2>&1
  else
    : >"$EVIDENCE_DIR/build.log"
  fi
  (cd "$ROOT" && bun run build) >>"$EVIDENCE_DIR/build.log" 2>&1
  (cd "$ROOT" && cargo build --locked) >>"$EVIDENCE_DIR/build.log" 2>&1
  [[ -x "$BIN" ]] || fail "Rust build did not create $BIN"

  local port url pid
  port="$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1", 0)); print(s.getsockname()[1]); s.close()')"
  url="http://127.0.0.1:$port"
  printf '%s\n' "$port" >"$RUN_DIR/port"
  printf '%s\n' "$url" >"$RUN_DIR/url"
  (
    cd "$ROOT"
    nohup env \
      VESTA_CONFIG="$RUN_DIR/config.toml" \
      VESTA_DATABASE="$RUN_DIR/data/vesta.sqlite" \
      VESTA_BIND="127.0.0.1:$port" \
      VESTA_STATIC_DIR="$ROOT/static" \
      VESTA_COOKIE_SECURE=false \
      VESTA_VERIFY_RUN_ID="$RUN_ID" \
      RUST_LOG=vesta=info \
      "$BIN" >"$RUN_DIR/server.log" 2>&1 </dev/null &
    echo "$!" >"$RUN_DIR/pid"
  )
  pid="$(cat "$RUN_DIR/pid")"

  local attempt=0
  until grep -q 'vesta listening' "$RUN_DIR/server.log" && curl -fsS "$url/api/health" >/dev/null && curl -fsS "$url/settings" >/dev/null && curl -fsS "$url/static/ui/app.js" >/dev/null; do
    if ! kill -0 "$pid" 2>/dev/null; then
      cat "$RUN_DIR/server.log" >&2
      fail "Vesta exited before becoming ready"
    fi
    ((attempt += 1))
    ((attempt < 150)) || fail "timed out waiting for the ready Vesta UI"
    sleep 0.2
  done
  is_our_process "$pid" || fail "started process does not carry this run's identity marker"
  port_is_owned "$pid" "$port" || fail "port $port is not owned by this run's Vesta process"
  check_http_ready "$url"
  [[ -s "$RUN_DIR/data/vesta.sqlite" ]] || fail "SQLite database was not initialized"
  cp "$RUN_DIR/server.log" "$EVIDENCE_DIR/server.log"
  trap - ERR

  printf 'Ready: %s\nRun ID: %s\nPID: %s\nScratch: %s\nEvidence: %s\n' \
    "$url" "$RUN_ID" "$pid" "$RUN_DIR" "$EVIDENCE_DIR"
}

doctor() {
  require_run
  local pid port url version
  pid="$(cat "$RUN_DIR/pid")"
  port="$(cat "$RUN_DIR/port")"
  url="$(cat "$RUN_DIR/url")"
  version="$(python3 - "$ROOT/Cargo.toml" <<'PY'
import sys
import tomllib
with open(sys.argv[1], "rb") as file:
    print(tomllib.load(file)["package"]["version"])
PY
)"

  is_our_process "$pid" || fail "recorded PID is not this Vesta binary/run"
  port_is_owned "$pid" "$port" || fail "port $port is not owned by recorded PID $pid"
  python3 - "$url" "$version" "$RUN_DIR/data/vesta.sqlite" <<'PY'
import json
import os
import sys
import urllib.error
import urllib.request

base, expected_version, database = sys.argv[1:]

def get(path):
    with urllib.request.urlopen(base + path, timeout=5) as response:
        return response.status, response.read()

status, body = get("/api/health?detailed=true")
health = json.loads(body)
if status != 200 or health.get("status") != "success":
    raise SystemExit("detailed health check failed")
if health.get("data", {}).get("version") != expected_version:
    raise SystemExit(f"wrong Vesta version: expected {expected_version}, got {health.get('data', {}).get('version')}")

for path in ("/settings", "/static/ui/app.css", "/static/ui/app.js"):
    status, body = get(path)
    if status != 200 or not body:
        raise SystemExit(f"{path} is not serving a usable response")

status, body = get("/api/auth/status")
auth = json.loads(body).get("data", {})
if (
    status != 200
    or not isinstance(auth.get("setup_required"), bool)
    or auth.get("authenticated") is not False
):
    raise SystemExit(f"unauthenticated auth status is invalid: {auth}")

try:
    urllib.request.urlopen(base + "/api/settings", timeout=5)
except urllib.error.HTTPError as error:
    if error.code != 401:
        raise SystemExit(f"unauthenticated settings returned HTTP {error.code}, expected 401")
else:
    raise SystemExit("settings endpoint accepted an unauthenticated request")

if not os.path.isfile(database) or os.path.getsize(database) == 0:
    raise SystemExit("SQLite database is missing or empty; startup migrations did not complete")
auth_state = "first-run setup required" if auth["setup_required"] else "administrator sign-in required"
print(f"Doctor passed: Vesta {expected_version}, settings UI/assets ready, {auth_state}, auth gate enforced, migrations initialized.")
PY
}

cleanup() {
  require_run
  local pid
  pid="$(cat "$RUN_DIR/pid" 2>/dev/null || true)"
  [[ -z "$pid" ]] || stop_recorded_process "$pid"
  copy_server_log
  [[ -s "$RUN_DIR/build.log" ]] && cp "$RUN_DIR/build.log" "$EVIDENCE_DIR/build.log"
  rm -rf -- "$RUN_DIR"
  [[ -d "$EVIDENCE_DIR" ]] || fail "evidence directory was unexpectedly removed"
  printf 'Removed scratch instance %s; preserved evidence at %s\n' "$RUN_ID" "$EVIDENCE_DIR"
}

case "$ACTION" in
  start) start ;;
  doctor) doctor ;;
  url)
    require_run
    cat "$RUN_DIR/url"
    ;;
  cleanup) cleanup ;;
  *)
    printf 'Usage: %s {start|doctor|url|cleanup} RUN_ID\n' "$0" >&2
    exit 2
    ;;
esac
