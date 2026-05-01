#!/usr/bin/env bash
set -euo pipefail

# Load DATABASE_URL from .env if not already set
if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  source .env
  set +a
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL not set (and .env missing)" >&2
  exit 1
fi

sqlx database create
sqlx migrate run --source migrations
