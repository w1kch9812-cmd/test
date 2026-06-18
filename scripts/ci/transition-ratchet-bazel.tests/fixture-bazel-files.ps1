    $guardrailNoCacheTag = if ($MissingGuardrailNoCacheTag) { "" } else { '    "no-cache",' }
    $guardrailExternalTag = if ($MissingGuardrailExternalTag) { "" } else { '    "external",' }
    Write-File -Root $Root -RelativePath "tools\bazel\BUILD.bazel" -Content @"
load("//tools/bazel:shell_test_compat.bzl", "transition_shell_test")

GUARDRAIL_TRANSITION_TAGS = [
    "manual",
    "local",
    "no-sandbox",
$guardrailNoCacheTag
$guardrailExternalTag
]

transition_shell_test(
    name = "ci_node_audit_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["node-audit"],
)

transition_shell_test(
    name = "ci_rust_check_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["rust-check"],
)

transition_shell_test(
    name = "ci_rustfmt_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["rustfmt-check"],
)

transition_shell_test(
    name = "frontend_e2e_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["frontend-e2e"],
)

transition_shell_test(
    name = "ci_migration_v001_full_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["migration-v001-full"],
)
"@

    $rustVerificationTarget = if ($AvailableMissingExitTarget) {
        ""
    } else {
        @'

test_suite(
    name = "rust_verification",
    tests = [],
)
'@
    }
    Write-File -Root $Root -RelativePath "BUILD.bazel" -Content @"
test_suite(
    name = "verify_supply_chain",
    tests = [],
)
$rustVerificationTarget
"@

    $runnerPsqlGuard = if ($MissingRunnerCommandGuard) { "" } else { "  require_command psql" }
    $runnerPostgresGuard = if ($MissingRunnerServiceGuard) { "" } else { "  wait_for_postgres" }
    $runnerRustCheckCase = if ($MissingRunnerTaskCase) {
        ""
    } else {
        @'
  rust-check)
    run_rust_check
    ;;
'@
    }
    Write-File -Root $Root -RelativePath "tools\bazel\run_ci_transition_task.sh" -Content @"
#!/usr/bin/env bash
set -euo pipefail

task="`${1:-}"

require_command() {
  command -v "`$1" >/dev/null 2>&1
}

wait_for_postgres() {
  require_command pg_isready
}

run_node_audit() {
  require_command pnpm
}

run_rust_check() {
  require_command cargo
}

run_rustfmt_check() {
  require_command cargo
}

run_frontend_e2e() {
  require_command pnpm
}

run_migration_v001_full() {
  require_command sqlx
$runnerPsqlGuard
$runnerPostgresGuard
}

case "`$task" in
  node-audit)
    run_node_audit
    ;;
$runnerRustCheckCase
  rustfmt-check)
    run_rustfmt_check
    ;;
  frontend-e2e)
    run_frontend_e2e
    ;;
  migration-v001-full)
    run_migration_v001_full
    ;;
  *)
    exit 2
    ;;
esac
"@
