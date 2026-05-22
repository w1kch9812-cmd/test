#!/usr/bin/env bash
set -euo pipefail

fail=0

report_error() {
  local file="$1"
  local lines="$2"
  if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
    echo "::error file=${file}::${lines} lines (> 1500). Split the file into focused parts."
  else
    echo "file-line-limit: ${file} has ${lines} lines (> 1500). Split the file into focused parts." >&2
  fi
  fail=1
}

report_warning() {
  local file="$1"
  local lines="$2"
  if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
    echo "::warning file=${file}::${lines} lines (> 500). Consider splitting."
  else
    echo "file-line-limit: ${file} has ${lines} lines (> 500). Consider splitting." >&2
  fi
}

while IFS= read -r -d '' file; do
  lines="$(wc -l < "$file" | tr -d '[:space:]')"
  if [ "$lines" -gt 1500 ]; then
    report_error "$file" "$lines"
  elif [ "$lines" -gt 500 ]; then
    report_warning "$file" "$lines"
  fi
done < <(
  find . -type f \
    \( -name "*.md" -o -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.sql" \) \
    ! -path "./.git/*" \
    ! -path "./node_modules/*" \
    ! -path "./_archive/*" \
    ! -path "./target/*" \
    ! -path "./.next/*" \
    ! -path "./reference/*" \
    ! -path "*/.venv/*" \
    ! -path "*/var/*" \
    -print0
)

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "file-line-limit-ok max=1500"
