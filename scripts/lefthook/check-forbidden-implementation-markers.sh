#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"

rg_bin="$(command -v rg || command -v rg.exe || true)"
if [ -z "$rg_bin" ]; then
  printf 'forbidden-implementation-markers: rg is required\n' >&2
  exit 1
fi

search_roots=()
for dir in .github apps services crates packages; do
  if [ -d "${root}/${dir}" ]; then
    search_roots+=("${root}/${dir}")
  fi
done

if [ "${#search_roots[@]}" -eq 0 ]; then
  printf 'forbidden-implementation-markers-ok files=0\n'
  exit 0
fi

rg_roots=()
for search_root in "${search_roots[@]}"; do
  if [[ "$rg_bin" == *.exe ]] && command -v cygpath >/dev/null 2>&1; then
    rg_roots+=("$(cygpath -w "$search_root")")
  elif [[ "$rg_bin" == *.exe && "$search_root" =~ ^/mnt/([A-Za-z])/(.*)$ ]]; then
    drive="${BASH_REMATCH[1]}"
    rest="${BASH_REMATCH[2]//\//\\}"
    rg_roots+=("${drive}:\\${rest}")
  else
    rg_roots+=("$search_root")
  fi
done

marker_pattern='(^|[^A-Za-z0-9])(TODO|HACK|XXX|TEMP|ALLOWED_FOR_FRONTEND_TEMP)([^A-Za-z0-9]|$)'
mojibake_pattern='\x{FFFD}|\x{6028}|\x{F9CD}|\x{936E}|\x{BA2F}|\x{AFA9}|\x{C493}|\x{6FE1}|\x{8E30}|\x{907A}|\x{ACD7}|\x{B6AE}|\x{BEA3}|\x{5A9B}|\x{5AC4}|\x{C208}|\x{BA84}|\x{D00E}|\x{BE37}|\x{BD23}|\x{BD38}'
matches="$(
  "$rg_bin" \
    --line-number \
    --no-heading \
    --color never \
    --glob '*.ts' \
    --glob '*.tsx' \
    --glob '*.js' \
    --glob '*.jsx' \
    --glob '*.rs' \
    --glob '*.py' \
    --glob '*.sql' \
    --glob '*.yaml' \
    --glob '*.yml' \
    --regexp "$marker_pattern" \
    --regexp "$mojibake_pattern" \
    "${rg_roots[@]}" || true
)"

if [ -n "$matches" ]; then
  matches="$(printf '%s\n' "$matches" | grep -v 'RUNNER_TEMP' || true)"
fi

if [ -n "$matches" ]; then
  printf 'forbidden implementation marker found:\n%s\n' "$matches" >&2
  exit 1
fi

printf 'forbidden-implementation-markers-ok\n'
