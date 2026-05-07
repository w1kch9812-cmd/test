#!/usr/bin/env bash
# SP10 spec § 5.2: apps/web/lib/panel/codec.ts 외부에서 ad-hoc split('>') 금지.
# URL grammar 우회 방지. 위반 시 commit 차단.
set -euo pipefail

staged=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/.*\.(ts|tsx)$' | grep -v '^apps/web/lib/panel/codec\.' || true)
if [ -z "$staged" ]; then
  exit 0
fi

# Pattern: split('>') or split(">"). Use grep -F-friendly alternation via -E.
# shellcheck disable=SC2086
bad=$(echo "$staged" | xargs -r grep -lE 'split\(("\>"|'"'"'>'"'"')\)' || true)
if [ -n "$bad" ]; then
  echo "ERROR: ad-hoc split('>') outside lib/panel/codec.ts (spec § 5.2)"
  echo "  offending: $bad"
  exit 1
fi
exit 0
