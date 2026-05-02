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

# 1. All 3 roles exist
for r in gongzzang_app_writer gongzzang_app_reader gongzzang_audit_archiver; do
  if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_roles where rolname='$r';" | grep -q '^1$'; then
    echo "FAIL: role $r missing" >&2; exit 1
  fi
done

# 2. writer cannot UPDATE/DELETE audit_log (revoked)
WRITER_UPD=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_writer', 'audit_log', 'UPDATE');")
if [ "$WRITER_UPD" != "f" ]; then
  echo "FAIL: writer has UPDATE on audit_log (expected revoked)" >&2; exit 1
fi
WRITER_DEL=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_writer', 'audit_log', 'DELETE');")
if [ "$WRITER_DEL" != "f" ]; then
  echo "FAIL: writer has DELETE on audit_log (expected revoked)" >&2; exit 1
fi

# 3. writer can INSERT audit_log
WRITER_INS=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_writer', 'audit_log', 'INSERT');")
if [ "$WRITER_INS" != "t" ]; then
  echo "FAIL: writer cannot INSERT audit_log (expected granted)" >&2; exit 1
fi

# 3b. writer can SELECT audit_log
WRITER_SEL=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_writer', 'audit_log', 'SELECT');")
if [ "$WRITER_SEL" != "t" ]; then
  echo "FAIL: writer cannot SELECT audit_log (expected granted)" >&2; exit 1
fi

# 3c. reader has SELECT only on audit_log
READER_SEL=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_reader', 'audit_log', 'SELECT');")
if [ "$READER_SEL" != "t" ]; then
  echo "FAIL: reader cannot SELECT audit_log (expected granted)" >&2; exit 1
fi
READER_INS=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_app_reader', 'audit_log', 'INSERT');")
if [ "$READER_INS" != "f" ]; then
  echo "FAIL: reader has INSERT on audit_log (expected denied)" >&2; exit 1
fi

# 4. audit_archiver has SELECT + DELETE on audit_log
ARC_SEL=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_audit_archiver', 'audit_log', 'SELECT');")
if [ "$ARC_SEL" != "t" ]; then
  echo "FAIL: audit_archiver missing SELECT on audit_log" >&2; exit 1
fi
ARC_DEL=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_audit_archiver', 'audit_log', 'DELETE');")
if [ "$ARC_DEL" != "t" ]; then
  echo "FAIL: audit_archiver missing DELETE on audit_log" >&2; exit 1
fi
ARC_INS=$(psql "$DATABASE_URL" -t -A -c "select has_table_privilege('gongzzang_audit_archiver', 'audit_log', 'INSERT');")
if [ "$ARC_INS" != "f" ]; then
  echo "FAIL: audit_archiver has INSERT on audit_log (expected denied)" >&2; exit 1
fi

# 5. Trigger function + 2 triggers exist
if ! psql "$DATABASE_URL" -t -A -c "select 1 from pg_proc where proname='reject_audit_mutation';" | grep -q '^1$'; then
  echo "FAIL: function reject_audit_mutation() missing" >&2; exit 1
fi
TRIG_COUNT=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_trigger where tgrelid='audit_log'::regclass and tgname like 'trg_audit_no_%';")
if [ "$TRIG_COUNT" != "2" ]; then
  echo "FAIL: expected 2 triggers (no_update, no_delete), got $TRIG_COUNT" >&2; exit 1
fi

# 6. Trigger blocks UPDATE for current connected user (when !=audit_archiver)
# Insert a test row first (as connected user, who is NOT audit_archiver)
psql "$DATABASE_URL" -c "insert into audit_log(id, action, resource_kind, resource_id, correlation_id) values('aud_test12345678901234567890ABC', 'test', 'test', 'r1', 'corr_test123456789');"

# Try UPDATE — should fail with SQLSTATE 45000 (i18n-stable; message text may change)
UPDATE_OUTPUT=$(psql "$DATABASE_URL" -v ON_ERROR_STOP=0 -c "update audit_log set action='hacked' where id='aud_test12345678901234567890ABC';" 2>&1 || true)
if ! echo "$UPDATE_OUTPUT" | grep -qE 'SQLSTATE 45000|is immutable'; then
  echo "FAIL: UPDATE on audit_log did not raise SQLSTATE 45000. Output: $UPDATE_OUTPUT" >&2
  exit 1
fi

# Try DELETE — should also fail
DELETE_OUTPUT=$(psql "$DATABASE_URL" -v ON_ERROR_STOP=0 -c "delete from audit_log where id='aud_test12345678901234567890ABC';" 2>&1 || true)
if ! echo "$DELETE_OUTPUT" | grep -qE 'SQLSTATE 45000|is immutable'; then
  echo "FAIL: DELETE on audit_log did not raise SQLSTATE 45000. Output: $DELETE_OUTPUT" >&2
  exit 1
fi

# 7. SET ROLE audit_archiver THEN delete — should succeed
psql "$DATABASE_URL" -c "set role gongzzang_audit_archiver; delete from audit_log where id='aud_test12345678901234567890ABC'; reset role;"
REMAINING=$(psql "$DATABASE_URL" -t -A -c "select count(*) from audit_log where id='aud_test12345678901234567890ABC';")
if [ "$REMAINING" != "0" ]; then
  echo "FAIL: audit_archiver could not DELETE audit_log row (count=$REMAINING)" >&2; exit 1
fi

echo "PASS: V002 — 3 roles + audit_log immutable (writer blocked, archiver allowed)"
exit 0
