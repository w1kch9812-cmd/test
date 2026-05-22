#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="${1:-}"
if [ -z "$root" ]; then
  root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
fi

python_bin="${PYTHON:-}"
if [ -z "$python_bin" ]; then
  if command -v python3 >/dev/null 2>&1; then
    python_bin="python3"
  elif command -v python >/dev/null 2>&1; then
    python_bin="python"
  else
    printf 'markdown-links: python3 or python is required\n' >&2
    exit 1
  fi
fi

exec "$python_bin" "$script_dir/check-markdown-links.py" "$root"
