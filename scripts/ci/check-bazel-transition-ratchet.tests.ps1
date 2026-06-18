Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-bazel-transition-ratchet.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-bazel-transition-ratchet-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    $output = & $PowerShellExe @arguments 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitNodeAuditPolicy,
        [switch] $AddStalePolicy,
        [switch] $ExpiredSunset,
        [switch] $MissingExternalCollectionFlag,
        [switch] $MissingApprovalGates,
        [switch] $UnknownApprovalGate,
        [switch] $MissingAdvisoryApprovalGate,
        [switch] $MissingBrowserRuntimeGate,
        [switch] $MissingRunnerTask,
        [switch] $MismatchedRunnerTask,
        [switch] $MissingRunnerTaskRegistry,
        [switch] $MissingRegisteredRunnerTask,
        [switch] $DuplicateRunnerTaskRegistry,
        [switch] $MissingRequiredCommandRegistry,
        [switch] $MissingRegisteredRequiredCommand,
        [switch] $DuplicateRequiredCommandRegistry,
        [switch] $MissingRequiredServiceRegistry,
        [switch] $MissingRegisteredRequiredService,
        [switch] $DuplicateRequiredServiceRegistry,
        [switch] $MissingRequiredCommand,
        [switch] $MissingRequiredService,
        [switch] $MissingRunnerCommandGuard,
        [switch] $MissingRunnerServiceGuard,
        [switch] $MissingRunnerTaskCase,
        [switch] $InvalidExitTarget,
        [switch] $TransitionExitTarget,
        [switch] $RetiredRustfmtTransition,
        [switch] $UntrackedCiTransition,
        [switch] $UnreferencedTransitionPolicy,
        [switch] $MissingWorkflowCommandProvisioning,
        [switch] $MissingWorkflowServiceProvisioning,
        [switch] $MissingExitState,
        [switch] $UnknownExitState,
        [switch] $MissingExitEvidenceRequirements,
        [switch] $MissingBlockingApprovalGate,
        [switch] $MissingExitTargetRegistry,
        [switch] $MissingRegisteredExitTarget,
        [switch] $MismatchedExitTargetEvidence,
        [switch] $MissingApprovalGateRegistry,
        [switch] $MissingRegisteredApprovalGate,
        [switch] $DuplicateApprovalGateRegistry,
        [switch] $MissingTransitionCategoryRegistry,
        [switch] $MissingRegisteredTransitionCategory,
        [switch] $MismatchedCategoryEvidence,
        [switch] $MissingExitEvidenceRequirementRegistry,
        [switch] $MissingRegisteredExitEvidenceRequirement,
        [switch] $DuplicateExitEvidenceRequirementRegistry
    )

    Write-File -Root $Root -RelativePath "tools\bazel\BUILD.bazel" -Content @'
load("//tools/bazel:shell_test_compat.bzl", "transition_shell_test")

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
'@

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

    $sunset = if ($ExpiredSunset) { "2020-01-01" } else { "2026-07-31" }
    $nodeAuditApprovalGates = if ($MissingAdvisoryApprovalGate) { "[]" } elseif ($UnknownApprovalGate) { '["typo_gate"]' } else { '["external_advisory_collection"]' }
    $approvalGatesLine = if ($MissingApprovalGates) { "" } else { "`"approval_gates`": []," }
    $frontendE2eApprovalGates = if ($MissingBrowserRuntimeGate) { "[]" } else { '["browser_runtime_provisioning"]' }
    $rustCheckExitTarget = if ($InvalidExitTarget) { "rust_verification" } elseif ($TransitionExitTarget) { "//tools/bazel:next_transition" } else { "//:rust_verification" }
    $rustCheckRunnerTaskLine = if ($MissingRunnerTask) { "" } elseif ($MismatchedRunnerTask) { '"runner_task": "rustfmt-check",' } else { '"runner_task": "rust-check",' }
    $nodeAuditRequiredCommands = if ($MissingRequiredCommand) { "[]" } else { '["pnpm"]' }
    $migrationRequiredServices = if ($MissingRequiredService) { "[]" } else { '["postgres"]' }
    $exitStateLine = if ($MissingExitState) { "" } elseif ($UnknownExitState) { '"exit_state": "done",' } else { '"exit_state": "blocked",' }
    $rustCheckEvidenceRequirements = if ($MissingExitEvidenceRequirements) { "[]" } else { '["native_bazel_test_target"]' }
    $nodeAuditBlockingApprovalGates = if ($MissingBlockingApprovalGate) { "[]" } else { '["external_advisory_collection"]' }
    $dependencyScaExitEvidenceRequirements = if ($MismatchedExitTargetEvidence) { '["native_bazel_evidence_target"]' } else { '["native_bazel_evidence_target", "pinned_advisory_evidence"]' }
    $externalAdvisoryCategoryEvidence = if ($MismatchedCategoryEvidence) { '["native_bazel_database_test"]' } else { '["native_bazel_evidence_target", "pinned_advisory_evidence"]' }
    $nativeBazelTestEvidenceEntry = if ($MissingRegisteredExitEvidenceRequirement) {
        ""
    } else {
        @'
    {
      "id": "native_bazel_test_target",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
'@
    }
    $duplicateExitEvidenceRequirementEntry = if ($DuplicateExitEvidenceRequirementRegistry) {
        @'
    {
      "id": "native_bazel_database_test",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "evidence_kind": "native_bazel_evidence"
    },
'@
    } else {
        ""
    }
    $registeredExitEvidenceRequirements = if ($MissingExitEvidenceRequirementRegistry) {
        ""
    } else {
        @"
  "exit_evidence_requirement_registry": [
    {
      "id": "database_service_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    },
    {
      "id": "native_bazel_coverage_evidence",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
$duplicateExitEvidenceRequirementEntry    {
      "id": "native_bazel_database_test",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
    {
      "id": "native_bazel_evidence_target",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
    {
      "id": "native_bazel_service_orchestration",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "native_bazel_evidence"
    },
$nativeBazelTestEvidenceEntry    {
      "id": "pinned_advisory_evidence",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "pinned_external_evidence"
    },
    {
      "id": "service_orchestration_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    },
    {
      "id": "toolchain_provisioning_decision",
      "owner": "build-platform",
      "reason": "fixture",
      "evidence_kind": "provisioning_decision"
    }
  ],
"@
    }
    $externalAdvisoryGateEntry = @'
    {
      "id": "external_advisory_collection",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "fixture",
      "requires_human_approval": true,
      "external_collection_approval_required": true
    },
'@
    $browserRuntimeGateEntry = if ($MissingRegisteredApprovalGate) {
        ""
    } else {
        @'
    {
      "id": "browser_runtime_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "fixture",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
'@
    }
    $duplicateApprovalGateEntry = if ($DuplicateApprovalGateRegistry) {
        @'
    {
      "id": "toolchain_provisioning",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "decision_reference": "fixture duplicate",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
'@
    } else {
        ""
    }
    $registeredApprovalGates = if ($MissingApprovalGateRegistry) {
        ""
    } else {
        @"
  "approval_gate_registry": [
$externalAdvisoryGateEntry$browserRuntimeGateEntry$duplicateApprovalGateEntry
    {
      "id": "toolchain_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "fixture",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
    {
      "id": "database_service_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "fixture",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    },
    {
      "id": "service_orchestration_provisioning",
      "owner": "build-platform",
      "reason": "fixture",
      "decision_reference": "fixture",
      "requires_human_approval": true,
      "external_collection_approval_required": false
    }
  ],
"@
    }
    $frontendReleaseCategoryEntry = if ($MissingRegisteredTransitionCategory) {
        ""
    } else {
        @'
    {
      "id": "frontend-release-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["native_bazel_test_target"],
      "required_approval_gates": ["browser_runtime_provisioning"],
      "external_collection_approval_required": false
    },
'@
    }
    $registeredTransitionCategories = if ($MissingTransitionCategoryRegistry) {
        ""
    } else {
        @"
  "transition_category_registry": [
    {
      "id": "external-advisory-sca",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": $externalAdvisoryCategoryEvidence,
      "required_approval_gates": ["external_advisory_collection"],
      "external_collection_approval_required": true
    },
    {
      "id": "rust-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["native_bazel_test_target"],
      "required_approval_gates": [],
      "external_collection_approval_required": false
    },
$frontendReleaseCategoryEntry    {
      "id": "database-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "required_approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "external_collection_approval_required": false
    },
    {
      "id": "stale-fixture",
      "owner": "build-platform",
      "reason": "fixture",
      "required_exit_evidence_requirements": [],
      "required_approval_gates": [],
      "external_collection_approval_required": false
    }
  ],
"@
    }
    $frontendE2eRunnerTaskEntry = if ($MissingRegisteredRunnerTask) {
        ""
    } else {
        @'
    {
      "id": "frontend-e2e",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pnpm"],
      "required_services": []
    },
'@
    }
    $duplicateRunnerTaskEntry = if ($DuplicateRunnerTaskRegistry) {
        @'
    {
      "id": "node-audit",
      "owner": "build-platform",
      "reason": "fixture duplicate",
      "required_commands": ["pnpm"],
      "required_services": []
    },
'@
    } else {
        ""
    }
    $pnpmRequiredCommandEntry = if ($MissingRegisteredRequiredCommand) {
        ""
    } else {
        @'
    {
      "id": "pnpm",
      "owner": "build-platform",
      "reason": "fixture"
    },
'@
    }
    $duplicateRequiredCommandEntry = if ($DuplicateRequiredCommandRegistry) {
        @'
    {
      "id": "cargo",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredRequiredCommands = if ($MissingRequiredCommandRegistry) {
        ""
    } else {
        @"
  "required_command_registry": [
    {
      "id": "cargo",
      "owner": "build-platform",
      "reason": "fixture"
    },
$duplicateRequiredCommandEntry    {
      "id": "pg_isready",
      "owner": "build-platform",
      "reason": "fixture"
    },
$pnpmRequiredCommandEntry    {
      "id": "psql",
      "owner": "build-platform",
      "reason": "fixture"
    },
    {
      "id": "sqlx",
      "owner": "build-platform",
      "reason": "fixture"
    }
  ],
"@
    }
    $postgresRequiredServiceEntry = if ($MissingRegisteredRequiredService) {
        @'
    {
      "id": "redis",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    } else {
        @'
    {
      "id": "postgres",
      "owner": "build-platform",
      "reason": "fixture"
    }
'@
    }
    $duplicateRequiredServiceEntry = if ($DuplicateRequiredServiceRegistry) {
        @'
    {
      "id": "postgres",
      "owner": "build-platform",
      "reason": "fixture duplicate"
    },
'@
    } else {
        ""
    }
    $registeredRequiredServices = if ($MissingRequiredServiceRegistry) {
        ""
    } else {
        @"
  "required_service_registry": [
$duplicateRequiredServiceEntry$postgresRequiredServiceEntry
  ],
"@
    }
    $registeredRunnerTasks = if ($MissingRunnerTaskRegistry) {
        ""
    } else {
        @"
  "runner_task_registry": [
    {
      "id": "deleted",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": [],
      "required_services": []
    },
$duplicateRunnerTaskEntry$frontendE2eRunnerTaskEntry    {
      "id": "migration-v001-full",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pg_isready", "psql", "sqlx"],
      "required_services": ["postgres"]
    },
    {
      "id": "node-audit",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["pnpm"],
      "required_services": []
    },
    {
      "id": "rust-check",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["cargo"],
      "required_services": []
    },
    {
      "id": "rustfmt-check",
      "owner": "build-platform",
      "reason": "fixture",
      "required_commands": ["cargo"],
      "required_services": []
    }
  ],
"@
    }
    $deletedExitTargetRegistryEntry = if ($AddStalePolicy) {
        @'
,
    {
      "bazel_target": "//:deleted",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": [],
      "blocking_approval_gates": []
    }
'@
    } else {
        ""
    }
    $registeredExitTargets = if ($MissingExitTargetRegistry) {
        ""
    } elseif ($MissingRegisteredExitTarget) {
        @"
  "exit_targets": [
    {
      "bazel_target": "//:rust_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
    }
$deletedExitTargetRegistryEntry
  ],
"@
    } else {
        @"
  "exit_targets": [
    {
      "bazel_target": "//:dependency_sca_evidence",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": $dependencyScaExitEvidenceRequirements,
      "blocking_approval_gates": ["external_advisory_collection"]
    },
    {
      "bazel_target": "//:rust_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//tools/bazel:rustfmt_check",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": []
    },
    {
      "bazel_target": "//:frontend_e2e",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": ["browser_runtime_provisioning"]
    },
    {
      "bazel_target": "//:migration_verification",
      "state": "planned",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"]
    }
$deletedExitTargetRegistryEntry
  ],
"@
    }
    $nodeAuditPolicy = if ($OmitNodeAuditPolicy) {
        ""
    } else {
        $externalCollection = if ($MissingExternalCollectionFlag) { "false" } else { "true" }
        @"
    {
      "bazel_target": "//tools/bazel:ci_node_audit_transition",
      "category": "external-advisory-sca",
      "owner": "build-platform",
      "reason": "pnpm audit still shells out until advisory SCA is represented by a pinned Bazel evidence target.",
      "exit_target": "//:dependency_sca_evidence",
$exitStateLine
      "exit_evidence_requirements": ["native_bazel_evidence_target", "pinned_advisory_evidence"],
      "blocking_approval_gates": $nodeAuditBlockingApprovalGates,
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "node-audit",
      "required_commands": $nodeAuditRequiredCommands,
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": $nodeAuditApprovalGates,
      "external_collection_approval_required": $externalCollection
    },
"@
    }
    $stalePolicy = if ($AddStalePolicy) {
        @'
    {
      "bazel_target": "//tools/bazel:deleted_transition",
      "category": "stale-fixture",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//:deleted",
      "exit_state": "blocked",
      "exit_evidence_requirements": [],
      "blocking_approval_gates": [],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "deleted",
      "required_commands": [],
      "required_services": [],
      "sunset": "2026-07-31",
      "approval_gates": [],
      "external_collection_approval_required": false
    },
'@
    } else {
        ""
    }
    $retiredTargets = if ($RetiredRustfmtTransition) {
        @'
  "retired_transition_targets": [
    "//tools/bazel:ci_rustfmt_transition"
  ],
'@
    } else {
        @'
  "retired_transition_targets": [],
'@
    }
    Write-File -Root $Root -RelativePath "docs\architecture\verification-transition-ratchet.v1.json" -Content @"
{
  "schema_version": "gongzzang.verification_transition_ratchet.v1",
  "repo_slug": "gongzzang",
  "default_decision": "deny_new_transition_without_policy",
$retiredTargets
$registeredApprovalGates
$registeredExitEvidenceRequirements
$registeredTransitionCategories
$registeredRequiredCommands
$registeredRequiredServices
$registeredRunnerTasks
$registeredExitTargets
  "transition_targets": [
$nodeAuditPolicy$stalePolicy    {
      "bazel_target": "//tools/bazel:ci_rust_check_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "cargo check transition until Rust check is a native Bazel rule target.",
      "exit_target": "$rustCheckExitTarget",
$exitStateLine
      "exit_evidence_requirements": $rustCheckEvidenceRequirements,
      "blocking_approval_gates": [],
$rustCheckRunnerTaskLine
      "runner_script": "run_ci_transition_task.sh",
      "required_commands": ["cargo"],
      "required_services": [],
      "sunset": "$sunset",
$approvalGatesLine
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:ci_rustfmt_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//tools/bazel:rustfmt_check",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": [],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "rustfmt-check",
      "required_commands": ["cargo"],
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": [],
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:frontend_e2e_transition",
      "category": "frontend-release-verification",
      "owner": "build-platform",
      "reason": "Playwright transition retained until browser provisioning and e2e execution are native Bazel targets.",
      "exit_target": "//:frontend_e2e",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["native_bazel_test_target"],
      "blocking_approval_gates": $frontendE2eApprovalGates,
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "frontend-e2e",
      "required_commands": ["pnpm"],
      "required_services": [],
      "sunset": "$sunset",
      "approval_gates": $frontendE2eApprovalGates,
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:ci_migration_v001_full_transition",
      "category": "database-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//:migration_verification",
      "exit_state": "blocked",
      "exit_evidence_requirements": ["database_service_provisioning_decision", "native_bazel_database_test"],
      "blocking_approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "migration-v001-full",
      "required_commands": ["pg_isready", "psql", "sqlx"],
      "required_services": $migrationRequiredServices,
      "sunset": "$sunset",
      "approval_gates": ["toolchain_provisioning", "database_service_provisioning"],
      "external_collection_approval_required": false
    }
  ]
}
"@

    $extraCi = if ($UntrackedCiTransition) {
        "      - run: bazelisk test //tools/bazel:unknown_transition --config=ci"
    } else {
        ""
    }
    $frontendE2eCi = if ($UnreferencedTransitionPolicy) {
        ""
    } else {
        "      - run: bazelisk test //tools/bazel:frontend_e2e_transition --config=ci"
    }
    $workflowPnpmInstall = if ($MissingWorkflowCommandProvisioning) { "" } else { "      - run: pnpm install --frozen-lockfile" }
    $workflowPostgresService = if ($MissingWorkflowServiceProvisioning) {
        ""
    } else {
        @'
    services:
      postgres:
        image: postgis/postgis:17-3.5
'@
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
jobs:
  verify:
$workflowPostgresService
    steps:
      - uses: pnpm/action-setup@0e279bb959325dab635dd2c09392533439d90093
$workflowPnpmInstall
      - uses: dtolnay/rust-toolchain@21dc36fb71dd22e3317045c0c31a3f4249868b17
      - run: |
          sudo apt-get update -qq
          sudo apt-get install -y postgresql-client
          cargo install sqlx-cli --version 0.8.6 --locked --no-default-features --features postgres,rustls
      - run: bazelisk test //tools/bazel:ci_node_audit_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_rust_check_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_rustfmt_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_migration_v001_full_transition --config=ci
$frontendE2eCi
$extraCi
"@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-Equals $success.ExitCode 0 "success exit code mismatch output=$($success.Output)"
    Assert-Contains $success.Output "bazel-transition-ratchet-ok"

    $missingPolicyRoot = Join-Path $TempRoot "missing-policy"
    Write-MinimalRepo -Root $missingPolicyRoot -OmitNodeAuditPolicy
    $missingPolicy = Invoke-Checker -Root $missingPolicyRoot
    Assert-Equals $missingPolicy.ExitCode 1 "missing policy exit code mismatch"
    Assert-Contains $missingPolicy.Output "missing transition policy"

    $stalePolicyRoot = Join-Path $TempRoot "stale-policy"
    Write-MinimalRepo -Root $stalePolicyRoot -AddStalePolicy
    $stalePolicy = Invoke-Checker -Root $stalePolicyRoot
    Assert-Equals $stalePolicy.ExitCode 1 "stale policy exit code mismatch"
    Assert-Contains $stalePolicy.Output "stale transition policy"

    $expiredSunsetRoot = Join-Path $TempRoot "expired-sunset"
    Write-MinimalRepo -Root $expiredSunsetRoot -ExpiredSunset
    $expiredSunset = Invoke-Checker -Root $expiredSunsetRoot
    Assert-Equals $expiredSunset.ExitCode 1 "expired sunset exit code mismatch"
    Assert-Contains $expiredSunset.Output "expired transition sunset"

    $missingExternalFlagRoot = Join-Path $TempRoot "missing-external-collection-flag"
    Write-MinimalRepo -Root $missingExternalFlagRoot -MissingExternalCollectionFlag
    $missingExternalFlag = Invoke-Checker -Root $missingExternalFlagRoot
    Assert-Equals $missingExternalFlag.ExitCode 1 "missing external collection flag exit code mismatch"
    Assert-Contains $missingExternalFlag.Output "transition category requires external collection approval"

    $missingApprovalGatesRoot = Join-Path $TempRoot "missing-approval-gates"
    Write-MinimalRepo -Root $missingApprovalGatesRoot -MissingApprovalGates
    $missingApprovalGates = Invoke-Checker -Root $missingApprovalGatesRoot
    Assert-Equals $missingApprovalGates.ExitCode 1 "missing approval gates exit code mismatch"
    Assert-Contains $missingApprovalGates.Output "transition policy //tools/bazel:ci_rust_check_transition missing 'approval_gates'"

    $unknownApprovalGateRoot = Join-Path $TempRoot "unknown-approval-gate"
    Write-MinimalRepo -Root $unknownApprovalGateRoot -UnknownApprovalGate
    $unknownApprovalGate = Invoke-Checker -Root $unknownApprovalGateRoot
    Assert-Equals $unknownApprovalGate.ExitCode 1 "unknown approval gate exit code mismatch"
    Assert-Contains $unknownApprovalGate.Output "transition approval gate is not registered"

    $missingAdvisoryApprovalGateRoot = Join-Path $TempRoot "missing-advisory-approval-gate"
    Write-MinimalRepo -Root $missingAdvisoryApprovalGateRoot -MissingAdvisoryApprovalGate
    $missingAdvisoryApprovalGate = Invoke-Checker -Root $missingAdvisoryApprovalGateRoot
    Assert-Equals $missingAdvisoryApprovalGate.ExitCode 1 "missing advisory approval gate exit code mismatch"
    Assert-Contains $missingAdvisoryApprovalGate.Output "transition category required_approval_gates"

    $missingBrowserRuntimeGateRoot = Join-Path $TempRoot "missing-browser-runtime-gate"
    Write-MinimalRepo -Root $missingBrowserRuntimeGateRoot -MissingBrowserRuntimeGate
    $missingBrowserRuntimeGate = Invoke-Checker -Root $missingBrowserRuntimeGateRoot
    Assert-Equals $missingBrowserRuntimeGate.ExitCode 1 "missing browser runtime gate exit code mismatch"
    Assert-Contains $missingBrowserRuntimeGate.Output "transition category required_approval_gates"

    $missingRunnerTaskRoot = Join-Path $TempRoot "missing-runner-task"
    Write-MinimalRepo -Root $missingRunnerTaskRoot -MissingRunnerTask
    $missingRunnerTask = Invoke-Checker -Root $missingRunnerTaskRoot
    Assert-Equals $missingRunnerTask.ExitCode 1 "missing runner task exit code mismatch"
    Assert-Contains $missingRunnerTask.Output "transition policy //tools/bazel:ci_rust_check_transition missing 'runner_task'"

    $mismatchedRunnerTaskRoot = Join-Path $TempRoot "mismatched-runner-task"
    Write-MinimalRepo -Root $mismatchedRunnerTaskRoot -MismatchedRunnerTask
    $mismatchedRunnerTask = Invoke-Checker -Root $mismatchedRunnerTaskRoot
    Assert-Equals $mismatchedRunnerTask.ExitCode 1 "mismatched runner task exit code mismatch"
    Assert-Contains $mismatchedRunnerTask.Output "transition policy runner_task does not match BUILD script_args"

    $missingRunnerTaskRegistryRoot = Join-Path $TempRoot "missing-runner-task-registry"
    Write-MinimalRepo -Root $missingRunnerTaskRegistryRoot -MissingRunnerTaskRegistry
    $missingRunnerTaskRegistry = Invoke-Checker -Root $missingRunnerTaskRegistryRoot
    Assert-Equals $missingRunnerTaskRegistry.ExitCode 1 "missing runner task registry exit code mismatch"
    Assert-Contains $missingRunnerTaskRegistry.Output "transition ratchet policy must declare runner_task_registry"

    $missingRegisteredRunnerTaskRoot = Join-Path $TempRoot "missing-registered-runner-task"
    Write-MinimalRepo -Root $missingRegisteredRunnerTaskRoot -MissingRegisteredRunnerTask
    $missingRegisteredRunnerTask = Invoke-Checker -Root $missingRegisteredRunnerTaskRoot
    Assert-Equals $missingRegisteredRunnerTask.ExitCode 1 "missing registered runner task exit code mismatch"
    Assert-Contains $missingRegisteredRunnerTask.Output "runner task is not registered"

    $duplicateRunnerTaskRegistryRoot = Join-Path $TempRoot "duplicate-runner-task-registry"
    Write-MinimalRepo -Root $duplicateRunnerTaskRegistryRoot -DuplicateRunnerTaskRegistry
    $duplicateRunnerTaskRegistry = Invoke-Checker -Root $duplicateRunnerTaskRegistryRoot
    Assert-Equals $duplicateRunnerTaskRegistry.ExitCode 1 "duplicate runner task registry exit code mismatch"
    Assert-Contains $duplicateRunnerTaskRegistry.Output "transition ratchet runner task duplicate"

    $missingRequiredCommandRegistryRoot = Join-Path $TempRoot "missing-required-command-registry"
    Write-MinimalRepo -Root $missingRequiredCommandRegistryRoot -MissingRequiredCommandRegistry
    $missingRequiredCommandRegistry = Invoke-Checker -Root $missingRequiredCommandRegistryRoot
    Assert-Equals $missingRequiredCommandRegistry.ExitCode 1 "missing required command registry exit code mismatch"
    Assert-Contains $missingRequiredCommandRegistry.Output "transition ratchet policy must declare required_command_registry"

    $missingRegisteredRequiredCommandRoot = Join-Path $TempRoot "missing-registered-required-command"
    Write-MinimalRepo -Root $missingRegisteredRequiredCommandRoot -MissingRegisteredRequiredCommand
    $missingRegisteredRequiredCommand = Invoke-Checker -Root $missingRegisteredRequiredCommandRoot
    Assert-Equals $missingRegisteredRequiredCommand.ExitCode 1 "missing registered required command exit code mismatch"
    Assert-Contains $missingRegisteredRequiredCommand.Output "required command is not registered"

    $duplicateRequiredCommandRegistryRoot = Join-Path $TempRoot "duplicate-required-command-registry"
    Write-MinimalRepo -Root $duplicateRequiredCommandRegistryRoot -DuplicateRequiredCommandRegistry
    $duplicateRequiredCommandRegistry = Invoke-Checker -Root $duplicateRequiredCommandRegistryRoot
    Assert-Equals $duplicateRequiredCommandRegistry.ExitCode 1 "duplicate required command registry exit code mismatch"
    Assert-Contains $duplicateRequiredCommandRegistry.Output "transition ratchet required command duplicate"

    $missingRequiredServiceRegistryRoot = Join-Path $TempRoot "missing-required-service-registry"
    Write-MinimalRepo -Root $missingRequiredServiceRegistryRoot -MissingRequiredServiceRegistry
    $missingRequiredServiceRegistry = Invoke-Checker -Root $missingRequiredServiceRegistryRoot
    Assert-Equals $missingRequiredServiceRegistry.ExitCode 1 "missing required service registry exit code mismatch"
    Assert-Contains $missingRequiredServiceRegistry.Output "transition ratchet policy must declare required_service_registry"

    $missingRegisteredRequiredServiceRoot = Join-Path $TempRoot "missing-registered-required-service"
    Write-MinimalRepo -Root $missingRegisteredRequiredServiceRoot -MissingRegisteredRequiredService
    $missingRegisteredRequiredService = Invoke-Checker -Root $missingRegisteredRequiredServiceRoot
    Assert-Equals $missingRegisteredRequiredService.ExitCode 1 "missing registered required service exit code mismatch"
    Assert-Contains $missingRegisteredRequiredService.Output "required service is not registered"

    $duplicateRequiredServiceRegistryRoot = Join-Path $TempRoot "duplicate-required-service-registry"
    Write-MinimalRepo -Root $duplicateRequiredServiceRegistryRoot -DuplicateRequiredServiceRegistry
    $duplicateRequiredServiceRegistry = Invoke-Checker -Root $duplicateRequiredServiceRegistryRoot
    Assert-Equals $duplicateRequiredServiceRegistry.ExitCode 1 "duplicate required service registry exit code mismatch"
    Assert-Contains $duplicateRequiredServiceRegistry.Output "transition ratchet required service duplicate"

    $missingRequiredCommandRoot = Join-Path $TempRoot "missing-required-command"
    Write-MinimalRepo -Root $missingRequiredCommandRoot -MissingRequiredCommand
    $missingRequiredCommand = Invoke-Checker -Root $missingRequiredCommandRoot
    Assert-Equals $missingRequiredCommand.ExitCode 1 "missing required command exit code mismatch"
    Assert-Contains $missingRequiredCommand.Output "transition policy required_commands for //tools/bazel:ci_node_audit_transition missing 'pnpm'"

    $missingRequiredServiceRoot = Join-Path $TempRoot "missing-required-service"
    Write-MinimalRepo -Root $missingRequiredServiceRoot -MissingRequiredService
    $missingRequiredService = Invoke-Checker -Root $missingRequiredServiceRoot
    Assert-Equals $missingRequiredService.ExitCode 1 "missing required service exit code mismatch"
    Assert-Contains $missingRequiredService.Output "transition policy required_services for //tools/bazel:ci_migration_v001_full_transition missing 'postgres'"

    $missingRunnerCommandGuardRoot = Join-Path $TempRoot "missing-runner-command-guard"
    Write-MinimalRepo -Root $missingRunnerCommandGuardRoot -MissingRunnerCommandGuard
    $missingRunnerCommandGuard = Invoke-Checker -Root $missingRunnerCommandGuardRoot
    Assert-Equals $missingRunnerCommandGuard.ExitCode 1 "missing runner command guard exit code mismatch"
    Assert-Contains $missingRunnerCommandGuard.Output "runner script missing required command guard"

    $missingRunnerServiceGuardRoot = Join-Path $TempRoot "missing-runner-service-guard"
    Write-MinimalRepo -Root $missingRunnerServiceGuardRoot -MissingRunnerServiceGuard
    $missingRunnerServiceGuard = Invoke-Checker -Root $missingRunnerServiceGuardRoot
    Assert-Equals $missingRunnerServiceGuard.ExitCode 1 "missing runner service guard exit code mismatch"
    Assert-Contains $missingRunnerServiceGuard.Output "runner script missing required service guard"

    $missingRunnerTaskCaseRoot = Join-Path $TempRoot "missing-runner-task-case"
    Write-MinimalRepo -Root $missingRunnerTaskCaseRoot -MissingRunnerTaskCase
    $missingRunnerTaskCase = Invoke-Checker -Root $missingRunnerTaskCaseRoot
    Assert-Equals $missingRunnerTaskCase.ExitCode 1 "missing runner task case exit code mismatch"
    Assert-Contains $missingRunnerTaskCase.Output "runner script missing task case"

    $invalidExitTargetRoot = Join-Path $TempRoot "invalid-exit-target"
    Write-MinimalRepo -Root $invalidExitTargetRoot -InvalidExitTarget
    $invalidExitTarget = Invoke-Checker -Root $invalidExitTargetRoot
    Assert-Equals $invalidExitTarget.ExitCode 1 "invalid exit target exit code mismatch"
    Assert-Contains $invalidExitTarget.Output "transition policy exit_target must be a Bazel label"

    $transitionExitTargetRoot = Join-Path $TempRoot "transition-exit-target"
    Write-MinimalRepo -Root $transitionExitTargetRoot -TransitionExitTarget
    $transitionExitTarget = Invoke-Checker -Root $transitionExitTargetRoot
    Assert-Equals $transitionExitTarget.ExitCode 1 "transition exit target exit code mismatch"
    Assert-Contains $transitionExitTarget.Output "transition policy exit_target must not be another transition"

    $retiredRustfmtRoot = Join-Path $TempRoot "retired-rustfmt"
    Write-MinimalRepo -Root $retiredRustfmtRoot -RetiredRustfmtTransition
    $retiredRustfmt = Invoke-Checker -Root $retiredRustfmtRoot
    Assert-Equals $retiredRustfmt.ExitCode 1 "retired rustfmt transition exit code mismatch"
    Assert-Contains $retiredRustfmt.Output "retired transition target still exists"

    $untrackedCiRoot = Join-Path $TempRoot "untracked-ci"
    Write-MinimalRepo -Root $untrackedCiRoot -UntrackedCiTransition
    $untrackedCi = Invoke-Checker -Root $untrackedCiRoot
    Assert-Equals $untrackedCi.ExitCode 1 "untracked CI transition exit code mismatch"
    Assert-Contains $untrackedCi.Output "CI references transition target without policy"

    $unreferencedTransitionRoot = Join-Path $TempRoot "unreferenced-transition"
    Write-MinimalRepo -Root $unreferencedTransitionRoot -UnreferencedTransitionPolicy
    $unreferencedTransition = Invoke-Checker -Root $unreferencedTransitionRoot
    Assert-Equals $unreferencedTransition.ExitCode 1 "unreferenced transition exit code mismatch"
    Assert-Contains $unreferencedTransition.Output "active transition target is not referenced by CI or hooks"

    $missingWorkflowCommandProvisioningRoot = Join-Path $TempRoot "missing-workflow-command-provisioning"
    Write-MinimalRepo -Root $missingWorkflowCommandProvisioningRoot -MissingWorkflowCommandProvisioning
    $missingWorkflowCommandProvisioning = Invoke-Checker -Root $missingWorkflowCommandProvisioningRoot
    Assert-Equals $missingWorkflowCommandProvisioning.ExitCode 1 "missing workflow command provisioning exit code mismatch"
    Assert-Contains $missingWorkflowCommandProvisioning.Output "workflow job missing required command provisioning"

    $missingWorkflowServiceProvisioningRoot = Join-Path $TempRoot "missing-workflow-service-provisioning"
    Write-MinimalRepo -Root $missingWorkflowServiceProvisioningRoot -MissingWorkflowServiceProvisioning
    $missingWorkflowServiceProvisioning = Invoke-Checker -Root $missingWorkflowServiceProvisioningRoot
    Assert-Equals $missingWorkflowServiceProvisioning.ExitCode 1 "missing workflow service provisioning exit code mismatch"
    Assert-Contains $missingWorkflowServiceProvisioning.Output "workflow job missing required service provisioning"

    $missingExitStateRoot = Join-Path $TempRoot "missing-exit-state"
    Write-MinimalRepo -Root $missingExitStateRoot -MissingExitState
    $missingExitState = Invoke-Checker -Root $missingExitStateRoot
    Assert-Equals $missingExitState.ExitCode 1 "missing exit state exit code mismatch"
    Assert-Contains $missingExitState.Output "missing 'exit_state'"

    $unknownExitStateRoot = Join-Path $TempRoot "unknown-exit-state"
    Write-MinimalRepo -Root $unknownExitStateRoot -UnknownExitState
    $unknownExitState = Invoke-Checker -Root $unknownExitStateRoot
    Assert-Equals $unknownExitState.ExitCode 1 "unknown exit state exit code mismatch"
    Assert-Contains $unknownExitState.Output "unknown transition exit_state"

    $missingExitEvidenceRoot = Join-Path $TempRoot "missing-exit-evidence"
    Write-MinimalRepo -Root $missingExitEvidenceRoot -MissingExitEvidenceRequirements
    $missingExitEvidence = Invoke-Checker -Root $missingExitEvidenceRoot
    Assert-Equals $missingExitEvidence.ExitCode 1 "missing exit evidence exit code mismatch"
    Assert-Contains $missingExitEvidence.Output "transition category required_exit_evidence_requirements"

    $missingBlockingApprovalGateRoot = Join-Path $TempRoot "missing-blocking-approval-gate"
    Write-MinimalRepo -Root $missingBlockingApprovalGateRoot -MissingBlockingApprovalGate
    $missingBlockingApprovalGate = Invoke-Checker -Root $missingBlockingApprovalGateRoot
    Assert-Equals $missingBlockingApprovalGate.ExitCode 1 "missing blocking approval gate exit code mismatch"
    Assert-Contains $missingBlockingApprovalGate.Output "transition policy blocking_approval_gates"

    $missingExitTargetRegistryRoot = Join-Path $TempRoot "missing-exit-target-registry"
    Write-MinimalRepo -Root $missingExitTargetRegistryRoot -MissingExitTargetRegistry
    $missingExitTargetRegistry = Invoke-Checker -Root $missingExitTargetRegistryRoot
    Assert-Equals $missingExitTargetRegistry.ExitCode 1 "missing exit target registry exit code mismatch"
    Assert-Contains $missingExitTargetRegistry.Output "transition ratchet policy must declare exit_targets"

    $missingRegisteredExitTargetRoot = Join-Path $TempRoot "missing-registered-exit-target"
    Write-MinimalRepo -Root $missingRegisteredExitTargetRoot -MissingRegisteredExitTarget
    $missingRegisteredExitTarget = Invoke-Checker -Root $missingRegisteredExitTargetRoot
    Assert-Equals $missingRegisteredExitTarget.ExitCode 1 "missing registered exit target exit code mismatch"
    Assert-Contains $missingRegisteredExitTarget.Output "transition exit_target is not registered"

    $mismatchedExitTargetEvidenceRoot = Join-Path $TempRoot "mismatched-exit-target-evidence"
    Write-MinimalRepo -Root $mismatchedExitTargetEvidenceRoot -MismatchedExitTargetEvidence
    $mismatchedExitTargetEvidence = Invoke-Checker -Root $mismatchedExitTargetEvidenceRoot
    Assert-Equals $mismatchedExitTargetEvidence.ExitCode 1 "mismatched exit target evidence exit code mismatch"
    Assert-Contains $mismatchedExitTargetEvidence.Output "exit target registry exit_evidence_requirements"

    $missingApprovalGateRegistryRoot = Join-Path $TempRoot "missing-approval-gate-registry"
    Write-MinimalRepo -Root $missingApprovalGateRegistryRoot -MissingApprovalGateRegistry
    $missingApprovalGateRegistry = Invoke-Checker -Root $missingApprovalGateRegistryRoot
    Assert-Equals $missingApprovalGateRegistry.ExitCode 1 "missing approval gate registry exit code mismatch"
    Assert-Contains $missingApprovalGateRegistry.Output "transition ratchet policy must declare approval_gate_registry"

    $missingRegisteredApprovalGateRoot = Join-Path $TempRoot "missing-registered-approval-gate"
    Write-MinimalRepo -Root $missingRegisteredApprovalGateRoot -MissingRegisteredApprovalGate
    $missingRegisteredApprovalGate = Invoke-Checker -Root $missingRegisteredApprovalGateRoot
    Assert-Equals $missingRegisteredApprovalGate.ExitCode 1 "missing registered approval gate exit code mismatch"
    Assert-Contains $missingRegisteredApprovalGate.Output "approval gate is not registered"

    $duplicateApprovalGateRegistryRoot = Join-Path $TempRoot "duplicate-approval-gate-registry"
    Write-MinimalRepo -Root $duplicateApprovalGateRegistryRoot -DuplicateApprovalGateRegistry
    $duplicateApprovalGateRegistry = Invoke-Checker -Root $duplicateApprovalGateRegistryRoot
    Assert-Equals $duplicateApprovalGateRegistry.ExitCode 1 "duplicate approval gate registry exit code mismatch"
    Assert-Contains $duplicateApprovalGateRegistry.Output "transition ratchet approval gate duplicate"

    $missingTransitionCategoryRegistryRoot = Join-Path $TempRoot "missing-transition-category-registry"
    Write-MinimalRepo -Root $missingTransitionCategoryRegistryRoot -MissingTransitionCategoryRegistry
    $missingTransitionCategoryRegistry = Invoke-Checker -Root $missingTransitionCategoryRegistryRoot
    Assert-Equals $missingTransitionCategoryRegistry.ExitCode 1 "missing transition category registry exit code mismatch"
    Assert-Contains $missingTransitionCategoryRegistry.Output "transition ratchet policy must declare transition_category_registry"

    $missingRegisteredTransitionCategoryRoot = Join-Path $TempRoot "missing-registered-transition-category"
    Write-MinimalRepo -Root $missingRegisteredTransitionCategoryRoot -MissingRegisteredTransitionCategory
    $missingRegisteredTransitionCategory = Invoke-Checker -Root $missingRegisteredTransitionCategoryRoot
    Assert-Equals $missingRegisteredTransitionCategory.ExitCode 1 "missing registered transition category exit code mismatch"
    Assert-Contains $missingRegisteredTransitionCategory.Output "transition category is not registered"

    $mismatchedCategoryEvidenceRoot = Join-Path $TempRoot "mismatched-category-evidence"
    Write-MinimalRepo -Root $mismatchedCategoryEvidenceRoot -MismatchedCategoryEvidence
    $mismatchedCategoryEvidence = Invoke-Checker -Root $mismatchedCategoryEvidenceRoot
    Assert-Equals $mismatchedCategoryEvidence.ExitCode 1 "mismatched category evidence exit code mismatch"
    Assert-Contains $mismatchedCategoryEvidence.Output "transition category required_exit_evidence_requirements"

    $missingExitEvidenceRequirementRegistryRoot = Join-Path $TempRoot "missing-exit-evidence-requirement-registry"
    Write-MinimalRepo -Root $missingExitEvidenceRequirementRegistryRoot -MissingExitEvidenceRequirementRegistry
    $missingExitEvidenceRequirementRegistry = Invoke-Checker -Root $missingExitEvidenceRequirementRegistryRoot
    Assert-Equals $missingExitEvidenceRequirementRegistry.ExitCode 1 "missing exit evidence requirement registry exit code mismatch"
    Assert-Contains $missingExitEvidenceRequirementRegistry.Output "transition ratchet policy must declare exit_evidence_requirement_registry"

    $missingRegisteredExitEvidenceRequirementRoot = Join-Path $TempRoot "missing-registered-exit-evidence-requirement"
    Write-MinimalRepo -Root $missingRegisteredExitEvidenceRequirementRoot -MissingRegisteredExitEvidenceRequirement
    $missingRegisteredExitEvidenceRequirement = Invoke-Checker -Root $missingRegisteredExitEvidenceRequirementRoot
    Assert-Equals $missingRegisteredExitEvidenceRequirement.ExitCode 1 "missing registered exit evidence requirement exit code mismatch"
    Assert-Contains $missingRegisteredExitEvidenceRequirement.Output "exit evidence requirement is not registered"

    $duplicateExitEvidenceRequirementRegistryRoot = Join-Path $TempRoot "duplicate-exit-evidence-requirement-registry"
    Write-MinimalRepo -Root $duplicateExitEvidenceRequirementRegistryRoot -DuplicateExitEvidenceRequirementRegistry
    $duplicateExitEvidenceRequirementRegistry = Invoke-Checker -Root $duplicateExitEvidenceRequirementRegistryRoot
    Assert-Equals $duplicateExitEvidenceRequirementRegistry.ExitCode 1 "duplicate exit evidence requirement registry exit code mismatch"
    Assert-Contains $duplicateExitEvidenceRequirementRegistry.Output "transition ratchet exit evidence requirement duplicate"

    Write-Host "bazel-transition-ratchet-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
