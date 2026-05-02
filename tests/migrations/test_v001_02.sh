#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$SCRIPT_DIR"
if [ -z "${DATABASE_URL:-}" ] && [ -f .env ]; then
  set -a; source <(tr -d '\r' < .env); set +a
fi
if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL not set" >&2; exit 1
fi

sqlx database drop -y >/dev/null 2>&1 || true
sqlx database create
sqlx migrate run --source migrations

EXPECTED=(bookmark_listing bookmark_external search_history analysis_report notification)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done

# bookmark_listing must have composite PK (user_id, listing_id) per spec § 5.2
PK_COLS=$(psql "$DATABASE_URL" -t -A -c "select string_agg(a.attname, ',' order by array_position(c.conkey, a.attnum)) from pg_constraint c join pg_attribute a on a.attrelid=c.conrelid and a.attnum=any(c.conkey) where c.contype='p' and c.conrelid='bookmark_listing'::regclass;")
if [ "$PK_COLS" != "user_id,listing_id" ]; then
  echo "FAIL: bookmark_listing PK expected (user_id, listing_id), got '$PK_COLS'" >&2; exit 1
fi

# bookmark_external must have unique(user_id, target_kind, target_id) per spec § 5.2
UQ_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where tablename='bookmark_external' and indexdef ilike '%unique%' and indexdef ilike '%user_id%' and indexdef ilike '%target_kind%' and indexdef ilike '%target_id%';")
if [ "$UQ_COUNT" -lt 1 ]; then
  echo "FAIL: bookmark_external missing unique(user_id, target_kind, target_id)" >&2; exit 1
fi

# search_history must have BRIN index on created_at per spec § 5.2
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='search_history' and indexdef ilike '%using brin%' and indexdef ilike '%created_at%';" | grep -q '^1$'; then
  echo "FAIL: search_history missing BRIN index on created_at" >&2; exit 1
fi

# notification must have partial index where read_at is null per spec § 5.2
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='notification' and indexdef ilike '%where (read_at is null)%';" | grep -q '^1$'; then
  echo "FAIL: notification missing partial index on read_at IS NULL" >&2; exit 1
fi

echo "PASS: V001_02 Insights BC 5 tables + composite PK + BRIN + partial idx"
exit 0
