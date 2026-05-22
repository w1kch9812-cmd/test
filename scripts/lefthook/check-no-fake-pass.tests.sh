#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="${script_dir}/check-no-fake-pass.sh"

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

write_lefthook() {
  mkdir -p "$tmp_root"
  cat > "${tmp_root}/lefthook.yml"
}

write_workflow() {
  mkdir -p "${tmp_root}/.github/workflows"
  cat > "${tmp_root}/.github/workflows/ci.yml"
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
write_lefthook <<'YAML'
pre-push:
  commands:
    cargo-check:
      run: cargo check --workspace --all-features
YAML
assert_success "allows strict commands" bash "$script" "$tmp_root"

reset_tmp_root
write_lefthook <<'YAML'
pre-push:
  commands:
    cargo-check:
      run: command -v cargo >/dev/null 2>&1 && cargo check --workspace --all-features || echo "cargo not installed - CI enforces"
YAML
assert_failure_contains "rejects echo fallback" "fake-pass fallback" bash "$script" "$tmp_root"

reset_tmp_root
write_lefthook <<'YAML'
pre-commit:
  commands:
    gitleaks:
      run: gitleaks protect --staged --redact -v || echo "gitleaks not installed locally - CI enforces"
YAML
assert_failure_contains "rejects ci-enforces skip wording" "CI enforces" bash "$script" "$tmp_root"

reset_tmp_root
write_lefthook <<'YAML'
pre-push:
  commands:
    cargo-check:
      run: cargo check --workspace --all-features
YAML
write_workflow <<'YAML'
name: CI
jobs:
  fake-pass:
    runs-on: ubuntu-22.04
    steps:
      - run: cargo check --workspace || echo "cargo not installed locally"
YAML
assert_failure_contains "rejects workflow echo fallback" "fake-pass fallback" bash "$script" "$tmp_root"
