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

EXPECTED=(pipeline_schedule pipeline_run)
for t in "${EXPECTED[@]}"; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_tables where schemaname='public' and tablename='$t';" | grep -q '^1$'; then
    echo "FAIL: table '$t' missing" >&2; exit 1
  fi
done

# pipeline_schedule.pipeline_kind must be UNIQUE per spec § 5.4
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='pipeline_schedule'::regclass and c.contype='u' and 'pipeline_kind' = any(select attname from pg_attribute where attrelid=c.conrelid and attnum=any(c.conkey));" | grep -q '^1$'; then
  echo "FAIL: pipeline_schedule.pipeline_kind must be UNIQUE" >&2; exit 1
fi

# pipeline_schedule.timezone default must be 'Asia/Seoul' per spec § 5.4
TZ_DEFAULT=$(psql "$DATABASE_URL" -t -A -c "select column_default from information_schema.columns where table_name='pipeline_schedule' and column_name='timezone';")
if [[ "$TZ_DEFAULT" != *"Asia/Seoul"* ]]; then
  echo "FAIL: pipeline_schedule.timezone default must be 'Asia/Seoul', got '$TZ_DEFAULT'" >&2; exit 1
fi

# pipeline_schedule must have partial index on next_run_at where enabled=true per spec § 5.4
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='pipeline_schedule' and indexdef ilike '%next_run_at%' and indexdef ilike '%where (enabled = true)%';" | grep -q '^1$'; then
  echo "FAIL: pipeline_schedule missing partial index on next_run_at WHERE enabled=true" >&2; exit 1
fi

# pipeline_run.steps must be jsonb (admin UI node graph renders this) per spec § 5.4
STEPS_TYPE=$(psql "$DATABASE_URL" -t -A -c "select data_type from information_schema.columns where table_name='pipeline_run' and column_name='steps';")
if [ "$STEPS_TYPE" != "jsonb" ]; then
  echo "FAIL: pipeline_run.steps must be jsonb, got '$STEPS_TYPE'" >&2; exit 1
fi

# pipeline_run.status CHECK must allow exactly the 5 spec values per spec § 5.4
for s in running success failed skipped_unchanged aborted; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='pipeline_run'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${s}%' and pg_get_constraintdef(c.oid) ilike '%status%';" | grep -q '^1$'; then
    echo "FAIL: pipeline_run.status CHECK missing '$s'" >&2; exit 1
  fi
done

# pipeline_run.triggered_by CHECK must allow schedule/manual/event per spec § 5.4
for tb in schedule manual event; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='pipeline_run'::regclass and c.contype='c' and pg_get_constraintdef(c.oid) ilike '%${tb}%' and pg_get_constraintdef(c.oid) ilike '%triggered_by%';" | grep -q '^1$'; then
    echo "FAIL: pipeline_run.triggered_by CHECK missing '$tb'" >&2; exit 1
  fi
done

# pipeline_run.schedule_id must FK → pipeline_schedule(id) per spec § 5.4
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint c where c.conrelid='pipeline_run'::regclass and c.contype='f' and c.confrelid='pipeline_schedule'::regclass and 'schedule_id' = any(select attname from pg_attribute where attrelid=c.conrelid and attnum=any(c.conkey));" | grep -q '^1$'; then
  echo "FAIL: pipeline_run.schedule_id missing FK to pipeline_schedule(id)" >&2; exit 1
fi

# pipeline_run partial index on running status per spec § 5.4 (admin UI active runs)
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='pipeline_run' and indexdef ilike '%started_at%' and indexdef ilike '%where (status = ''running''::text)%';" | grep -q '^1$'; then
  echo "FAIL: pipeline_run missing partial index on started_at WHERE status='running'" >&2; exit 1
fi

# pipeline_run composite index on (schedule_id, started_at desc) per spec § 5.4
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_indexes where tablename='pipeline_run' and indexdef ilike '%(schedule_id, started_at desc%';" | grep -q '^1$'; then
  echo "FAIL: pipeline_run missing composite index (schedule_id, started_at desc)" >&2; exit 1
fi

echo "PASS: V001_04 Pipeline 2 tables (pipeline_schedule, pipeline_run)"
exit 0
