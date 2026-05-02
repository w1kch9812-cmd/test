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

EXPECTED=(audit_log outbox_event)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done

# audit_log must have BRIN index on created_at per spec § 5.3
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='audit_log' and indexdef ilike '%using brin%' and indexdef ilike '%created_at%';" | grep -q '^1$'; then
  echo "FAIL: audit_log missing BRIN index on created_at" >&2; exit 1
fi

# audit_log actor partial index (where actor_id is not null) per spec § 5.3
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='audit_log' and indexdef ilike '%actor_id%' and indexdef ilike '%where (actor_id is not null)%';" | grep -q '^1$'; then
  echo "FAIL: audit_log missing partial index on actor_id IS NOT NULL" >&2; exit 1
fi

# audit_log must have exactly 3 indexes (created_brin, resource, actor) per spec § 5.3
AUDIT_IDX_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where tablename='audit_log' and indexname like 'audit_log_%_idx';")
if [ "$AUDIT_IDX_COUNT" != "3" ]; then
  echo "FAIL: audit_log expected 3 secondary indexes, got '$AUDIT_IDX_COUNT'" >&2; exit 1
fi

# outbox_event must have partial index for unpublished events (where published_at is null) per spec § 5.3
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='outbox_event' and indexdef ilike '%where (published_at is null)%';" | grep -q '^1$'; then
  echo "FAIL: outbox_event missing partial index on published_at IS NULL" >&2; exit 1
fi

# outbox_event payload must be NOT NULL per spec § 5.3
PAYLOAD_NOTNULL=$(psql "$DATABASE_URL" -t -A -c "select attnotnull from pg_attribute where attrelid='outbox_event'::regclass and attname='payload';")
if [ "$PAYLOAD_NOTNULL" != "t" ]; then
  echo "FAIL: outbox_event.payload must be NOT NULL" >&2; exit 1
fi

echo "PASS: V001_03 System 2 tables (audit_log, outbox_event)"
exit 0
