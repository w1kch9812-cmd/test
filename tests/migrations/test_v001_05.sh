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

# All 6 Operations tables must exist per spec § 5.5
EXPECTED=(admin_action business_verification_queue listing_review_queue listing_report featured_content system_alert)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done

# admin_action timeline index (admin_id, created_at desc) per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='admin_action' and indexdef ilike '%(admin_id, created_at desc%';" | grep -q '^1$'; then
  echo "FAIL: admin_action missing composite index (admin_id, created_at desc)" >&2; exit 1
fi

# business_verification_queue.status CHECK must allow exactly the 4 spec values per spec § 5.5
for s in pending approved rejected needs_more_info; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='business_verification_queue'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${s}%' and pg_get_constraintdef(c.oid) ilike '%status%';" | grep -q '^1$'; then
    echo "FAIL: business_verification_queue.status CHECK missing '$s'" >&2; exit 1
  fi
done

# business_verification_queue partial index on pending entries per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='business_verification_queue' and indexdef ilike '%submitted_at%' and indexdef ilike '%where (status = ''pending''::text)%';" | grep -q '^1$'; then
  echo "FAIL: business_verification_queue missing partial index on submitted_at WHERE status='pending'" >&2; exit 1
fi

# listing_review_queue.decision CHECK must allow approve/reject/request_changes per spec § 5.5
for d in approve reject request_changes; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='listing_review_queue'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${d}%' and pg_get_constraintdef(c.oid) ilike '%decision%';" | grep -q '^1$'; then
    echo "FAIL: listing_review_queue.decision CHECK missing '$d'" >&2; exit 1
  fi
done

# listing_review_queue.listing_id must FK → listing(id) ON DELETE CASCADE per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='listing_review_queue'::regclass and c.contype='f' and c.confrelid='listing'::regclass and c.confdeltype='c' and 'listing_id' = any(select attname from pg_attribute where attrelid=c.conrelid and attnum=any(c.conkey));" | grep -q '^1$'; then
  echo "FAIL: listing_review_queue.listing_id missing FK to listing(id) ON DELETE CASCADE" >&2; exit 1
fi

# listing_review_queue partial index where decision is null per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='listing_review_queue' and indexdef ilike '%submitted_at%' and indexdef ilike '%where (decision is null)%';" | grep -q '^1$'; then
  echo "FAIL: listing_review_queue missing partial index on submitted_at WHERE decision IS NULL" >&2; exit 1
fi

# listing_report.reason CHECK must allow exactly the 6 spec values per spec § 5.5
for r in fake_listing wrong_price wrong_location inappropriate_content spam other; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='listing_report'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${r}%' and pg_get_constraintdef(c.oid) ilike '%reason%';" | grep -q '^1$'; then
    echo "FAIL: listing_report.reason CHECK missing '$r'" >&2; exit 1
  fi
done

# listing_report.status CHECK must allow open/investigating/confirmed/dismissed per spec § 5.5
for s in open investigating confirmed dismissed; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='listing_report'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${s}%' and pg_get_constraintdef(c.oid) ilike '%status%';" | grep -q '^1$'; then
    echo "FAIL: listing_report.status CHECK missing '$s'" >&2; exit 1
  fi
done

# listing_report.reporter_id must be nullable (anonymous reports) per spec § 5.5
REPORTER_NOTNULL=$(psql "$DATABASE_URL" -t -A -c "select attnotnull from pg_attribute where attrelid='listing_report'::regclass and attname='reporter_id';")
if [ "$REPORTER_NOTNULL" != "f" ]; then
  echo "FAIL: listing_report.reporter_id must be nullable (anonymous reports allowed)" >&2; exit 1
fi

# featured_content.target_kind CHECK must allow listing/industrial_complex/manufacturer per spec § 5.5
for tk in listing industrial_complex manufacturer; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='featured_content'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${tk}%' and pg_get_constraintdef(c.oid) ilike '%target_kind%';" | grep -q '^1$'; then
    echo "FAIL: featured_content.target_kind CHECK missing '$tk'" >&2; exit 1
  fi
done

# featured_content.feature_kind CHECK must allow homepage_featured/search_top/sponsored_marker/newsletter per spec § 5.5
for fk in homepage_featured search_top sponsored_marker newsletter; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='featured_content'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${fk}%' and pg_get_constraintdef(c.oid) ilike '%feature_kind%';" | grep -q '^1$'; then
    echo "FAIL: featured_content.feature_kind CHECK missing '$fk'" >&2; exit 1
  fi
done

# featured_content active partial index per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='featured_content' and indexdef ilike '%(feature_kind, starts_at, ends_at%';" | grep -q '^1$'; then
  echo "FAIL: featured_content missing composite index (feature_kind, starts_at, ends_at)" >&2; exit 1
fi

# system_alert.severity CHECK must allow info/warning/error/critical per spec § 5.5
for sv in info warning error critical; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='system_alert'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${sv}%' and pg_get_constraintdef(c.oid) ilike '%severity%';" | grep -q '^1$'; then
    echo "FAIL: system_alert.severity CHECK missing '$sv'" >&2; exit 1
  fi
done

# system_alert partial index on unacknowledged alerts per spec § 5.5
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='system_alert' and indexdef ilike '%(severity, created_at desc%' and indexdef ilike '%where (acknowledged_at is null)%';" | grep -q '^1$'; then
  echo "FAIL: system_alert missing partial index on (severity, created_at desc) WHERE acknowledged_at IS NULL" >&2; exit 1
fi

echo "PASS: V001_05 Operations 6 tables + queue/report/featured/alert enums + partial indexes"
exit 0
