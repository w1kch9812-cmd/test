#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="${script_dir}/file-line-limit.sh"

tmp_root=""

cleanup() {
  if [ -n "$tmp_root" ] && [ -d "$tmp_root" ]; then
    rm -rf "$tmp_root"
  fi
}
trap cleanup EXIT

reset_tmp_root() {
  cleanup
  tmp_root="$(mktemp -d)"
}

write_lines() {
  local path="$1"
  local lines="$2"
  mkdir -p "$(dirname "$path")"
  : > "$path"
  for _ in $(seq 1 "$lines"); do
    printf 'line\n' >> "$path"
  done
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
write_lines "${tmp_root}/docs/superpowers/plans/oversized.md" 1501
assert_failure_contains "scans superpowers plan files" "oversized.md has 1501 lines" bash "$script" "$tmp_root"

reset_tmp_root
write_lines "${tmp_root}/docs/superpowers/plans/at-limit.md" 1500
assert_success "allows files at the hard limit" bash "$script" "$tmp_root"

reset_tmp_root
write_lines "${tmp_root}/services/scraper-py/.venv/vendor.md" 1501
write_lines "${tmp_root}/apps/web/var/logs/browser-smoke.md" 1501
assert_success "ignores local virtualenv and runtime artifacts" bash "$script" "$tmp_root"

reset_tmp_root
git -C "$tmp_root" init -q
printf '.wrangler/\n' > "${tmp_root}/.gitignore"
write_lines "${tmp_root}/.wrangler/state/generated.ts" 1501
write_lines "${tmp_root}/apps/web/lib/runtime.ts" 20
assert_success "uses git ignore rules for local generated artifacts" bash "$script" "$tmp_root"

reset_tmp_root
git -C "$tmp_root" init -q
write_lines "${tmp_root}/apps/web/lib/oversized.ts" 1501
assert_failure_contains "checks untracked files in git worktrees" "oversized.ts has 1501 lines" bash "$script" "$tmp_root"
