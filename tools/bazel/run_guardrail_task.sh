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

to_windows_path() {
  local path="$1"
  if [[ "$path" =~ ^/mnt/([A-Za-z])/(.*)$ ]]; then
    local drive="${BASH_REMATCH[1]^^}"
    local rest="${BASH_REMATCH[2]//\//\\}"
    printf '%s:\\%s' "$drive" "$rest"
    return
  fi
  printf '%s' "$path"
}

resolve_windows_powershell() {
  if command -v powershell.exe >/dev/null 2>&1; then
    command -v powershell.exe
    return
  fi
  if [ -x /mnt/c/WINDOWS/System32/WindowsPowerShell/v1.0/powershell.exe ]; then
    printf '%s\n' /mnt/c/WINDOWS/System32/WindowsPowerShell/v1.0/powershell.exe
    return
  fi
  if [ -x /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe ]; then
    printf '%s\n' /mnt/c/Windows/System32/WindowsPowerShell/v1.0/powershell.exe
    return
  fi
}

run_pwsh() {
  local script="$1"
  shift
  local windows_powershell
  windows_powershell="$(resolve_windows_powershell || true)"
  if [[ "$repo_root" =~ ^/mnt/[A-Za-z]/ ]] && [ -n "$windows_powershell" ]; then
    local script_path="$script"
    if [[ "$script_path" != /* ]]; then
      script_path="${repo_root}/${script_path}"
    fi
    local converted_args=()
    local arg
    for arg in "$@"; do
      converted_args+=("$(to_windows_path "$arg")")
    done
    exec "$windows_powershell" -NoProfile -ExecutionPolicy Bypass -File "$(to_windows_path "$script_path")" "${converted_args[@]}"
  fi
  if command -v pwsh >/dev/null 2>&1; then
    exec pwsh -NoProfile -ExecutionPolicy Bypass -File "$script" "$@"
  fi
  if [ -n "$windows_powershell" ]; then
    exec "$windows_powershell" -NoProfile -ExecutionPolicy Bypass -File "$script" "$@"
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
  verification-control-plane)
    run_pwsh scripts/ci/check-verification-control-plane.ps1 -Root "$repo_root"
    ;;
  verification-control-plane-tests)
    run_pwsh scripts/ci/check-verification-control-plane.tests.ps1
    ;;
  bazel-transition-ratchet)
    run_pwsh scripts/ci/check-bazel-transition-ratchet.ps1 -Root "$repo_root"
    ;;
  bazel-transition-ratchet-tests)
    run_pwsh scripts/ci/check-bazel-transition-ratchet.tests.ps1
    ;;
  generated-artifact-registry)
    run_pwsh scripts/ci/check-generated-artifact-registry.ps1 -Root "$repo_root"
    ;;
  generated-artifact-registry-tests)
    run_pwsh scripts/ci/check-generated-artifact-registry.tests.ps1
    ;;
  coverage-transition-ssot)
    run_pwsh scripts/ci/check-coverage-transition-ssot.ps1 -Root "$repo_root"
    ;;
  coverage-transition-ssot-tests)
    run_pwsh scripts/ci/check-coverage-transition-ssot.tests.ps1
    ;;
  verification-task-registry)
    run_pwsh scripts/ci/check-verification-task-registry.ps1 -Root "$repo_root"
    ;;
  verification-task-registry-tests)
    run_pwsh scripts/ci/check-verification-task-registry.tests.ps1
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
  lakehouse-registry-integration)
    run_pwsh scripts/ci/check-lakehouse-registry-integration.ps1 -Root "$repo_root"
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
