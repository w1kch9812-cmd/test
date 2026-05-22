#!/usr/bin/env bash
set -euo pipefail

root="${1:-}"
if [ -z "$root" ]; then
  root="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
fi

lefthook_file="${root}/lefthook.yml"
if [ ! -f "$lefthook_file" ]; then
  printf 'lefthook-no-fake-pass: missing %s\n' "$lefthook_file" >&2
  exit 1
fi

status=0

scan_file() {
  local file="$1"
  local label="${file#"$root"/}"
  local line_number=0

  while IFS= read -r line || [ -n "$line" ]; do
    line_number=$((line_number + 1))
    trimmed="${line#"${line%%[![:space:]]*}"}"

    case "$trimmed" in
      ""|\#*) continue ;;
    esac

    if [[ "$line" == *"|| echo"* ]]; then
      printf 'lefthook-no-fake-pass: fake-pass fallback in %s:%s: %s\n' "$label" "$line_number" "$line" >&2
      status=1
    fi

    if [[ "$line" == *"CI enforces"* ]]; then
      printf 'lefthook-no-fake-pass: CI enforces skip wording in %s:%s: %s\n' "$label" "$line_number" "$line" >&2
      status=1
    fi

    if [[ "$line" == *"not installed"* && "$line" == *"run:"* ]]; then
      printf 'lefthook-no-fake-pass: tool absence skip wording in %s:%s: %s\n' "$label" "$line_number" "$line" >&2
      status=1
    fi
  done < "$file"
}

scan_file "$lefthook_file"

shopt -s nullglob
workflow_files=("${root}"/.github/workflows/*.yml "${root}"/.github/workflows/*.yaml)
shopt -u nullglob

for workflow_file in "${workflow_files[@]}"; do
  scan_file "$workflow_file"
done

if [ "$status" -ne 0 ]; then
  exit "$status"
fi

printf 'lefthook-no-fake-pass-ok\n'
