#!/usr/bin/env bash
set -euo pipefail

fail=0
root="${1:-.}"
extensions=(
  "*.md"
  "*.rs"
  "*.ts"
  "*.tsx"
  "*.sql"
)

if [ ! -d "$root" ]; then
  echo "file-line-limit: root directory does not exist: ${root}" >&2
  exit 2
fi

cd "$root"

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

list_git_files() {
  git ls-files -z --cached --others --exclude-standard -- "${extensions[@]}"
}

list_find_files() {
  find . -type f \
    \( -name "*.md" -o -name "*.rs" -o -name "*.ts" -o -name "*.tsx" -o -name "*.sql" \) \
    ! -path "./.git/*" \
    ! -path "./node_modules/*" \
    ! -path "./_archive/*" \
    ! -path "./target/*" \
    ! -path "./.next/*" \
    ! -path "./.wrangler/*" \
    ! -path "./.turbo/*" \
    ! -path "./.superpowers/*" \
    ! -path "./.codex/*" \
    ! -path "./.mcp-local/*" \
    ! -path "./.playwright-mcp/*" \
    ! -path "./reference/*" \
    ! -path "./apps/web/public/dev-tiles/*" \
    ! -path "./apps/web/public/pmtiles/*" \
    ! -path "*/.venv/*" \
    ! -path "*/var/*" \
    -print0
}

list_files() {
  if command -v git >/dev/null 2>&1 && git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    list_git_files
  else
    list_find_files
  fi
}

while IFS= read -r -d '' file; do
  if [ ! -f "$file" ]; then
    continue
  fi
  lines="$(wc -l < "$file" | tr -d '[:space:]')"
  if [ "$lines" -gt 1500 ]; then
    report_error "$file" "$lines"
  elif [ "$lines" -gt 500 ]; then
    report_warning "$file" "$lines"
  fi
done < <(list_files)

if [ "$fail" -ne 0 ]; then
  exit 1
fi

echo "file-line-limit-ok max=1500"
