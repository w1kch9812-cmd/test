$policyByTarget = @{}
$exitTargetByLabel = @{}
$transitionCategoryById = @{}
$allowedApprovalGates = @{}
$externalCollectionApprovalGateSet = @{}
foreach ($entry in $approvalGateRegistryEntries) {
    $context = "approval gate registry"
    $approvalGate = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($approvalGate -notmatch '^[a-z][a-z0-9_]*$') {
        throw "approval gate registry id must be lowercase snake_case: $approvalGate"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "approval gate registry $approvalGate")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "approval gate registry $approvalGate")
    $decisionReference = Get-RequiredString `
        -Object $entry `
        -Name "decision_reference" `
        -Context "approval gate registry $approvalGate"
    if ($decisionReference -notmatch '^docs/[A-Za-z0-9_./-]+\.(md|json|jsonc)$') {
        throw "approval gate decision_reference must point to a docs file: $approvalGate -> $decisionReference"
    }
    $decisionReferencePath = Resolve-RepoPath -RelativePath $decisionReference
    if (!(Test-Path -LiteralPath $decisionReferencePath -PathType Leaf)) {
        throw "approval gate decision_reference file is missing: $approvalGate -> $decisionReference"
    }
    $requiresHumanApproval = Get-RequiredBoolean `
        -Object $entry `
        -Name "requires_human_approval" `
        -Context "approval gate registry $approvalGate"
    if (!$requiresHumanApproval) {
        throw "approval gate registry $approvalGate requires_human_approval must be true"
    }
    $externalCollectionApprovalRequired = Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "approval gate registry $approvalGate"
    $allowedApprovalGates[$approvalGate] = $true
    if ($externalCollectionApprovalRequired) {
        $externalCollectionApprovalGateSet[$approvalGate] = $true
    }
}
if ($externalCollectionApprovalGateSet.Count -eq 0) {
    throw "approval_gate_registry must declare an external collection approval gate"
}
$allowedRequiredCommands = @{}
foreach ($entry in $requiredCommandRegistryEntries) {
    $context = "required command registry"
    $requiredCommand = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($requiredCommand -notmatch '^[a-z][a-z0-9_-]*$') {
        throw "required command registry id must be lowercase command token: $requiredCommand"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "required command registry $requiredCommand")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "required command registry $requiredCommand")
    $allowedRequiredCommands[$requiredCommand] = $true
}
$allowedRequiredServices = @{}
foreach ($entry in $requiredServiceRegistryEntries) {
    $context = "required service registry"
    $requiredService = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($requiredService -notmatch '^[a-z][a-z0-9-]*$') {
        throw "required service registry id must be lowercase kebab-case: $requiredService"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "required service registry $requiredService")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "required service registry $requiredService")
    $allowedRequiredServices[$requiredService] = $true
}
$runnerTaskRequirements = @{}
foreach ($entry in $runnerTaskRegistryEntries) {
    $context = "runner task registry"
    $runnerTask = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($runnerTask -notmatch '^[a-z][a-z0-9-]*$') {
        throw "runner task registry id must be lowercase kebab-case: $runnerTask"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "runner task registry $runnerTask")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "runner task registry $runnerTask")
    $registeredRequiredCommands = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_commands" `
        -Context "runner task registry $runnerTask")
    Assert-Unique -Values $registeredRequiredCommands -Message "runner task registry $runnerTask required command"
    foreach ($registeredRequiredCommand in $registeredRequiredCommands) {
        if (!$allowedRequiredCommands.ContainsKey($registeredRequiredCommand)) {
            throw "required command is not registered for ${runnerTask}: $registeredRequiredCommand"
        }
    }
    $registeredRequiredServices = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_services" `
        -Context "runner task registry $runnerTask")
    Assert-Unique -Values $registeredRequiredServices -Message "runner task registry $runnerTask required service"
    foreach ($registeredRequiredService in $registeredRequiredServices) {
        if (!$allowedRequiredServices.ContainsKey($registeredRequiredService)) {
            throw "required service is not registered for ${runnerTask}: $registeredRequiredService"
        }
    }
    $runnerTaskRequirements[$runnerTask] = [pscustomobject]@{
        Commands = $registeredRequiredCommands
        Services = $registeredRequiredServices
    }
}
$allowedExitStates = @{}
foreach ($entry in $transitionExitStateRegistryEntries) {
    $context = "transition exit state registry"
    $transitionExitState = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($transitionExitState -notmatch '^[a-z][a-z0-9_]*$') {
        throw "transition exit state registry id must be lowercase snake_case: $transitionExitState"
    }
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "owner" `
        -Context "transition exit state registry $transitionExitState")
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "reason" `
        -Context "transition exit state registry $transitionExitState")
    $allowedExitStates[$transitionExitState] = $true
}
$allowedExitEvidenceRequirements = @{}
$allowedExitEvidenceKinds = @{}
foreach ($entry in $evidenceKindRegistryEntries) {
    $context = "evidence kind registry"
    $evidenceKind = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($evidenceKind -notmatch '^[a-z][a-z0-9_]*$') {
        throw "evidence kind registry id must be lowercase snake_case: $evidenceKind"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "evidence kind registry $evidenceKind")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "evidence kind registry $evidenceKind")
    $allowedExitEvidenceKinds[$evidenceKind] = $true
}
foreach ($entry in $exitEvidenceRequirementEntries) {
    $context = "exit evidence requirement registry"
    $evidenceRequirement = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($evidenceRequirement -notmatch '^[a-z][a-z0-9_]*$') {
        throw "exit evidence requirement registry id must be lowercase snake_case: $evidenceRequirement"
    }
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "owner" `
        -Context "exit evidence requirement registry $evidenceRequirement")
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "reason" `
        -Context "exit evidence requirement registry $evidenceRequirement")
    $evidenceKind = Get-RequiredString `
        -Object $entry `
        -Name "evidence_kind" `
        -Context "exit evidence requirement registry $evidenceRequirement"
    if (!$allowedExitEvidenceKinds.ContainsKey($evidenceKind)) {
        throw "evidence kind is not registered for ${evidenceRequirement}: $evidenceKind"
    }
    $allowedExitEvidenceRequirements[$evidenceRequirement] = $true
}
$exitEvidenceTargetByKey = @{}
foreach ($entry in $exitEvidenceTargetEntries) {
    $context = "exit evidence target registry"
    $exitTarget = Get-RequiredString -Object $entry -Name "exit_target" -Context $context
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "exit evidence target registry exit_target must be a Bazel label: $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "exit evidence target registry exit_target must not be a transition: $exitTarget"
    }
    $requirement = Get-RequiredString -Object $entry -Name "requirement" -Context "$context $exitTarget"
    if (!$allowedExitEvidenceRequirements.ContainsKey($requirement)) {
        throw "exit evidence requirement is not registered for exit evidence target ${exitTarget}: $requirement"
    }
    $plannedTarget = Get-RequiredString -Object $entry -Name "planned_bazel_target" -Context "$context $exitTarget $requirement"
    if ($plannedTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "planned exit evidence target must be a Bazel label: $exitTarget -> $requirement -> $plannedTarget"
    }
    if ($plannedTarget -match '_transition$') {
        throw "planned exit evidence target must not be a transition: $exitTarget -> $requirement -> $plannedTarget"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "$context $exitTarget $requirement")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "$context $exitTarget $requirement")
    $exitEvidenceTargetByKey["$exitTarget|$requirement"] = $entry
}
$plannedEvidenceBlockerById = @{}
foreach ($entry in $plannedEvidenceBlockerEntries) {
    $context = "planned evidence blocker registry"
    $plannedEvidenceBlocker = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($plannedEvidenceBlocker -notmatch '^[a-z][a-z0-9_]*$') {
        throw "planned evidence blocker registry id must be lowercase snake_case: $plannedEvidenceBlocker"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "planned evidence blocker registry $plannedEvidenceBlocker")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "planned evidence blocker registry $plannedEvidenceBlocker")
    $hasApprovalGate = $entry.PSObject.Properties.Name -contains "approval_gate"
    $hasEvidenceRequirement = $entry.PSObject.Properties.Name -contains "exit_evidence_requirement"
    if ($hasApprovalGate -eq $hasEvidenceRequirement) {
        throw "planned evidence blocker must declare exactly one of approval_gate or exit_evidence_requirement: $plannedEvidenceBlocker"
    }
    if ($hasApprovalGate) {
        $approvalGate = Get-RequiredString `
            -Object $entry `
            -Name "approval_gate" `
            -Context "planned evidence blocker registry $plannedEvidenceBlocker"
        if (!$allowedApprovalGates.ContainsKey($approvalGate)) {
            throw "planned evidence blocker approval_gate is not registered for ${plannedEvidenceBlocker}: $approvalGate"
        }
    }
    if ($hasEvidenceRequirement) {
        $blockedRequirement = Get-RequiredString `
            -Object $entry `
            -Name "exit_evidence_requirement" `
            -Context "planned evidence blocker registry $plannedEvidenceBlocker"
        if (!$allowedExitEvidenceRequirements.ContainsKey($blockedRequirement)) {
            throw "planned evidence blocker exit evidence requirement is not registered for ${plannedEvidenceBlocker}: $blockedRequirement"
        }
    }
    $plannedEvidenceBlockerById[$plannedEvidenceBlocker] = $entry
}
foreach ($entry in $transitionCategoryEntries) {
    $context = "transition category registry"
    $category = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($category -notmatch '^[a-z][a-z0-9-]*$') {
        throw "transition category registry id must be lowercase kebab-case: $category"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition category registry $category")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition category registry $category")
    $categoryEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_exit_evidence_requirements" `
        -Context "transition category registry $category")
    Assert-Unique -Values $categoryEvidenceRequirements -Message "transition category registry $category evidence requirement"
    foreach ($categoryEvidenceRequirement in $categoryEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($categoryEvidenceRequirement)) {
            throw "transition category exit evidence requirement is not registered for ${category}: $categoryEvidenceRequirement"
        }
    }
    $categoryApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_approval_gates" `
        -Context "transition category registry $category")
    Assert-Unique -Values $categoryApprovalGates -Message "transition category registry $category approval gate"
    foreach ($categoryApprovalGate in $categoryApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($categoryApprovalGate)) {
            throw "transition category approval gate is not registered for ${category}: $categoryApprovalGate"
        }
    }
    [void] (Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "transition category registry $category")
    $transitionCategoryById[$category] = $entry
}
$allowedExitTargetStates = @{}
foreach ($entry in $exitTargetStateRegistryEntries) {
    $context = "exit target state registry"
    $exitTargetState = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($exitTargetState -notmatch '^[a-z][a-z0-9_]*$') {
        throw "exit target state registry id must be lowercase snake_case: $exitTargetState"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "exit target state registry $exitTargetState")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "exit target state registry $exitTargetState")
    $allowedExitTargetStates[$exitTargetState] = $true
}
$coveredExitEvidenceTargetKeys = @{}
foreach ($entry in $exitTargetEntries) {
    $context = "exit target registry"
    $exitTarget = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "exit target registry target must be a Bazel label: $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "exit target registry target must not be a transition: $exitTarget"
    }
    $exitTargetState = Get-RequiredString -Object $entry -Name "state" -Context "exit target registry $exitTarget"
    if (!$allowedExitTargetStates.ContainsKey($exitTargetState)) {
        throw "exit target state is not registered for ${exitTarget}: $exitTargetState"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "exit target registry $exitTarget")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "exit target registry $exitTarget")
    $exitTargetEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "exit_evidence_requirements" `
        -Context "exit target registry $exitTarget")
    Assert-Unique -Values $exitTargetEvidenceRequirements -Message "exit target registry $exitTarget evidence requirement"
    foreach ($exitTargetEvidenceRequirement in $exitTargetEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($exitTargetEvidenceRequirement)) {
            throw "exit target exit evidence requirement is not registered for ${exitTarget}: $exitTargetEvidenceRequirement"
        }
    }
    $exitTargetBlockingApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "blocking_approval_gates" `
        -Context "exit target registry $exitTarget")
    Assert-Unique -Values $exitTargetBlockingApprovalGates -Message "exit target registry $exitTarget blocking approval gate"
    foreach ($exitTargetBlockingApprovalGate in $exitTargetBlockingApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($exitTargetBlockingApprovalGate)) {
            throw "exit target approval gate is not registered for ${exitTarget}: $exitTargetBlockingApprovalGate"
        }
    }
    if (!($entry.PSObject.Properties.Name -contains "evidence_status")) {
        throw "exit target registry evidence_status missing for $exitTarget"
    }
    $evidenceStatusEntries = @($entry.evidence_status)
    $evidenceStatusRequirements = @()
    $coveredApprovalBlockers = @{}
    $hasPlannedEvidence = $false
    foreach ($evidenceStatus in $evidenceStatusEntries) {
        $context = "exit target registry evidence_status for $exitTarget"
        $requirement = Get-RequiredString -Object $evidenceStatus -Name "requirement" -Context $context
        if (!$allowedExitEvidenceRequirements.ContainsKey($requirement)) {
            throw "exit target evidence_status requirement is not registered for ${exitTarget}: $requirement"
        }
        $exitEvidenceTargetKey = "$exitTarget|$requirement"
        if (!$exitEvidenceTargetByKey.ContainsKey($exitEvidenceTargetKey)) {
            throw "exit evidence target registry missing entry: $exitTarget -> $requirement"
        }
        $registeredExitEvidenceTarget = $exitEvidenceTargetByKey[$exitEvidenceTargetKey]
        $coveredExitEvidenceTargetKeys[$exitEvidenceTargetKey] = $true
        $state = Get-RequiredString -Object $evidenceStatus -Name "state" -Context "$context $requirement"
        if (!$allowedExitTargetStates.ContainsKey($state)) {
            throw "exit target evidence_status state is not registered for ${exitTarget}: $state"
        }
        [void] (Get-RequiredString -Object $evidenceStatus -Name "reason" -Context "$context $requirement")
        if ($state -eq "available") {
            if ($evidenceStatus.PSObject.Properties.Name -contains "blocked_by") {
                throw "available evidence_status must not declare blocked_by: $exitTarget -> $requirement"
            }
            $evidenceTarget = Get-RequiredString `
                -Object $evidenceStatus `
                -Name "bazel_target" `
                -Context "$context $requirement"
            if ($evidenceTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
                throw "exit target evidence_status bazel_target must be a Bazel label: $exitTarget -> $evidenceTarget"
            }
            if ($evidenceTarget -match '_transition$') {
                throw "exit target evidence_status bazel_target must not be a transition: $exitTarget -> $evidenceTarget"
            }
            $registeredEvidenceTarget = [string] $registeredExitEvidenceTarget.planned_bazel_target
            if ($registeredEvidenceTarget -ne $evidenceTarget) {
                throw "available exit evidence target must match registry planned_bazel_target: $exitTarget -> $requirement"
            }
        } else {
            if (!($evidenceStatus.PSObject.Properties.Name -contains "blocked_by")) {
                throw "planned evidence_status blocked_by missing for $exitTarget -> $requirement"
            }
            $plannedBlockers = @(Get-RequiredStringArray `
                -Object $evidenceStatus `
                -Name "blocked_by" `
                -Context "$context $requirement")
            if ($plannedBlockers.Count -eq 0) {
                throw "planned evidence_status blocked_by must not be empty for $exitTarget -> $requirement"
            }
            Assert-Unique -Values $plannedBlockers -Message "planned evidence_status blocker for $exitTarget $requirement"
            foreach ($plannedBlocker in $plannedBlockers) {
                if (!$plannedEvidenceBlockerById.ContainsKey($plannedBlocker)) {
                    throw "planned evidence blocker is not registered for ${exitTarget}: $plannedBlocker"
                }
                $blockerEntry = $plannedEvidenceBlockerById[$plannedBlocker]
                if ($blockerEntry.PSObject.Properties.Name -contains "approval_gate") {
                    $approvalGate = [string] $blockerEntry.approval_gate
                    if (!(Test-ContainsValue -Values $exitTargetBlockingApprovalGates -Expected $approvalGate)) {
                        throw "planned evidence approval blocker must be covered by exit target blocking_approval_gates: $exitTarget -> $plannedBlocker"
                    }
                    $coveredApprovalBlockers[$approvalGate] = $true
                }
                if ($blockerEntry.PSObject.Properties.Name -contains "exit_evidence_requirement") {
                    $blockedRequirement = [string] $blockerEntry.exit_evidence_requirement
                    if ($blockedRequirement -ne $requirement) {
                        throw "planned implementation blocker must match evidence_status requirement: $exitTarget -> $plannedBlocker"
                    }
                }
            }
            $hasPlannedEvidence = $true
        }
        $evidenceStatusRequirements += $requirement
    }
    Assert-Unique -Values $evidenceStatusRequirements -Message "exit target registry $exitTarget evidence_status requirement"
    Assert-ContainsAll `
        -Actual $evidenceStatusRequirements `
        -Expected $exitTargetEvidenceRequirements `
        -Message "exit target registry evidence_status for $exitTarget"
    Assert-ContainsAll `
        -Actual $exitTargetEvidenceRequirements `
        -Expected $evidenceStatusRequirements `
        -Message "exit target registry evidence_status requirements for $exitTarget"
    if ($exitTargetState -eq "available" -and $hasPlannedEvidence) {
        throw "available exit target must not have planned evidence_status: $exitTarget"
    }
    foreach ($blockingApprovalGate in $exitTargetBlockingApprovalGates) {
        if (!$coveredApprovalBlockers.ContainsKey($blockingApprovalGate)) {
            throw "exit target blocking_approval_gates must be covered by planned evidence blockers: $exitTarget -> $blockingApprovalGate"
        }
    }
    $exitTargetByLabel[$exitTarget] = $entry
}
foreach ($key in $exitEvidenceTargetByKey.Keys) {
    if (!$coveredExitEvidenceTargetKeys.ContainsKey($key)) {
        throw "exit evidence target registry has unreferenced entry: $key"
    }
}
