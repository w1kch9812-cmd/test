#!/usr/bin/env sh
set -eu

if [ "${1:-}" != "metadata" ]; then
  echo "sqlx cargo metadata stub only supports: cargo metadata; got: $*" >&2
  exit 2
fi

workspace_root="$(pwd)"
while [ "${workspace_root}" != "/" ]; do
  if [ -f "${workspace_root}/MODULE.bazel" ] && [ -f "${workspace_root}/Cargo.toml" ]; then
    break
  fi
  workspace_root="$(dirname "${workspace_root}")"
done

if [ "${workspace_root}" = "/" ]; then
  echo "could not locate Bazel/Cargo workspace root" >&2
  exit 3
fi

target_directory="${workspace_root}/bazel-out"

printf '{'
printf '"packages":[],'
printf '"workspace_members":[],'
printf '"workspace_default_members":[],'
printf '"resolve":null,'
printf '"target_directory":"%s",' "${target_directory}"
printf '"version":1,'
printf '"workspace_root":"%s",' "${workspace_root}"
printf '"metadata":null'
printf '}\n'
