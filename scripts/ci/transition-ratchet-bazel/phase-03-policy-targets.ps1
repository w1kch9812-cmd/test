foreach ($entry in $policyEntries) {
    $context = "transition policy"
    $target = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "transition policy target must be a Bazel _transition label: $target"
    }
    $category = Get-RequiredString -Object $entry -Name "category" -Context "transition policy $target"
    if (!$transitionCategoryById.ContainsKey($category)) {
        throw "transition category is not registered: $target -> $category"
    }
    $registeredCategory = $transitionCategoryById[$category]
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition policy $target")
    $exitTarget = Get-RequiredString -Object $entry -Name "exit_target" -Context "transition policy $target"
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "transition policy exit_target must be a Bazel label: $target -> $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "transition policy exit_target must not be another transition: $target -> $exitTarget"
    }
    if (!$exitTargetByLabel.ContainsKey($exitTarget)) {
        throw "transition exit_target is not registered: $target -> $exitTarget"
    }
    $registeredExitTarget = $exitTargetByLabel[$exitTarget]
    $exitState = Get-RequiredString -Object $entry -Name "exit_state" -Context "transition policy $target"
    if (!$allowedExitStates.ContainsKey($exitState)) {
        throw "transition exit_state is not registered for ${target}: $exitState"
    }
    $exitEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "exit_evidence_requirements" `
        -Context "transition policy $target")
    Assert-Unique -Values $exitEvidenceRequirements -Message "transition policy $target exit evidence requirement"
    foreach ($exitEvidenceRequirement in $exitEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($exitEvidenceRequirement)) {
            throw "transition exit evidence requirement is not registered for ${target}: $exitEvidenceRequirement"
        }
    }
    Assert-ContainsAll `
        -Actual $exitEvidenceRequirements `
        -Expected @($registeredCategory.required_exit_evidence_requirements | ForEach-Object { [string] $_ }) `
        -Message "transition category required_exit_evidence_requirements for $target"
    Assert-ContainsAll `
        -Actual @($registeredExitTarget.exit_evidence_requirements | ForEach-Object { [string] $_ }) `
        -Expected $exitEvidenceRequirements `
        -Message "exit target registry exit_evidence_requirements for $exitTarget"
    $blockingApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "blocking_approval_gates" `
        -Context "transition policy $target")
    Assert-Unique -Values $blockingApprovalGates -Message "transition policy $target blocking approval gate"
    foreach ($blockingApprovalGate in $blockingApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($blockingApprovalGate)) {
            throw "transition blocking approval gate is not registered for ${target}: $blockingApprovalGate"
        }
    }
    Assert-ContainsAll `
        -Actual @($registeredExitTarget.blocking_approval_gates | ForEach-Object { [string] $_ }) `
        -Expected $blockingApprovalGates `
        -Message "exit target registry blocking_approval_gates for $exitTarget"
    $registeredExitTargetState = [string] $registeredExitTarget.state
    if ($exitState -eq "blocked" -and $registeredExitTargetState -ne "planned") {
        throw "blocked transition must point at a planned exit target: $target -> $exitTarget"
    }
    if ($exitState -eq "ready_to_retire" -and $registeredExitTargetState -ne "available") {
        throw "ready_to_retire transition must point at an available exit target: $target -> $exitTarget"
    }
    $runnerScript = Get-RequiredString -Object $entry -Name "runner_script" -Context "transition policy $target"
    $runnerTask = Get-RequiredString -Object $entry -Name "runner_task" -Context "transition policy $target"
    if (!$runnerTaskRequirements.ContainsKey($runnerTask)) {
        throw "runner task is not registered: ${target} -> $runnerTask"
    }
    $requiredCommands = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_commands" `
        -Context "transition policy $target")
    Assert-Unique -Values $requiredCommands -Message "transition policy $target required command"
    foreach ($requiredCommand in $requiredCommands) {
        if (!$allowedRequiredCommands.ContainsKey($requiredCommand)) {
            throw "required command is not registered for ${target}: $requiredCommand"
        }
    }
    $requiredServices = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_services" `
        -Context "transition policy $target")
    Assert-Unique -Values $requiredServices -Message "transition policy $target required service"
    foreach ($requiredService in $requiredServices) {
        if (!$allowedRequiredServices.ContainsKey($requiredService)) {
            throw "required service is not registered for ${target}: $requiredService"
        }
    }
    $runnerRequirements = $runnerTaskRequirements[$runnerTask]
    Assert-ContainsAll `
        -Actual $requiredCommands `
        -Expected @($runnerRequirements.Commands) `
        -Message "transition policy required_commands for $target"
    Assert-ContainsAll `
        -Actual $requiredServices `
        -Expected @($runnerRequirements.Services) `
        -Message "transition policy required_services for $target"
    $approvalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "approval_gates" `
        -Context "transition policy $target")
    Assert-Unique -Values $approvalGates -Message "transition policy $target approval gate"
    foreach ($approvalGate in $approvalGates) {
        if (!$allowedApprovalGates.ContainsKey($approvalGate)) {
            throw "transition approval gate is not registered for ${target}: $approvalGate"
        }
    }
    $sunset = Get-RequiredString -Object $entry -Name "sunset" -Context "transition policy $target"
    $sunsetDate = [DateTime]::ParseExact(
        $sunset,
        "yyyy-MM-dd",
        [System.Globalization.CultureInfo]::InvariantCulture
    )
    if ($sunsetDate.Date -lt [DateTime]::UtcNow.Date) {
        throw "expired transition sunset for ${target}: $sunset"
    }
    $externalCollection = Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "transition policy $target"
    Assert-ContainsAll `
        -Actual $approvalGates `
        -Expected @($registeredCategory.required_approval_gates | ForEach-Object { [string] $_ }) `
        -Message "transition category required_approval_gates for $target"
    $categoryExternalCollectionApprovalRequired = [bool] $registeredCategory.external_collection_approval_required
    if ($categoryExternalCollectionApprovalRequired -and !$externalCollection) {
        throw "transition category requires external collection approval: $target -> $category"
    }
    $hasExternalCollectionApprovalGate = $false
    foreach ($approvalGate in $approvalGates) {
        if ($externalCollectionApprovalGateSet.ContainsKey($approvalGate)) {
            $hasExternalCollectionApprovalGate = $true
        }
    }
    if ($externalCollection -and !$hasExternalCollectionApprovalGate) {
        throw "external collection transition must declare external advisory approval gate: $target"
    }
    foreach ($approvalGate in $approvalGates) {
        if ($externalCollectionApprovalGateSet.ContainsKey($approvalGate) -and !$externalCollection) {
            throw "external advisory approval gate must require external collection approval: $target"
        }
    }
    foreach ($approvalGate in $approvalGates) {
        if (!(Test-ContainsValue -Values $blockingApprovalGates -Expected $approvalGate)) {
            throw "transition policy blocking_approval_gates for $target missing '$approvalGate'"
        }
    }
    foreach ($blockingApprovalGate in $blockingApprovalGates) {
        if (!(Test-ContainsValue -Values $approvalGates -Expected $blockingApprovalGate)) {
            throw "transition policy blocking_approval_gates for $target must be declared in approval_gates: $blockingApprovalGate"
        }
    }
    if ($exitState -eq "ready_to_retire" -and $approvalGates.Count -gt 0) {
        throw "ready_to_retire transition must not have unresolved approval_gates: $target"
    }
    $policyByTarget[$target] = $entry
}
