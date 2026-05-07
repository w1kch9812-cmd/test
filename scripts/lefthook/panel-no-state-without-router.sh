#!/usr/bin/env bash
# SP10 spec § 5.4: zustand store 가 panelStack 보유 금지 — URL 이 SSOT.
# 위반 시 commit 차단.
set -euo pipefail

staged=$(git diff --cached --name-only --diff-filter=ACM | grep -E '^apps/web/stores/.*\.(ts|tsx)$' || true)
if [ -z "$staged" ]; then
  exit 0
fi

# shellcheck disable=SC2086
bad=$(echo "$staged" | xargs -r grep -lE "panelStack" || true)
if [ -n "$bad" ]; then
  echo "ERROR: zustand store must not hold panelStack — URL is SSOT (spec § 5.4)"
  echo "  offending: $bad"
  exit 1
fi
exit 0
