#!/usr/bin/env bash
set -euo pipefail

runfiles_root="${TEST_SRCDIR:-}/${TEST_WORKSPACE:-}"
web_archive="${1:-${runfiles_root}/gongzzang-web-next-build.tgz}"
api_binary="${2:-${runfiles_root}/gongzzang-api-release/api}"

if [ -z "$web_archive" ] || [ -z "$api_binary" ]; then
  printf 'check-release-artifacts: web archive and API binary paths are required\n' >&2
  exit 2
fi

if [ ! -s "$web_archive" ]; then
  printf 'check-release-artifacts: web archive is missing or empty: %s\n' "$web_archive" >&2
  exit 1
fi

listing="$(mktemp)"
trap 'rm -f "$listing"' EXIT
tar -tzf "$web_archive" > "$listing"

if ! grep -q '^gongzzang-web-next-build/\.next/' "$listing"; then
  printf 'check-release-artifacts: web archive must contain gongzzang-web-next-build/.next/\n' >&2
  exit 1
fi

if [ ! -s "$api_binary" ]; then
  printf 'check-release-artifacts: API binary is missing or empty: %s\n' "$api_binary" >&2
  exit 1
fi

if [ ! -x "$api_binary" ]; then
  printf 'check-release-artifacts: API binary must be executable: %s\n' "$api_binary" >&2
  exit 1
fi

printf 'release-artifacts-ok web=%s api=%s\n' "$web_archive" "$api_binary"
