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

EXPECTED=("user" listing listing_photo)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done
if ! psql "$DATABASE_URL" -t -A -c "select 1 from information_schema.columns where table_name='listing' and column_name='geom_point';" | grep -q '^1$'; then
  echo "FAIL: listing.geom_point missing" >&2; exit 1
fi

# Verify SRID 4326 (charter §1: SRID 미지정 공간 쿼리 금지)
SRID=$(psql "$DATABASE_URL" -t -A -c "select srid from geometry_columns where f_table_name='listing' and f_geometry_column='geom_point';")
if [ "$SRID" != "4326" ]; then
  echo "FAIL: listing.geom_point SRID expected 4326, got '$SRID'" >&2; exit 1
fi

# Verify all 10 indexes created (3 user + 6 listing + 1 listing_photo)
IDX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename in ('user','listing','listing_photo') and indexname not like '%_pkey';")
if [ "$IDX_COUNT" -lt 10 ]; then
  echo "FAIL: expected ≥10 non-PK indexes on Core BC tables, got $IDX_COUNT" >&2; exit 1
fi

echo "PASS: V001_01 Core BC 3 tables + SRID 4326 + 10 indexes"
exit 0
