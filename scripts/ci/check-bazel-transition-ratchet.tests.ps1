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
        [switch] $MissingRequiredCommand,
        [switch] $MissingRequiredService,
        [switch] $InvalidExitTarget,
        [switch] $TransitionExitTarget,
        [switch] $RetiredRustfmtTransition,
        [switch] $UntrackedCiTransition,
        [switch] $UnreferencedTransitionPolicy
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

    $sunset = if ($ExpiredSunset) { "2020-01-01" } else { "2026-07-31" }
    $nodeAuditApprovalGates = if ($MissingAdvisoryApprovalGate) { "[]" } elseif ($UnknownApprovalGate) { '["typo_gate"]' } else { '["external_advisory_collection"]' }
    $approvalGatesLine = if ($MissingApprovalGates) { "" } else { "`"approval_gates`": []," }
    $frontendE2eApprovalGates = if ($MissingBrowserRuntimeGate) { "[]" } else { '["browser_runtime_provisioning"]' }
    $rustCheckExitTarget = if ($InvalidExitTarget) { "rust_verification" } elseif ($TransitionExitTarget) { "//tools/bazel:next_transition" } else { "//:rust_verification" }
    $rustCheckRunnerTaskLine = if ($MissingRunnerTask) { "" } elseif ($MismatchedRunnerTask) { '"runner_task": "rustfmt-check",' } else { '"runner_task": "rust-check",' }
    $nodeAuditRequiredCommands = if ($MissingRequiredCommand) { "[]" } else { '["pnpm"]' }
    $migrationRequiredServices = if ($MissingRequiredService) { "[]" } else { '["postgres"]' }
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
  "transition_targets": [
$nodeAuditPolicy$stalePolicy    {
      "bazel_target": "//tools/bazel:ci_rust_check_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "cargo check transition until Rust check is a native Bazel rule target.",
      "exit_target": "$rustCheckExitTarget",
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
      "runner_script": "run_ci_transition_task.sh",
      "runner_task": "migration-v001-full",
      "required_commands": ["sqlx", "psql"],
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
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
jobs:
  verify:
    steps:
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
    Assert-Equals $success.ExitCode 0 "success exit code mismatch"
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
    Assert-Contains $missingExternalFlag.Output "external advisory collection transition must require approval"

    $missingApprovalGatesRoot = Join-Path $TempRoot "missing-approval-gates"
    Write-MinimalRepo -Root $missingApprovalGatesRoot -MissingApprovalGates
    $missingApprovalGates = Invoke-Checker -Root $missingApprovalGatesRoot
    Assert-Equals $missingApprovalGates.ExitCode 1 "missing approval gates exit code mismatch"
    Assert-Contains $missingApprovalGates.Output "transition policy //tools/bazel:ci_rust_check_transition missing 'approval_gates'"

    $unknownApprovalGateRoot = Join-Path $TempRoot "unknown-approval-gate"
    Write-MinimalRepo -Root $unknownApprovalGateRoot -UnknownApprovalGate
    $unknownApprovalGate = Invoke-Checker -Root $unknownApprovalGateRoot
    Assert-Equals $unknownApprovalGate.ExitCode 1 "unknown approval gate exit code mismatch"
    Assert-Contains $unknownApprovalGate.Output "unknown transition approval gate for //tools/bazel:ci_node_audit_transition"

    $missingAdvisoryApprovalGateRoot = Join-Path $TempRoot "missing-advisory-approval-gate"
    Write-MinimalRepo -Root $missingAdvisoryApprovalGateRoot -MissingAdvisoryApprovalGate
    $missingAdvisoryApprovalGate = Invoke-Checker -Root $missingAdvisoryApprovalGateRoot
    Assert-Equals $missingAdvisoryApprovalGate.ExitCode 1 "missing advisory approval gate exit code mismatch"
    Assert-Contains $missingAdvisoryApprovalGate.Output "external advisory transition must declare approval gate"

    $missingBrowserRuntimeGateRoot = Join-Path $TempRoot "missing-browser-runtime-gate"
    Write-MinimalRepo -Root $missingBrowserRuntimeGateRoot -MissingBrowserRuntimeGate
    $missingBrowserRuntimeGate = Invoke-Checker -Root $missingBrowserRuntimeGateRoot
    Assert-Equals $missingBrowserRuntimeGate.ExitCode 1 "missing browser runtime gate exit code mismatch"
    Assert-Contains $missingBrowserRuntimeGate.Output "frontend e2e transition must declare browser runtime provisioning gate"

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

    Write-Host "bazel-transition-ratchet-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
