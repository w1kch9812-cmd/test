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
        [switch] $InvalidApprovalGateDecisionReference,
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
        [switch] $MissingGuardrailNoCacheTag,
        [switch] $MissingGuardrailExternalTag,
        [switch] $InvalidExitTarget,
        [switch] $TransitionExitTarget,
        [switch] $RetiredRustfmtTransition,
        [switch] $UntrackedCiTransition,
        [switch] $UnreferencedTransitionPolicy,
        [switch] $MissingWorkflowCommandProvisioning,
        [switch] $MissingWorkflowServiceProvisioning,
        [switch] $MissingExitState,
        [switch] $UnknownExitState,
        [switch] $MissingTransitionExitStateRegistry,
        [switch] $MissingRegisteredTransitionExitState,
        [switch] $DuplicateTransitionExitStateRegistry,
        [switch] $MissingExitEvidenceRequirements,
        [switch] $MissingBlockingApprovalGate,
        [switch] $MissingExitTargetRegistry,
        [switch] $MissingRegisteredExitTarget,
        [switch] $MissingExitEvidenceTargetRegistry,
        [switch] $MissingRegisteredExitEvidenceTarget,
        [switch] $InvalidPlannedExitEvidenceTarget,
        [switch] $TransitionPlannedExitEvidenceTarget,
        [switch] $MissingExitTargetStateRegistry,
        [switch] $MissingRegisteredExitTargetState,
        [switch] $DuplicateExitTargetStateRegistry,
        [switch] $AvailableMissingExitTarget,
        [switch] $MissingExitTargetEvidenceStatus,
        [switch] $AvailableMissingEvidenceTarget,
        [switch] $MismatchedExitTargetEvidence,
        [switch] $MissingApprovalGateRegistry,
        [switch] $MissingRegisteredApprovalGate,
        [switch] $DuplicateApprovalGateRegistry,
        [switch] $MissingTransitionCategoryRegistry,
        [switch] $MissingRegisteredTransitionCategory,
        [switch] $MismatchedCategoryEvidence,
        [switch] $MissingEvidenceKindRegistry,
        [switch] $MissingRegisteredEvidenceKind,
        [switch] $DuplicateEvidenceKindRegistry,
        [switch] $MissingPlannedEvidenceBlockerRegistry,
        [switch] $MissingRegisteredPlannedEvidenceBlocker,
        [switch] $DuplicatePlannedEvidenceBlockerRegistry,
        [switch] $MissingPlannedEvidenceBlockedBy,
        [switch] $ExtraUncoveredExitBlockingGate,
        [switch] $MissingExitEvidenceRequirementRegistry,
        [switch] $MissingRegisteredExitEvidenceRequirement,
        [switch] $DuplicateExitEvidenceRequirementRegistry
    )

    $FixtureRoot = Join-Path $PSScriptRoot "transition-ratchet-bazel.tests"
    . (Join-Path $FixtureRoot "fixture-bazel-files.ps1")
    . (Join-Path $FixtureRoot "fixture-registry-core-values.ps1")
    . (Join-Path $FixtureRoot "fixture-registry-governance-values.ps1")
    . (Join-Path $FixtureRoot "fixture-registry-runner-values.ps1")
    . (Join-Path $FixtureRoot "fixture-registry-exit-target-values.ps1")
    . (Join-Path $FixtureRoot "fixture-transition-policy-doc.ps1")
    . (Join-Path $FixtureRoot "fixture-ci-workflow.ps1")
}
