#!/usr/bin/env bash
# Walking-skeleton E2E: migrate, run integration tests, boot the API in mock-JWT
# mode, and assert the auth/user provisioning happy path over HTTP.
#
# Native replacement for the deleted Bazel target
# //tools/bazel:ci_walking_skeleton_e2e_transition, which only wrapped this exact
# sequence via run_ci_transition_task.sh::run_walking_skeleton_e2e.
set -euo pipefail

script_path="${BASH_SOURCE[0]}"
if command -v realpath >/dev/null 2>&1; then
  script_path="$(realpath "$script_path")"
fi
repo_root="$(cd "$(dirname "$script_path")/../.." && pwd)"
cd "$repo_root"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'walking-skeleton-e2e: required command is missing: %s\n' "$1" >&2
    exit 127
  fi
}

if [ -z "${DATABASE_URL:-}" ]; then
  printf 'walking-skeleton-e2e: DATABASE_URL is required\n' >&2
  exit 2
fi

require_command cargo
require_command curl
require_command python3
require_command psql
require_command sqlx

for _ in {1..30}; do
  if pg_isready -h localhost -p 5432 -U "${POSTGRES_USER:-gongzzang}" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

sqlx migrate run --source migrations
cargo test --workspace --features integration --no-fail-fast -- --test-threads=1

PGPASSWORD="${POSTGRES_PASSWORD:-ci_only_changeme}" psql -h localhost -U "${POSTGRES_USER:-gongzzang}" -d "${POSTGRES_DB:-gongzzang}" \
  -c 'truncate "user", listing, listing_photo cascade;'

cargo build --package api --release

api_pid=""
cleanup_api() {
  if [ -n "$api_pid" ] && kill -0 "$api_pid" 2>/dev/null; then
    kill "$api_pid"
  fi
}
trap cleanup_api EXIT

AUTH_DEV_MODE=true ./target/release/api > /tmp/api.log 2>&1 &
api_pid=$!
for index in {1..30}; do
  if curl -sf http://localhost:8080/healthz >/dev/null 2>&1; then
    printf 'API ready after %ss\n' "$index"
    break
  fi
  sleep 1
done
curl -sf http://localhost:8080/healthz >/dev/null 2>&1 || {
  printf 'walking-skeleton-e2e: API failed to start\n' >&2
  cat /tmp/api.log >&2
  exit 1
}

body="$(curl -sf http://localhost:8080/healthz)"
status="$(printf '%s' "$body" | python3 -c "import json,sys;print(json.load(sys.stdin).get('status', ''))")"
if [ "$status" != "ok" ]; then
  printf "FAIL: /healthz expected status=ok, got '%s'\n" "$body" >&2
  exit 1
fi
printf 'PASS: /healthz.status=ok\n'

status="$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/users/me)"
if [ "$status" != "401" ]; then
  printf 'FAIL: /users/me expected 401, got %s\n' "$status" >&2
  cat /tmp/api.log >&2
  exit 1
fi
printf 'PASS: /users/me without auth returns 401\n'

status="$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer bogus.token" http://localhost:8080/users/me)"
if [ "$status" != "401" ]; then
  printf 'FAIL: bad token expected 401, got %s\n' "$status" >&2
  cat /tmp/api.log >&2
  exit 1
fi
printf 'PASS: bad token returns 401\n'

token="DEV.test-user-1"
response="$(curl -sf -H "Authorization: Bearer $token" http://localhost:8080/users/me)" || {
  printf 'FAIL: GET /users/me with mock token failed\n' >&2
  cat /tmp/api.log >&2
  exit 1
}
user_id="$(printf '%s' "$response" | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])")"
zsub="$(printf '%s' "$response" | python3 -c "import json,sys;print(json.load(sys.stdin)['zitadel_sub'])")"
if [ -z "$user_id" ]; then
  printf 'FAIL: no id\n' >&2
  exit 1
fi
if [ "$zsub" != "test-user-1" ]; then
  printf 'FAIL: zitadel_sub mismatch: %s\n' "$zsub" >&2
  exit 1
fi
printf 'PASS: first sign-in id=%s sub=%s\n' "$user_id" "$zsub"

response="$(curl -sf -H "Authorization: Bearer $token" http://localhost:8080/users/me)"
user_id_2="$(printf '%s' "$response" | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])")"
if [ "$user_id_2" != "$user_id" ]; then
  printf 'FAIL: duplicate user first=%s second=%s\n' "$user_id" "$user_id_2" >&2
  exit 1
fi
row_count="$(PGPASSWORD="${POSTGRES_PASSWORD:-ci_only_changeme}" psql -h localhost -U "${POSTGRES_USER:-gongzzang}" -d "${POSTGRES_DB:-gongzzang}" -t -A -c 'select count(*) from "user";')"
if [ "$row_count" != "1" ]; then
  printf 'FAIL: expected 1 user row, got %s\n' "$row_count" >&2
  exit 1
fi
printf 'PASS: same id, 1 user row\n'

token_2="DEV.test-user-2"
response="$(curl -sf -H "Authorization: Bearer $token_2" http://localhost:8080/users/me)"
user_id_3="$(printf '%s' "$response" | python3 -c "import json,sys;print(json.load(sys.stdin)['id'])")"
if [ "$user_id_3" = "$user_id" ]; then
  printf 'FAIL: different sub should create different user\n' >&2
  exit 1
fi
row_count="$(PGPASSWORD="${POSTGRES_PASSWORD:-ci_only_changeme}" psql -h localhost -U "${POSTGRES_USER:-gongzzang}" -d "${POSTGRES_DB:-gongzzang}" -t -A -c 'select count(*) from "user";')"
if [ "$row_count" != "2" ]; then
  printf 'FAIL: expected 2 user rows, got %s\n' "$row_count" >&2
  exit 1
fi
printf 'PASS: different sub creates different user, 2 rows\n'
