#!/usr/bin/env bash
set -euo pipefail

root="${1:-.}"

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

rg_bin="$(command -v rg || command -v rg.exe || true)"

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

marker_pattern='(^|[^A-Za-z0-9])(TODO|HACK|XXX|TEMP|ALLOWED_FOR_FRONTEND_TEMP|임시|(?i:claude\.com|anthropic\.com))([^A-Za-z0-9]|$)'
mojibake_pattern='\x{FFFD}|\x{6028}|\x{F9CD}|\x{936E}|\x{BA2F}|\x{AFA9}|\x{C493}|\x{6FE1}|\x{8E30}|\x{907A}|\x{ACD7}|\x{B6AE}|\x{BEA3}|\x{5A9B}|\x{5AC4}|\x{C208}|\x{BA84}|\x{D00E}|\x{BE37}|\x{BD23}|\x{BD38}'
if [ -n "$rg_bin" ]; then
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
      --glob '*.css' \
      --glob '*.yaml' \
      --glob '*.yml' \
      --regexp "$marker_pattern" \
      --regexp "$mojibake_pattern" \
      "${rg_roots[@]}" || true
  )"
else
  python_bin="$(command -v python3 || command -v python || true)"
  if [ -z "$python_bin" ]; then
    printf 'forbidden-implementation-markers: rg or python is required\n' >&2
    exit 1
  fi
  matches="$(
    "$python_bin" - "${search_roots[@]}" <<'PY'
import os
import re
import sys

extensions = {".ts", ".tsx", ".js", ".jsx", ".rs", ".py", ".sql", ".css", ".yaml", ".yml"}
marker_re = re.compile(r"(^|[^A-Za-z0-9])(TODO|HACK|XXX|TEMP|ALLOWED_FOR_FRONTEND_TEMP|임시|(?i:claude\.com|anthropic\.com))([^A-Za-z0-9]|$)")
mojibake_re = re.compile("[\uFFFD\u6028\uF9CD\u936E\uBA2F\uAFA9\uC493\u6FE1\u8E30\u907A\uACD7\uB6AE\uBEA3\u5A9B\u5AC4\uC208\uBA84\uD00E\uBE37\uBD23\uBD38]")

for search_root in sys.argv[1:]:
    for dirpath, dirnames, filenames in os.walk(search_root):
        dirnames[:] = [
            dirname
            for dirname in dirnames
            if dirname not in {".git", ".next", "node_modules", "target"}
        ]
        for filename in filenames:
            if os.path.splitext(filename)[1] not in extensions:
                continue
            path = os.path.join(dirpath, filename)
            try:
                with open(path, "r", encoding="utf-8", errors="replace") as handle:
                    for line_number, line in enumerate(handle, start=1):
                        line = line.rstrip("\n")
                        if marker_re.search(line) or mojibake_re.search(line):
                            print(f"{path}:{line_number}:{line}")
            except OSError:
                continue
PY
  )"
fi

if [ -n "$matches" ]; then
  matches="$(printf '%s\n' "$matches" | grep -v 'RUNNER_TEMP' || true)"
fi

if [ -n "$matches" ]; then
  printf 'forbidden implementation marker found:\n%s\n' "$matches" >&2
  exit 1
fi

printf 'forbidden-implementation-markers-ok\n'
