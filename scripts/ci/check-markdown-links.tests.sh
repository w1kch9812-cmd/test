#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="${script_dir}/check-markdown-links.sh"

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
  mkdir -p "$tmp_root/docs"
}

assert_success_contains() {
  local name="$1"
  local expected="$2"
  shift 2
  local output
  if ! output="$("$@" 2>&1)"; then
    printf 'not ok - %s\n%s\n' "$name" "$output" >&2
    exit 1
  fi
  if [[ "$output" != *"$expected"* ]]; then
    printf 'not ok - %s\nexpected output containing: %s\nactual output:\n%s\n' "$name" "$expected" "$output" >&2
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
cat > "$tmp_root/docs/good.md" <<'MD'
# Good

[root](../README.md)
[self line](good.md#L1)
[sibling repo](../../platform-core/docs/adr/example.md)
[external](https://example.invalid/no-network)
[email](mailto:docs@example.com)
`[example syntax](missing.md)`
MD
cat > "$tmp_root/README.md" <<'MD'
# Root

[docs](docs/good.md)
MD
mkdir -p "$tmp_root/docs/superpowers/plans"
cat > "$tmp_root/docs/superpowers/plans/archive.md" <<'MD'
# Historical plan

[archival broken link](missing.md)
MD
assert_success_contains "checks internal links without network" "markdown-links-ok files=2 links=3" bash "$script" "$tmp_root"

reset_tmp_root
cat > "$tmp_root/docs/broken.md" <<'MD'
# Broken

[missing](missing.md)
MD
assert_failure_contains "fails on missing internal relative link" "broken markdown link: docs/broken.md -> missing.md" bash "$script" "$tmp_root"
