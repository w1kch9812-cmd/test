#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
script="${script_dir}/check-workspace-typecheck-coverage.py"
python_bin="${PYTHON:-}"
if [ -z "$python_bin" ]; then
  if command -v python3 >/dev/null 2>&1; then
    python_bin="python3"
  elif command -v python >/dev/null 2>&1; then
    python_bin="python"
  elif command -v python.exe >/dev/null 2>&1; then
    python_bin="python.exe"
  else
    printf 'python is required\n' >&2
    exit 1
  fi
fi

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
  mkdir -p "$tmp_root/apps/web" "$tmp_root/packages/ui"
  cat > "$tmp_root/pnpm-workspace.yaml" <<'YAML'
packages:
  - "apps/*"
  - "packages/*"
YAML
}

write_package_json() {
  local path="$1"
  local scripts="$2"
  mkdir -p "$(dirname "$path")"
  cat > "$path" <<JSON
{
  "name": "fixture",
  "private": true,
  "scripts": $scripts
}
JSON
}

write_build_bazel() {
  local path="$1"
  local body="$2"
  mkdir -p "$(dirname "$path")"
  cat > "$path" <<BAZEL
$body
BAZEL
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
write_package_json "$tmp_root/apps/web/package.json" '{"typecheck": "tsc --noEmit"}'
write_package_json "$tmp_root/packages/ui/package.json" '{"typecheck": "tsc --noEmit"}'
write_build_bazel "$tmp_root/apps/web/BUILD.bazel" 'test_suite(name = "typecheck")'
write_build_bazel "$tmp_root/packages/ui/BUILD.bazel" 'test_suite(name = "typecheck")'
assert_success_contains "accepts full workspace typecheck coverage" "workspace-typecheck-coverage-ok packages=2" "$python_bin" "$script" "$tmp_root"

reset_tmp_root
write_package_json "$tmp_root/apps/web/package.json" '{"typecheck": "tsc --noEmit"}'
write_package_json "$tmp_root/packages/ui/package.json" '{}'
write_build_bazel "$tmp_root/apps/web/BUILD.bazel" 'test_suite(name = "typecheck")'
assert_failure_contains "rejects package missing typecheck script" "missing typecheck script: packages/ui/package.json" "$python_bin" "$script" "$tmp_root"

reset_tmp_root
write_package_json "$tmp_root/apps/web/package.json" '{"typecheck": "tsc --noEmit"}'
write_package_json "$tmp_root/packages/ui/package.json" '{"typecheck": "tsc --noEmit"}'
write_build_bazel "$tmp_root/apps/web/BUILD.bazel" 'test_suite(name = "typecheck")'
assert_failure_contains "rejects package missing Bazel typecheck target" "missing Bazel typecheck target: packages/ui/BUILD.bazel:typecheck" "$python_bin" "$script" "$tmp_root"
