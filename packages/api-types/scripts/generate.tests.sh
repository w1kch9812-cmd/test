#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
package_dir="$(cd "${script_dir}/.." && pwd)"
node_bin="${NODE_BIN:-}"
if [ -z "$node_bin" ]; then
  if command -v node >/dev/null 2>&1; then
    node_bin="node"
  elif command -v node.exe >/dev/null 2>&1; then
    node_bin="node.exe"
  else
    printf 'not ok - node is required\n' >&2
    exit 1
  fi
fi
node_package_dir="$package_dir"
if [[ "$node_bin" == *".exe" ]] && command -v cygpath >/dev/null 2>&1; then
  node_package_dir="$(cygpath -w "$package_dir")"
elif [[ "$node_bin" == *".exe" && "$package_dir" =~ ^/mnt/([a-zA-Z])/(.*)$ ]]; then
  drive="${BASH_REMATCH[1]}"
  rest="${BASH_REMATCH[2]}"
  node_package_dir="${drive^^}:/${rest}"
fi

set +e
output="$("$node_bin" "$node_package_dir/node_modules/tsx/dist/cli.mjs" "$node_package_dir/scripts/generate.ts" 2>&1)"
status=$?
set -e

if [ "$status" -eq 0 ]; then
  printf 'not ok - generate must fail when services/api/openapi.json is missing\n%s\n' "$output" >&2
  exit 1
fi

if [[ "$output" != *"OpenAPI spec not found"* ]]; then
  printf 'not ok - missing OpenAPI failure message should be explicit\n%s\n' "$output" >&2
  exit 1
fi

schema_file="${package_dir}/generated/schema.ts"
if grep -Eq 'Placeholder|minimal stub|"/healthz"' "$schema_file"; then
  printf 'not ok - generated schema must not contain fake placeholder paths\n' >&2
  grep -En 'Placeholder|minimal stub|"/healthz"' "$schema_file" >&2
  exit 1
fi

printf 'api-types-generate-contract-ok\n'
