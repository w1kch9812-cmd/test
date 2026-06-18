$policy = Read-JsonFile -RelativePath "docs/architecture/verification-transition-ratchet.v1.json"
Assert-Equals -Actual $policy.schema_version -Expected "gongzzang.verification_transition_ratchet.v1" -Message "transition ratchet schema_version mismatch"
Assert-Equals -Actual $policy.repo_slug -Expected "gongzzang" -Message "transition ratchet repo_slug mismatch"
Assert-Equals `
    -Actual $policy.default_decision `
    -Expected "deny_new_transition_without_policy" `
    -Message "transition ratchet default decision mismatch"

$policyEntries = @($policy.transition_targets)
if ($policyEntries.Count -eq 0) {
    throw "transition ratchet policy must declare at least one transition target"
}
Assert-Unique -Values @($policyEntries | ForEach-Object { [string] $_.bazel_target }) -Message "transition ratchet policy target"

$retiredTransitionTargets = @()
if ($policy.PSObject.Properties.Name -contains "retired_transition_targets") {
    $retiredTransitionTargets = @($policy.retired_transition_targets | ForEach-Object { [string] $_ })
}
Assert-Unique -Values $retiredTransitionTargets -Message "retired transition target"
$retiredTransitionTargetSet = @{}
foreach ($target in $retiredTransitionTargets) {
    if ([string]::IsNullOrWhiteSpace($target)) {
        throw "retired transition target must not be empty"
    }
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "retired transition target must be a Bazel _transition label: $target"
    }
    $retiredTransitionTargetSet[$target] = $true
}

$exitTargetEntries = @()
if ($policy.PSObject.Properties.Name -contains "exit_targets") {
    $exitTargetEntries = @($policy.exit_targets)
}
if ($exitTargetEntries.Count -eq 0) {
    throw "transition ratchet policy must declare exit_targets"
}
Assert-Unique -Values @($exitTargetEntries | ForEach-Object { [string] $_.bazel_target }) -Message "transition ratchet exit target"

$approvalGateRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "approval_gate_registry") {
    $approvalGateRegistryEntries = @($policy.approval_gate_registry)
}
if ($approvalGateRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare approval_gate_registry"
}
Assert-Unique -Values @($approvalGateRegistryEntries | ForEach-Object { [string] $_.id }) -Message "transition ratchet approval gate"

$exitEvidenceRequirementEntries = @()
if ($policy.PSObject.Properties.Name -contains "exit_evidence_requirement_registry") {
    $exitEvidenceRequirementEntries = @($policy.exit_evidence_requirement_registry)
}
if ($exitEvidenceRequirementEntries.Count -eq 0) {
    throw "transition ratchet policy must declare exit_evidence_requirement_registry"
}
Assert-Unique `
    -Values @($exitEvidenceRequirementEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet exit evidence requirement"

$evidenceKindRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "evidence_kind_registry") {
    $evidenceKindRegistryEntries = @($policy.evidence_kind_registry)
}
if ($evidenceKindRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare evidence_kind_registry"
}
Assert-Unique `
    -Values @($evidenceKindRegistryEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet evidence kind"

$transitionCategoryEntries = @()
if ($policy.PSObject.Properties.Name -contains "transition_category_registry") {
    $transitionCategoryEntries = @($policy.transition_category_registry)
}
if ($transitionCategoryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare transition_category_registry"
}
Assert-Unique -Values @($transitionCategoryEntries | ForEach-Object { [string] $_.id }) -Message "transition ratchet category"

$runnerTaskRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "runner_task_registry") {
    $runnerTaskRegistryEntries = @($policy.runner_task_registry)
}
if ($runnerTaskRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare runner_task_registry"
}
Assert-Unique -Values @($runnerTaskRegistryEntries | ForEach-Object { [string] $_.id }) -Message "transition ratchet runner task"

$requiredCommandRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "required_command_registry") {
    $requiredCommandRegistryEntries = @($policy.required_command_registry)
}
if ($requiredCommandRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare required_command_registry"
}
Assert-Unique `
    -Values @($requiredCommandRegistryEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet required command"

$requiredServiceRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "required_service_registry") {
    $requiredServiceRegistryEntries = @($policy.required_service_registry)
}
if ($requiredServiceRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare required_service_registry"
}
Assert-Unique `
    -Values @($requiredServiceRegistryEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet required service"

$exitTargetStateRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "exit_target_state_registry") {
    $exitTargetStateRegistryEntries = @($policy.exit_target_state_registry)
}
if ($exitTargetStateRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare exit_target_state_registry"
}
Assert-Unique `
    -Values @($exitTargetStateRegistryEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet exit target state"

$transitionExitStateRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "transition_exit_state_registry") {
    $transitionExitStateRegistryEntries = @($policy.transition_exit_state_registry)
}
if ($transitionExitStateRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare transition_exit_state_registry"
}
Assert-Unique `
    -Values @($transitionExitStateRegistryEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet transition exit state"
