#!/usr/bin/env bash
set -euo pipefail

workspace="${TEST_WORKSPACE:-_main}"
runfiles_root="${TEST_SRCDIR:-}"
repo_root=""

if [ -n "$runfiles_root" ] && [ -d "${runfiles_root}/${workspace}" ]; then
  repo_root="${runfiles_root}/${workspace}"
elif [ -d ".sqlx" ]; then
  repo_root="$PWD"
else
  printf 'check-sqlx-offline-data: unable to locate runfiles workspace\n' >&2
  exit 2
fi

sqlx_dir="${repo_root}/.sqlx"
if [ ! -d "$sqlx_dir" ]; then
  printf 'check-sqlx-offline-data: missing .sqlx directory in Bazel runfiles\n' >&2
  exit 1
fi

shopt -s nullglob
metadata_files=("${sqlx_dir}"/query-*.json)
if [ "${#metadata_files[@]}" -eq 0 ]; then
  printf 'check-sqlx-offline-data: expected at least one .sqlx/query-*.json file\n' >&2
  exit 1
fi

python3 - "$sqlx_dir" "${metadata_files[@]}" <<'PY'
import json
import re
import sys
from pathlib import Path

sqlx_dir = Path(sys.argv[1])
paths = [Path(path) for path in sys.argv[2:]]
name_pattern = re.compile(r"^query-[0-9a-f]{64}\.json$")

for path in paths:
    if path.parent != sqlx_dir:
        raise SystemExit(f"metadata outside .sqlx directory: {path}")
    if not name_pattern.match(path.name):
        raise SystemExit(f"invalid sqlx metadata filename: {path.name}")

    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid json in {path.name}: {exc}") from exc

    if payload.get("db_name") != "PostgreSQL":
        raise SystemExit(f"{path.name}: db_name must be PostgreSQL")
    if not isinstance(payload.get("query"), str) or not payload["query"].strip():
        raise SystemExit(f"{path.name}: query must be a non-empty string")
    describe = payload.get("describe")
    if not isinstance(describe, dict):
        raise SystemExit(f"{path.name}: describe must be an object")
    columns = describe.get("columns")
    if not isinstance(columns, list):
        raise SystemExit(f"{path.name}: describe.columns must be an array")
    nullable = describe.get("nullable")
    if not isinstance(nullable, list):
        raise SystemExit(f"{path.name}: describe.nullable must be an array")
    parameters = describe.get("parameters")
    if not isinstance(parameters, dict):
        raise SystemExit(f"{path.name}: describe.parameters must be an object")

print(f"sqlx-offline-data-ok files={len(paths)}")
PY
