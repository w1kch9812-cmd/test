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

EXPECTED_18=( "user" listing listing_photo \
  bookmark_listing bookmark_external search_history analysis_report notification \
  audit_log outbox_event \
  pipeline_schedule pipeline_run \
  admin_action business_verification_queue listing_review_queue listing_report featured_content system_alert )

# Each table exists
for t in "${EXPECTED_18[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: missing table $t" >&2; exit 1
  fi
done

# Exactly 18 public tables (excluding sqlx + PostGIS system tables)
COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_tables where schemaname='public' and tablename not like '\\_sqlx%' and tablename not in ('spatial_ref_sys');")
if [ "$COUNT" != "18" ]; then
  echo "FAIL: expected exactly 18 RDS tables (excl _sqlx_*, spatial_ref_sys), got $COUNT" >&2
  echo "All public tables:" >&2
  psql "$DATABASE_URL" -c "select tablename from pg_tables where schemaname='public' order by tablename;" >&2
  exit 1
fi

# PostGIS extension is loaded (listing.geom_point depends on it)
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_extension where extname='postgis';" | grep -q '^1$'; then
  echo "FAIL: postgis extension not loaded" >&2; exit 1
fi

# Total non-PK indexes ≥ 25 (rough integrity check across all 5 migrations)
IDX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and indexname not like '%_pkey';")
if [ "$IDX_COUNT" -lt 25 ]; then
  echo "FAIL: expected ≥25 non-PK indexes across 18 tables, got $IDX_COUNT" >&2; exit 1
fi

# Geometry columns exist with correct SRID 4326
SRID=$(psql "$DATABASE_URL" -t -A -c "select srid from geometry_columns where f_table_schema='public' and f_table_name='listing' and f_geometry_column='geom_point';")
if [ "$SRID" != "4326" ]; then
  echo "FAIL: listing.geom_point SRID expected 4326, got '$SRID'" >&2; exit 1
fi

echo "PASS: V001 18 RDS tables + PostGIS + ≥25 indexes (spec § 5.6)"
exit 0
