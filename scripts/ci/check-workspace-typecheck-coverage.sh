#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
python_bin="${PYTHON:-}"
if [ -z "$python_bin" ]; then
  if command -v python3 >/dev/null 2>&1; then
    python_bin="python3"
  elif command -v python >/dev/null 2>&1; then
    python_bin="python"
  elif command -v python.exe >/dev/null 2>&1; then
    python_bin="python.exe"
  else
    printf 'workspace-typecheck-coverage: python3 or python is required\n' >&2
    exit 1
  fi
fi

exec "$python_bin" "$script_dir/check-workspace-typecheck-coverage.py" "${1:-}"
