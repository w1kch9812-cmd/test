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
line_number=0

while IFS= read -r line || [ -n "$line" ]; do
  line_number=$((line_number + 1))
  trimmed="${line#"${line%%[![:space:]]*}"}"

  case "$trimmed" in
    ""|\#*) continue ;;
  esac

  if [[ "$line" == *"|| echo"* ]]; then
    printf 'lefthook-no-fake-pass: fake-pass fallback in lefthook.yml:%s: %s\n' "$line_number" "$line" >&2
    status=1
  fi

  if [[ "$line" == *"CI enforces"* ]]; then
    printf 'lefthook-no-fake-pass: CI enforces skip wording in lefthook.yml:%s: %s\n' "$line_number" "$line" >&2
    status=1
  fi

  if [[ "$line" == *"not installed"* && "$line" == *"run:"* ]]; then
    printf 'lefthook-no-fake-pass: tool absence skip wording in lefthook.yml:%s: %s\n' "$line_number" "$line" >&2
    status=1
  fi
done < "$lefthook_file"

if [ "$status" -ne 0 ]; then
  exit "$status"
fi

printf 'lefthook-no-fake-pass-ok\n'
