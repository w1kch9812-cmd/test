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

EXPECTED_TABLES=( "user" listing listing_photo \
  bookmark_listing bookmark_external search_history analysis_report notification \
  audit_log outbox_event \
  pipeline_schedule pipeline_run \
  admin_action business_verification_queue listing_review_queue listing_report featured_content system_alert \
  parcel_external_data \
  api_health_check \
  external_account )

# Each table exists
for t in "${EXPECTED_TABLES[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: missing table $t" >&2; exit 1
  fi
done

# Exactly 21 public tables (excluding sqlx + PostGIS system tables)
# 18 base + parcel_external_data (V003_06, SP4-iii-d) + api_health_check (V003_07 = 30007, SP7-iii)
# + external_account (V003_08 = 30008, SP6-i).
EXPECTED_COUNT=${#EXPECTED_TABLES[@]}
COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_tables where schemaname='public' and tablename not like '\\_sqlx%' and tablename not in ('spatial_ref_sys');")
if [ "$COUNT" != "$EXPECTED_COUNT" ]; then
  echo "FAIL: expected exactly $EXPECTED_COUNT RDS tables (excl _sqlx_*, spatial_ref_sys), got $COUNT" >&2
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

# V003_01: listing transaction_type cross-field CHECK exists
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing'::regclass and conname='listing_transaction_fields_chk';" | grep -q '^1$'; then
  echo "FAIL: listing_transaction_fields_chk missing (V003_01)" >&2; exit 1
fi

# V003_02: BVQ + LRQ optimistic locking — version column exists with default 1
for tbl in business_verification_queue listing_review_queue; do
  EXISTS=$(psql "$DATABASE_URL" -t -A -c "select 1 from information_schema.columns where table_schema='public' and table_name='$tbl' and column_name='version' and data_type='bigint' and is_nullable='NO';")
  if [ "$EXISTS" != "1" ]; then
    echo "FAIL: $tbl.version missing or wrong type/nullability (V003_02)" >&2; exit 1
  fi
  DEFAULT=$(psql "$DATABASE_URL" -t -A -c "select column_default from information_schema.columns where table_schema='public' and table_name='$tbl' and column_name='version';")
  if [ "$DEFAULT" != "1" ]; then
    echo "FAIL: $tbl.version default expected '1', got '$DEFAULT'" >&2; exit 1
  fi
done

# V003_03: featured_content ends_at > starts_at CHECK exists
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='featured_content'::regclass and conname='featured_content_time_bound_chk';" | grep -q '^1$'; then
  echo "FAIL: featured_content_time_bound_chk missing (V003_03)" >&2; exit 1
fi

# V003_05: user.roles CHECK 제약 (UserRole 7 enum 값)
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='\"user\"'::regclass and conname='user_roles_valid_chk';" | grep -q '^1$'; then
  echo "FAIL: user_roles_valid_chk missing (V003_05)" >&2; exit 1
fi

# V003_06: parcel_external_data table — source CHECK constraint + BRIN index
PED_PK=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.table_constraints where table_schema='public' and table_name='parcel_external_data' and constraint_type='PRIMARY KEY';")
if [ "$PED_PK" != "1" ]; then
  echo "FAIL: parcel_external_data PK missing (V003_06)" >&2; exit 1
fi
PED_CHECK=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_constraint where conrelid='parcel_external_data'::regclass and contype='c';")
if [ "$PED_CHECK" -lt 1 ]; then
  echo "FAIL: parcel_external_data source CHECK missing (V003_06)" >&2; exit 1
fi
PED_BRIN=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='parcel_external_data' and indexname='parcel_external_data_fetched_brin_idx';")
if [ "$PED_BRIN" != "1" ]; then
  echo "FAIL: parcel_external_data_fetched_brin_idx missing (V003_06)" >&2; exit 1
fi

echo "PASS: V001-V004 $EXPECTED_COUNT RDS tables + PostGIS + ≥25 indexes (spec § 5.6)"
exit 0
