#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="${script_dir}/check-forbidden-implementation-markers.sh"
repo_root="$(cd "${script_dir}/../.." && pwd)"
tmp_parent="${repo_root}/target/check-forbidden-implementation-markers-tests"

tmp_root=""

cleanup() {
  if [ -n "$tmp_root" ] && [ -d "$tmp_root" ]; then
    rm -rf "$tmp_root"
  fi
}
trap cleanup EXIT

reset_tmp_root() {
  cleanup
  mkdir -p "$tmp_parent"
  tmp_root="$(mktemp -d "${tmp_parent}/run.XXXXXX")"
}

write_file() {
  local relative_path="$1"
  mkdir -p "$(dirname "${tmp_root}/${relative_path}")"
  cat > "${tmp_root}/${relative_path}"
}

assert_success() {
  local name="$1"
  shift
  local output
  if ! output="$("$@" 2>&1)"; then
    printf 'not ok - %s\n%s\n' "$name" "$output" >&2
    exit 1
  fi
  printf 'ok - %s\n' "$name"
}

assert_failure_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local output
  set +e
  output="$("$@" 2>&1)"
  local status=$?
  set -e
  if [ "$status" -eq 0 ]; then
    printf 'not ok - %s\nexpected failure containing: %s\nactual output:\n%s\n' "$name" "$expected" "$output" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected"* ]]; then
    printf 'not ok - %s\nexpected output containing: %s\nactual output:\n%s\n' "$name" "$expected" "$output" >&2
    exit 1
  fi
  printf 'ok - %s\n' "$name"
}

reset_tmp_root
write_file "apps/web/lib/session/cookie.ts" <<'TS'
export const AUTH_STATE_COOKIE_NAME = "auth-state";
export const MAX_ATTEMPTS = 3;
TS
assert_success "allows explicit names and words containing attempts" bash "$script" "$tmp_root"

reset_tmp_root
write_file "apps/web/lib/session/cookie.ts" <<'TS'
export const TEMP_COOKIE_NAME = "auth-tmp";
TS
assert_failure_contains "rejects TEMP identifiers" "forbidden implementation marker" bash "$script" "$tmp_root"

reset_tmp_root
write_file "services/api/src/startup.rs" <<'RS'
// HACK: bypass production startup.
fn main() {}
RS
assert_failure_contains "rejects HACK comments" "forbidden implementation marker" bash "$script" "$tmp_root"
