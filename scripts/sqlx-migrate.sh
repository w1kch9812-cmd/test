#!/usr/bin/env bash
set -euo pipefail

# Lock to repo root regardless of caller cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

# Load DATABASE_URL from .env if not already set.
# CRLF guard: strip any \r so Windows-checked-out .env still sources cleanly.
if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  source <(tr -d '\r' < .env)
  set +a
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL not set (and .env missing at repo root)" >&2
  exit 1
fi

sqlx database create
sqlx migrate run --source migrations
