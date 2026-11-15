#!/usr/bin/env bash
set -Eeuo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
EVIDENCE_ROOT="${VESTA_VERIFY_EVIDENCE_ROOT:-$ROOT/artifacts/verification/vesta}"
RUN_ID="${1:-}"
shift || true

if [[ -z "$RUN_ID" || ! "$RUN_ID" =~ ^[a-zA-Z0-9][a-zA-Z0-9._-]*$ || $# -eq 0 ]]; then
  printf 'Usage: %s RUN_ID PLAYWRIGHT_CLI_COMMAND [ARGS...]\n' "$0" >&2
  exit 2
fi

EVIDENCE_DIR="$EVIDENCE_ROOT/$RUN_ID"
mkdir -p "$EVIDENCE_DIR"
cd "$EVIDENCE_DIR"
exec bunx --package @playwright/cli playwright-cli "-s=vesta-$RUN_ID" "$@"
