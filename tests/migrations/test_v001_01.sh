#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$SCRIPT_DIR"

if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a
  source <(tr -d '\r' < .env)
  set +a
fi

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL not set" >&2
  exit 1
fi
SQLX_BIN="${SQLX_BIN:-sqlx}"

"$SQLX_BIN" database drop -y >/dev/null 2>&1 || true
"$SQLX_BIN" database create
"$SQLX_BIN" migrate run --source migrations

EXPECTED=("user" listing listing_photo)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2
    exit 1
  fi
done

GEOM_COLUMN_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.columns where table_schema='public' and table_name='listing' and column_name='geom_point';")
if [ "$GEOM_COLUMN_COUNT" != "0" ]; then
  echo "FAIL: listing.geom_point must be absent from V001 baseline schema, got $GEOM_COLUMN_COUNT" >&2
  exit 1
fi

GEOM_INDEX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing' and indexname='listing_geom_gist_idx';")
if [ "$GEOM_INDEX_COUNT" != "0" ]; then
  echo "FAIL: listing_geom_gist_idx must be absent from V001 baseline schema, got $GEOM_INDEX_COUNT" >&2
  exit 1
fi

IDX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename in ('user','listing','listing_photo') and indexname not like '%_pkey';")
if [ "$IDX_COUNT" -lt 9 ]; then
  echo "FAIL: expected at least 9 non-PK indexes on Core BC tables, got $IDX_COUNT" >&2
  exit 1
fi

echo "PASS: V001_01 Core BC 3 tables + PNU marker ownership"
