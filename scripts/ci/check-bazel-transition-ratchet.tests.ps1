Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-bazel-transition-ratchet.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-bazel-transition-ratchet-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

. (Join-Path $PSScriptRoot "transition-ratchet-bazel.tests.helpers.ps1")

function Assert-FileLineCountAtMost {
    param(
        [string] $Path,
        [int] $MaxLines
    )

    $lineCount = (Get-Content -LiteralPath $Path | Measure-Object -Line).Lines
    if ($lineCount -gt $MaxLines) {
        throw "$Path line count $lineCount exceeds $MaxLines"
    }
}

Assert-FileLineCountAtMost -Path $PSCommandPath -MaxLines 600
Assert-FileLineCountAtMost -Path $ScriptPath -MaxLines 600

$checkerModuleRoot = Join-Path $PSScriptRoot "transition-ratchet-bazel"
Get-ChildItem -LiteralPath $checkerModuleRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

$testHelperPath = Join-Path $PSScriptRoot "transition-ratchet-bazel.tests.helpers.ps1"
Assert-FileLineCountAtMost -Path $testHelperPath -MaxLines 600

$testFixtureRoot = Join-Path $PSScriptRoot "transition-ratchet-bazel.tests"
Get-ChildItem -LiteralPath $testFixtureRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
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

    $invalidApprovalGateDecisionReferenceRoot = Join-Path $TempRoot "invalid-approval-gate-decision-reference"
    Write-MinimalRepo -Root $invalidApprovalGateDecisionReferenceRoot -InvalidApprovalGateDecisionReference
    $invalidApprovalGateDecisionReference = Invoke-Checker -Root $invalidApprovalGateDecisionReferenceRoot
    Assert-Equals $invalidApprovalGateDecisionReference.ExitCode 1 "invalid approval gate decision reference exit code mismatch"
    Assert-Contains $invalidApprovalGateDecisionReference.Output "approval gate decision_reference must point to a docs file"

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

    $missingGuardrailNoCacheTagRoot = Join-Path $TempRoot "missing-guardrail-no-cache-tag"
    Write-MinimalRepo -Root $missingGuardrailNoCacheTagRoot -MissingGuardrailNoCacheTag
    $missingGuardrailNoCacheTag = Invoke-Checker -Root $missingGuardrailNoCacheTagRoot
    Assert-Equals $missingGuardrailNoCacheTag.ExitCode 1 "missing guardrail no-cache tag exit code mismatch"
    Assert-Contains $missingGuardrailNoCacheTag.Output "GUARDRAIL_TRANSITION_TAGS missing 'no-cache'"

    $missingGuardrailExternalTagRoot = Join-Path $TempRoot "missing-guardrail-external-tag"
    Write-MinimalRepo -Root $missingGuardrailExternalTagRoot -MissingGuardrailExternalTag
    $missingGuardrailExternalTag = Invoke-Checker -Root $missingGuardrailExternalTagRoot
    Assert-Equals $missingGuardrailExternalTag.ExitCode 1 "missing guardrail external tag exit code mismatch"
    Assert-Contains $missingGuardrailExternalTag.Output "GUARDRAIL_TRANSITION_TAGS missing 'external'"

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

    $missingTransitionExitStateRegistryRoot = Join-Path $TempRoot "missing-transition-exit-state-registry"
    Write-MinimalRepo -Root $missingTransitionExitStateRegistryRoot -MissingTransitionExitStateRegistry
    $missingTransitionExitStateRegistry = Invoke-Checker -Root $missingTransitionExitStateRegistryRoot
    Assert-Equals $missingTransitionExitStateRegistry.ExitCode 1 "missing transition exit state registry exit code mismatch"
    Assert-Contains $missingTransitionExitStateRegistry.Output "transition ratchet policy must declare transition_exit_state_registry"

    $missingRegisteredTransitionExitStateRoot = Join-Path $TempRoot "missing-registered-transition-exit-state"
    Write-MinimalRepo -Root $missingRegisteredTransitionExitStateRoot -MissingRegisteredTransitionExitState
    $missingRegisteredTransitionExitState = Invoke-Checker -Root $missingRegisteredTransitionExitStateRoot
    Assert-Equals $missingRegisteredTransitionExitState.ExitCode 1 "missing registered transition exit state exit code mismatch"
    Assert-Contains $missingRegisteredTransitionExitState.Output "transition exit_state is not registered"

    $duplicateTransitionExitStateRegistryRoot = Join-Path $TempRoot "duplicate-transition-exit-state-registry"
    Write-MinimalRepo -Root $duplicateTransitionExitStateRegistryRoot -DuplicateTransitionExitStateRegistry
    $duplicateTransitionExitStateRegistry = Invoke-Checker -Root $duplicateTransitionExitStateRegistryRoot
    Assert-Equals $duplicateTransitionExitStateRegistry.ExitCode 1 "duplicate transition exit state registry exit code mismatch"
    Assert-Contains $duplicateTransitionExitStateRegistry.Output "transition ratchet transition exit state duplicate"

    $missingExitStateRoot = Join-Path $TempRoot "missing-exit-state"
    Write-MinimalRepo -Root $missingExitStateRoot -MissingExitState
    $missingExitState = Invoke-Checker -Root $missingExitStateRoot
    Assert-Equals $missingExitState.ExitCode 1 "missing exit state exit code mismatch"
    Assert-Contains $missingExitState.Output "missing 'exit_state'"

    $unknownExitStateRoot = Join-Path $TempRoot "unknown-exit-state"
    Write-MinimalRepo -Root $unknownExitStateRoot -UnknownExitState
    $unknownExitState = Invoke-Checker -Root $unknownExitStateRoot
    Assert-Equals $unknownExitState.ExitCode 1 "unknown exit state exit code mismatch"
    Assert-Contains $unknownExitState.Output "transition exit_state is not registered"

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

    $missingExitTargetStateRegistryRoot = Join-Path $TempRoot "missing-exit-target-state-registry"
    Write-MinimalRepo -Root $missingExitTargetStateRegistryRoot -MissingExitTargetStateRegistry
    $missingExitTargetStateRegistry = Invoke-Checker -Root $missingExitTargetStateRegistryRoot
    Assert-Equals $missingExitTargetStateRegistry.ExitCode 1 "missing exit target state registry exit code mismatch"
    Assert-Contains $missingExitTargetStateRegistry.Output "transition ratchet policy must declare exit_target_state_registry"

    $missingRegisteredExitTargetStateRoot = Join-Path $TempRoot "missing-registered-exit-target-state"
    Write-MinimalRepo -Root $missingRegisteredExitTargetStateRoot -MissingRegisteredExitTargetState
    $missingRegisteredExitTargetState = Invoke-Checker -Root $missingRegisteredExitTargetStateRoot
    Assert-Equals $missingRegisteredExitTargetState.ExitCode 1 "missing registered exit target state exit code mismatch"
    Assert-Contains $missingRegisteredExitTargetState.Output "exit target state is not registered"

    $duplicateExitTargetStateRegistryRoot = Join-Path $TempRoot "duplicate-exit-target-state-registry"
    Write-MinimalRepo -Root $duplicateExitTargetStateRegistryRoot -DuplicateExitTargetStateRegistry
    $duplicateExitTargetStateRegistry = Invoke-Checker -Root $duplicateExitTargetStateRegistryRoot
    Assert-Equals $duplicateExitTargetStateRegistry.ExitCode 1 "duplicate exit target state registry exit code mismatch"
    Assert-Contains $duplicateExitTargetStateRegistry.Output "transition ratchet exit target state duplicate"

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

    $missingExitTargetEvidenceStatusRoot = Join-Path $TempRoot "missing-exit-target-evidence-status"
    Write-MinimalRepo -Root $missingExitTargetEvidenceStatusRoot -MissingExitTargetEvidenceStatus
    $missingExitTargetEvidenceStatus = Invoke-Checker -Root $missingExitTargetEvidenceStatusRoot
    Assert-Equals $missingExitTargetEvidenceStatus.ExitCode 1 "missing exit target evidence status exit code mismatch"
    Assert-Contains $missingExitTargetEvidenceStatus.Output "exit target registry evidence_status"

    $availableMissingEvidenceTargetRoot = Join-Path $TempRoot "available-missing-evidence-target"
    Write-MinimalRepo -Root $availableMissingEvidenceTargetRoot -AvailableMissingEvidenceTarget
    $availableMissingEvidenceTarget = Invoke-Checker -Root $availableMissingEvidenceTargetRoot
    Assert-Equals $availableMissingEvidenceTarget.ExitCode 1 "available missing evidence target exit code mismatch"
    Assert-Contains $availableMissingEvidenceTarget.Output "available exit evidence target does not exist in Bazel BUILD files"

    $availableMissingExitTargetRoot = Join-Path $TempRoot "available-missing-exit-target"
    Write-MinimalRepo -Root $availableMissingExitTargetRoot -AvailableMissingExitTarget
    $availableMissingExitTarget = Invoke-Checker -Root $availableMissingExitTargetRoot
    Assert-Equals $availableMissingExitTarget.ExitCode 1 "available missing exit target exit code mismatch"
    Assert-Contains $availableMissingExitTarget.Output "available exit target does not exist in Bazel BUILD files"

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

    $missingEvidenceKindRegistryRoot = Join-Path $TempRoot "missing-evidence-kind-registry"
    Write-MinimalRepo -Root $missingEvidenceKindRegistryRoot -MissingEvidenceKindRegistry
    $missingEvidenceKindRegistry = Invoke-Checker -Root $missingEvidenceKindRegistryRoot
    Assert-Equals $missingEvidenceKindRegistry.ExitCode 1 "missing evidence kind registry exit code mismatch"
    Assert-Contains $missingEvidenceKindRegistry.Output "transition ratchet policy must declare evidence_kind_registry"

    $missingRegisteredEvidenceKindRoot = Join-Path $TempRoot "missing-registered-evidence-kind"
    Write-MinimalRepo -Root $missingRegisteredEvidenceKindRoot -MissingRegisteredEvidenceKind
    $missingRegisteredEvidenceKind = Invoke-Checker -Root $missingRegisteredEvidenceKindRoot
    Assert-Equals $missingRegisteredEvidenceKind.ExitCode 1 "missing registered evidence kind exit code mismatch"
    Assert-Contains $missingRegisteredEvidenceKind.Output "evidence kind is not registered"

    $duplicateEvidenceKindRegistryRoot = Join-Path $TempRoot "duplicate-evidence-kind-registry"
    Write-MinimalRepo -Root $duplicateEvidenceKindRegistryRoot -DuplicateEvidenceKindRegistry
    $duplicateEvidenceKindRegistry = Invoke-Checker -Root $duplicateEvidenceKindRegistryRoot
    Assert-Equals $duplicateEvidenceKindRegistry.ExitCode 1 "duplicate evidence kind registry exit code mismatch"
    Assert-Contains $duplicateEvidenceKindRegistry.Output "transition ratchet evidence kind duplicate"

    $missingPlannedEvidenceBlockerRegistryRoot = Join-Path $TempRoot "missing-planned-evidence-blocker-registry"
    Write-MinimalRepo -Root $missingPlannedEvidenceBlockerRegistryRoot -MissingPlannedEvidenceBlockerRegistry
    $missingPlannedEvidenceBlockerRegistry = Invoke-Checker -Root $missingPlannedEvidenceBlockerRegistryRoot
    Assert-Equals $missingPlannedEvidenceBlockerRegistry.ExitCode 1 "missing planned evidence blocker registry exit code mismatch"
    Assert-Contains $missingPlannedEvidenceBlockerRegistry.Output "transition ratchet policy must declare planned_evidence_blocker_registry"

    $missingRegisteredPlannedEvidenceBlockerRoot = Join-Path $TempRoot "missing-registered-planned-evidence-blocker"
    Write-MinimalRepo -Root $missingRegisteredPlannedEvidenceBlockerRoot -MissingRegisteredPlannedEvidenceBlocker
    $missingRegisteredPlannedEvidenceBlocker = Invoke-Checker -Root $missingRegisteredPlannedEvidenceBlockerRoot
    Assert-Equals $missingRegisteredPlannedEvidenceBlocker.ExitCode 1 "missing registered planned evidence blocker exit code mismatch"
    Assert-Contains $missingRegisteredPlannedEvidenceBlocker.Output "planned evidence blocker is not registered"

    $duplicatePlannedEvidenceBlockerRegistryRoot = Join-Path $TempRoot "duplicate-planned-evidence-blocker-registry"
    Write-MinimalRepo -Root $duplicatePlannedEvidenceBlockerRegistryRoot -DuplicatePlannedEvidenceBlockerRegistry
    $duplicatePlannedEvidenceBlockerRegistry = Invoke-Checker -Root $duplicatePlannedEvidenceBlockerRegistryRoot
    Assert-Equals $duplicatePlannedEvidenceBlockerRegistry.ExitCode 1 "duplicate planned evidence blocker registry exit code mismatch"
    Assert-Contains $duplicatePlannedEvidenceBlockerRegistry.Output "transition ratchet planned evidence blocker duplicate"

    $missingPlannedEvidenceBlockedByRoot = Join-Path $TempRoot "missing-planned-evidence-blocked-by"
    Write-MinimalRepo -Root $missingPlannedEvidenceBlockedByRoot -MissingPlannedEvidenceBlockedBy
    $missingPlannedEvidenceBlockedBy = Invoke-Checker -Root $missingPlannedEvidenceBlockedByRoot
    Assert-Equals $missingPlannedEvidenceBlockedBy.ExitCode 1 "missing planned evidence blocked_by exit code mismatch"
    Assert-Contains $missingPlannedEvidenceBlockedBy.Output "planned evidence_status blocked_by"

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
