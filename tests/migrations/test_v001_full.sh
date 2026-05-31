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

EXPECTED_TABLES=(
  "user"
  listing
  listing_photo
  bookmark_listing
  bookmark_external
  search_history
  analysis_report
  notification
  audit_log
  outbox_event
  admin_action
  business_verification_queue
  listing_review_queue
  listing_report
  featured_content
  system_alert
  parcel_marker_anchor
  listing_marker_projection
  listing_marker_filter_registry
  listing_marker_tombstone_log
  listing_marker_delta_log
  listing_marker_dirty_tile_queue
  platform_core_event_inbox
  external_account
)

for t in "${EXPECTED_TABLES[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: missing table $t" >&2
    exit 1
  fi
done

FORBIDDEN_TABLES=(
  api_health_check
  parcel_external_data
  pipeline_run
  pipeline_schedule
)

for t in "${FORBIDDEN_TABLES[@]}"; do
  if psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: Platform Core legacy table must be absent after cleanup migration: $t" >&2
    exit 1
  fi
done

EXPECTED_COUNT=${#EXPECTED_TABLES[@]}
COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_tables where schemaname='public' and tablename not like '\\_sqlx%' and tablename not in ('spatial_ref_sys');")
if [ "$COUNT" != "$EXPECTED_COUNT" ]; then
  echo "FAIL: expected exactly $EXPECTED_COUNT RDS tables (excl _sqlx_*, spatial_ref_sys), got $COUNT" >&2
  echo "All public tables:" >&2
  psql "$DATABASE_URL" -c "select tablename from pg_tables where schemaname='public' order by tablename;" >&2
  exit 1
fi

if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_extension where extname='postgis';" | grep -q '^1$'; then
  echo "FAIL: postgis extension not loaded" >&2
  exit 1
fi

IDX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and indexname not like '%_pkey';")
if [ "$IDX_COUNT" -lt 18 ]; then
  echo "FAIL: expected at least 18 non-PK indexes across application tables, got $IDX_COUNT" >&2
  exit 1
fi

GEOM_COLUMN_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.columns where table_schema='public' and table_name='listing' and column_name='geom_point';")
if [ "$GEOM_COLUMN_COUNT" != "0" ]; then
  echo "FAIL: listing.geom_point must be absent from baseline schema, got $GEOM_COLUMN_COUNT" >&2
  exit 1
fi

GEOM_INDEX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing' and indexname='listing_geom_gist_idx';")
if [ "$GEOM_INDEX_COUNT" != "0" ]; then
  echo "FAIL: listing_geom_gist_idx must be absent from baseline schema, got $GEOM_INDEX_COUNT" >&2
  exit 1
fi

if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing'::regclass and conname='listing_transaction_fields_chk';" | grep -q '^1$'; then
  echo "FAIL: listing_transaction_fields_chk missing (V003_01)" >&2
  exit 1
fi

for tbl in business_verification_queue listing_review_queue; do
  EXISTS=$(psql "$DATABASE_URL" -t -A -c "select 1 from information_schema.columns where table_schema='public' and table_name='$tbl' and column_name='version' and data_type='bigint' and is_nullable='NO';")
  if [ "$EXISTS" != "1" ]; then
    echo "FAIL: $tbl.version missing or wrong type/nullability (V003_02)" >&2
    exit 1
  fi

  DEFAULT=$(psql "$DATABASE_URL" -t -A -c "select column_default from information_schema.columns where table_schema='public' and table_name='$tbl' and column_name='version';")
  if [ "$DEFAULT" != "1" ]; then
    echo "FAIL: $tbl.version default expected '1', got '$DEFAULT'" >&2
    exit 1
  fi
done

if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='featured_content'::regclass and conname='featured_content_time_bound_chk';" | grep -q '^1$'; then
  echo "FAIL: featured_content_time_bound_chk missing (V003_03)" >&2
  exit 1
fi

if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='\"user\"'::regclass and conname='user_roles_valid_chk';" | grep -q '^1$'; then
  echo "FAIL: user_roles_valid_chk missing (V003_05)" >&2
  exit 1
fi

PMA_SRID_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='parcel_marker_anchor'::regclass and conname='parcel_marker_anchor_srid_chk';")
if [ "$PMA_SRID_CHECK" != "1" ]; then
  echo "FAIL: parcel_marker_anchor_srid_chk missing" >&2
  exit 1
fi

PMA_GIST=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='parcel_marker_anchor' and indexname='parcel_marker_anchor_point_gist_idx';")
if [ "$PMA_GIST" != "1" ]; then
  echo "FAIL: parcel_marker_anchor_point_gist_idx missing" >&2
  exit 1
fi

PMA_LNG_LAT_COLUMNS=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.columns where table_schema='public' and table_name='parcel_marker_anchor' and column_name in ('anchor_lng', 'anchor_lat');")
if [ "$PMA_LNG_LAT_COLUMNS" != "0" ]; then
  echo "FAIL: parcel_marker_anchor must not duplicate anchor_lng/anchor_lat columns, got $PMA_LNG_LAT_COLUMNS" >&2
  exit 1
fi

LMP_SRID_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing_marker_projection'::regclass and conname='listing_marker_projection_anchor_srid_chk';")
if [ "$LMP_SRID_CHECK" != "1" ]; then
  echo "FAIL: listing_marker_projection_anchor_srid_chk missing" >&2
  exit 1
fi

LMP_TILE_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing_marker_projection' and indexname='listing_marker_projection_z14_tile_idx';")
if [ "$LMP_TILE_IDX" != "1" ]; then
  echo "FAIL: listing_marker_projection_z14_tile_idx missing" >&2
  exit 1
fi

LMP_LNG_LAT_COLUMNS=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.columns where table_schema='public' and table_name='listing_marker_projection' and column_name in ('listing_lng', 'listing_lat', 'geom_point');")
if [ "$LMP_LNG_LAT_COLUMNS" != "0" ]; then
  echo "FAIL: listing_marker_projection must not introduce listing-owned coordinate columns, got $LMP_LNG_LAT_COLUMNS" >&2
  exit 1
fi

LMFR_SPEC_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing_marker_filter_registry'::regclass and conname='listing_marker_filter_registry_spec_shape_chk';")
if [ "$LMFR_SPEC_CHECK" != "1" ]; then
  echo "FAIL: listing_marker_filter_registry_spec_shape_chk missing" >&2
  exit 1
fi

LMFR_HASH_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing_marker_filter_registry'::regclass and conname='listing_marker_filter_registry_hash_chk';")
if [ "$LMFR_HASH_CHECK" != "1" ]; then
  echo "FAIL: listing_marker_filter_registry_hash_chk missing" >&2
  exit 1
fi

LMT_TILE_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing_marker_tombstone_log' and indexname='listing_marker_tombstone_tile_active_idx';")
if [ "$LMT_TILE_IDX" != "1" ]; then
  echo "FAIL: listing_marker_tombstone_tile_active_idx missing" >&2
  exit 1
fi

LMD_TILE_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing_marker_delta_log' and indexname='listing_marker_delta_tile_active_idx';")
if [ "$LMD_TILE_IDX" != "1" ]; then
  echo "FAIL: listing_marker_delta_tile_active_idx missing" >&2
  exit 1
fi

LMDT_PENDING_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing_marker_dirty_tile_queue' and indexname='listing_marker_dirty_tile_pending_once_idx';")
if [ "$LMDT_PENDING_IDX" != "1" ]; then
  echo "FAIL: listing_marker_dirty_tile_pending_once_idx missing" >&2
  exit 1
fi

LMDT_STATUS_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing_marker_dirty_tile_queue'::regclass and conname='listing_marker_dirty_tile_status_chk';")
if [ "$LMDT_STATUS_CHECK" != "1" ]; then
  echo "FAIL: listing_marker_dirty_tile_status_chk missing" >&2
  exit 1
fi

PCEI_PAYLOAD_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='platform_core_event_inbox'::regclass and conname='platform_core_event_inbox_anchor_payload_chk';")
if [ "$PCEI_PAYLOAD_CHECK" != "1" ]; then
  echo "FAIL: platform_core_event_inbox_anchor_payload_chk missing" >&2
  exit 1
fi

PCEI_PENDING_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='platform_core_event_inbox' and indexname='platform_core_event_inbox_pending_idx';")
if [ "$PCEI_PENDING_IDX" != "1" ]; then
  echo "FAIL: platform_core_event_inbox_pending_idx missing" >&2
  exit 1
fi

PCEI_ANCHOR_SNAPSHOT_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='platform_core_event_inbox' and indexname='platform_core_event_inbox_anchor_snapshot_idx';")
if [ "$PCEI_ANCHOR_SNAPSHOT_IDX" != "1" ]; then
  echo "FAIL: platform_core_event_inbox_anchor_snapshot_idx missing" >&2
  exit 1
fi

echo "PASS: full migration chain $EXPECTED_COUNT RDS tables + PostGIS + marker coordinate ownership"
