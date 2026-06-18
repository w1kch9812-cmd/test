function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker {
    param(
        [string] $Root,
        [switch] $IncludeProductionPromotion
    )
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    if ($IncludeProductionPromotion) {
        $arguments += "-IncludeProductionPromotion"
    }
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
        [switch] $OmitCiWiring,
        [switch] $OmitViteOverride,
        [switch] $OmitSupplyChainProvenance,
        [switch] $OmitSupplyChainCi,
        [switch] $OmitMigrationPrefixCi,
        [switch] $OmitDeployCandidateVerifier,
        [switch] $OmitDeployGateRunbook,
        [switch] $OmitDeployAdmissionWorkflow,
        [switch] $OmitProductionEdgeAdmission,
        [switch] $OmitLoadEvidenceAdmission,
        [switch] $OmitMigrationPrefixGuardrail,
        [switch] $OmitMigrationPrefixLefthook,
        [switch] $OmitDefaultDenyIdentityRuntime,
        [switch] $OmitCatalogRuntimeSurfaces,
        [switch] $OmitWorkloadIdentityTokenFileSupport,
        [switch] $LegacyCiGeneratedSbomOutput,
        [switch] $ExpiredException
    )
    $FixtureRoot = Join-Path $PSScriptRoot "integration-policy-platform.tests"
    . (Join-Path $FixtureRoot "fixture-index-route-call-service.ps1")
    . (Join-Path $FixtureRoot "fixture-webhook-supply-chain.ps1")
    . (Join-Path $FixtureRoot "fixture-operations-exceptions-boundary.ps1")
    . (Join-Path $FixtureRoot "fixture-package-ci.ps1")
    . (Join-Path $FixtureRoot "fixture-production-promotion.ps1")
    . (Join-Path $FixtureRoot "fixture-runtime-docs-events.ps1")
}
