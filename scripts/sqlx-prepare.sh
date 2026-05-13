#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a
  # shellcheck disable=SC1091
  source <(tr -d '\r' < .env)
  set +a
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL is required. Set it directly or add DATABASE_URL to .env." >&2
  exit 1
fi

if ! command -v sqlx >/dev/null 2>&1; then
  echo "ERROR: sqlx-cli is required. Run:" >&2
  echo "cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls" >&2
  exit 1
fi

sqlx database create
sqlx migrate run --source migrations
cargo sqlx prepare --workspace
