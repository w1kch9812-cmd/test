#!/usr/bin/env bash
set -euo pipefail

task="${1:-}"
if [ -z "$task" ]; then
  printf 'run-guardrail-task: task argument is required\n' >&2
  exit 2
fi

script_path="${BASH_SOURCE[0]}"
if command -v realpath >/dev/null 2>&1; then
  script_path="$(realpath "$script_path")"
fi
repo_root="$(cd "$(dirname "$script_path")/../.." && pwd)"

cd "$repo_root"

run_pwsh() {
  local script="$1"
  shift
  if command -v pwsh >/dev/null 2>&1; then
    exec pwsh -NoProfile -ExecutionPolicy Bypass -File "$script" "$@"
  fi
  if command -v powershell.exe >/dev/null 2>&1; then
    exec powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$script" "$@"
  fi
  printf 'run-guardrail-task: PowerShell is required for %s\n' "$script" >&2
  exit 127
}

case "$task" in
  no-fake-pass)
    exec bash scripts/lefthook/check-no-fake-pass.sh "$repo_root"
    ;;
  forbidden-implementation-markers)
    exec bash scripts/lefthook/check-forbidden-implementation-markers.sh "$repo_root"
    ;;
  file-line-limit)
    exec bash scripts/lefthook/file-line-limit.sh "$repo_root"
    ;;
  catalog-m1-boundary)
    exec bash scripts/lefthook/catalog-m1-boundary.sh
    ;;
  markdown-links)
    exec bash scripts/ci/check-markdown-links.sh "$repo_root"
    ;;
  workspace-typecheck-coverage)
    exec bash scripts/ci/check-workspace-typecheck-coverage.sh "$repo_root"
    ;;
  platform-core-boundary)
    run_pwsh scripts/ci/check-platform-core-boundary.ps1 -Root "$repo_root"
    ;;
  platform-core-dependency-boundary)
    run_pwsh scripts/ci/check-platform-core-dependency-boundary.ps1 -Root "$repo_root"
    ;;
  platform-core-catalog-api-contract)
    run_pwsh scripts/ci/check-platform-core-catalog-api-contract.ps1 -Root "$repo_root"
    ;;
  platform-core-event-receiver-contract)
    run_pwsh scripts/ci/check-platform-core-event-receiver-contract.ps1 -Root "$repo_root"
    ;;
  pnu-anchor-pbf-marker-contract)
    run_pwsh scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1 -Root "$repo_root"
    ;;
  migration-version-prefixes)
    run_pwsh scripts/ci/check-migration-version-prefixes.ps1 -Root "$repo_root"
    ;;
  platform-core-anchor-inbox-db-approval)
    run_pwsh scripts/ci/check-platform-core-anchor-inbox-db-approval.ps1 -Root "$repo_root"
    ;;
  platform-integration-policy)
    run_pwsh scripts/ci/check-platform-integration-policy.ps1 -Root "$repo_root"
    ;;
  traffic-auth-policy-registry)
    run_pwsh scripts/ci/check-traffic-auth-policy-registry.ps1 -Root "$repo_root"
    ;;
  traffic-auth-policy-registry-tests)
    run_pwsh scripts/ci/check-traffic-auth-policy-registry.tests.ps1
    ;;
  traffic-auth-api-control-plane-tests)
    run_pwsh scripts/ci/check-traffic-auth-api-control-plane.tests.ps1
    ;;
  github-actions-node-runtime)
    run_pwsh scripts/ci/check-github-actions-node-runtime.ps1 -Root "$repo_root"
    ;;
  github-actions-node-runtime-tests)
    run_pwsh scripts/ci/check-github-actions-node-runtime.tests.ps1
    ;;
  load-test-assets)
    run_pwsh scripts/ci/check-load-test-assets.ps1 -Root "$repo_root"
    ;;
  load-test-assets-tests)
    run_pwsh scripts/ci/check-load-test-assets.tests.ps1
    ;;
  *)
    printf 'run-guardrail-task: unknown task: %s\n' "$task" >&2
    exit 2
    ;;
esac
