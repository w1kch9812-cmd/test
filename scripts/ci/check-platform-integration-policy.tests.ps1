Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-integration-policy.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-integration-policy-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

. (Join-Path $PSScriptRoot "integration-policy-platform.tests.helpers.ps1")

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

$checkerModuleRoot = Join-Path $PSScriptRoot "integration-policy-platform"
Get-ChildItem -LiteralPath $checkerModuleRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

$testHelperPath = Join-Path $PSScriptRoot "integration-policy-platform.tests.helpers.ps1"
Assert-FileLineCountAtMost -Path $testHelperPath -MaxLines 600

$testFixtureRoot = Join-Path $PSScriptRoot "integration-policy-platform.tests"
Get-ChildItem -LiteralPath $testFixtureRoot -File -Filter "*.ps1" -Recurse |
    ForEach-Object {
        Assert-FileLineCountAtMost -Path $_.FullName -MaxLines 600
    }

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    if ($success.ExitCode -ne 0) {
        throw "successful checker exit code mismatch expected='0' actual='$($success.ExitCode)': $($success.Output)"
    }
    Assert-Contains $success.Output "platform-integration-policy-ok"

    $coreOnlyRoot = Join-Path $TempRoot "core-without-production-promotion"
    Write-MinimalRepo `
        -Root $coreOnlyRoot `
        -OmitDeployCandidateVerifier `
        -OmitDeployGateRunbook `
        -OmitDeployAdmissionWorkflow `
        -OmitProductionEdgeAdmission `
        -OmitLoadEvidenceAdmission
    $coreOnly = Invoke-Checker -Root $coreOnlyRoot
    Assert-Equals $coreOnly.ExitCode 0 "core platform integration checker must not require production promotion gates"
    Assert-Contains $coreOnly.Output "platform-integration-policy-ok"

    $missingCiRoot = Join-Path $TempRoot "missing-ci"
    Write-MinimalRepo -Root $missingCiRoot -OmitCiWiring
    $missingCi = Invoke-Checker -Root $missingCiRoot
    Assert-Equals $missingCi.ExitCode 1 "missing CI wiring exit code mismatch"
    Assert-Contains $missingCi.Output "check-platform-integration-policy.ps1"

    $missingOverrideRoot = Join-Path $TempRoot "missing-override"
    Write-MinimalRepo -Root $missingOverrideRoot -OmitViteOverride
    $missingOverride = Invoke-Checker -Root $missingOverrideRoot
    Assert-Equals $missingOverride.ExitCode 1 "missing override exit code mismatch"
    Assert-Contains $missingOverride.Output "pnpm override mismatch for vite"

    $missingSupplyChainRoot = Join-Path $TempRoot "missing-supply-chain-provenance"
    Write-MinimalRepo -Root $missingSupplyChainRoot -OmitSupplyChainProvenance
    $missingSupplyChain = Invoke-Checker -Root $missingSupplyChainRoot
    Assert-Equals $missingSupplyChain.ExitCode 1 "missing supply chain provenance exit code mismatch"
    Assert-Contains $missingSupplyChain.Output "supply chain provenance requirement mismatch"

    $missingSupplyChainCiRoot = Join-Path $TempRoot "missing-supply-chain-ci"
    Write-MinimalRepo -Root $missingSupplyChainCiRoot -OmitSupplyChainCi
    $missingSupplyChainCi = Invoke-Checker -Root $missingSupplyChainCiRoot
    Assert-Equals $missingSupplyChainCi.ExitCode 1 "missing supply chain CI exit code mismatch"
    Assert-Contains $missingSupplyChainCi.Output "CI required jobs or steps"

    $ciGeneratedSbomRoot = Join-Path $TempRoot "ci-generated-sbom-output"
    Write-MinimalRepo -Root $ciGeneratedSbomRoot -LegacyCiGeneratedSbomOutput
    $ciGeneratedSbom = Invoke-Checker -Root $ciGeneratedSbomRoot
    Assert-Equals $ciGeneratedSbom.ExitCode 1 "CI-generated SBOM output path exit code mismatch"
    Assert-Contains $ciGeneratedSbom.Output "supply chain SBOM output_file must be a Bazel output"

    $missingMigrationPrefixCiRoot = Join-Path $TempRoot "missing-migration-prefix-ci"
    Write-MinimalRepo -Root $missingMigrationPrefixCiRoot -OmitMigrationPrefixCi
    $missingMigrationPrefixCi = Invoke-Checker -Root $missingMigrationPrefixCiRoot
    Assert-Equals $missingMigrationPrefixCi.ExitCode 1 "missing migration prefix CI exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixCi.Output `
        "CI required jobs or steps missing 'check-migration-version-prefixes.ps1'"

    $missingDeployAdmissionRoot = Join-Path $TempRoot "missing-deploy-admission"
    Write-MinimalRepo -Root $missingDeployAdmissionRoot -OmitDeployAdmissionWorkflow
    $missingDeployAdmission = Invoke-Checker -Root $missingDeployAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingDeployAdmission.ExitCode 1 "missing deploy admission exit code mismatch"
    Assert-Contains $missingDeployAdmission.Output "production-deploy-admission.yml"

    $missingProductionEdgeAdmissionRoot = Join-Path $TempRoot "missing-production-edge-admission"
    Write-MinimalRepo -Root $missingProductionEdgeAdmissionRoot -OmitProductionEdgeAdmission
    $missingProductionEdgeAdmission = Invoke-Checker -Root $missingProductionEdgeAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingProductionEdgeAdmission.ExitCode 1 "missing production edge admission exit code mismatch"
    Assert-Contains $missingProductionEdgeAdmission.Output "production edge admission"

    $missingLoadEvidenceAdmissionRoot = Join-Path $TempRoot "missing-load-evidence-admission"
    Write-MinimalRepo -Root $missingLoadEvidenceAdmissionRoot -OmitLoadEvidenceAdmission
    $missingLoadEvidenceAdmission = Invoke-Checker -Root $missingLoadEvidenceAdmissionRoot -IncludeProductionPromotion
    Assert-Equals $missingLoadEvidenceAdmission.ExitCode 1 "missing load evidence admission exit code mismatch"
    Assert-Contains $missingLoadEvidenceAdmission.Output "load-test capacity admission"

    $missingMigrationPrefixGuardrailRoot = Join-Path $TempRoot "missing-migration-prefix-guardrail"
    Write-MinimalRepo -Root $missingMigrationPrefixGuardrailRoot -OmitMigrationPrefixGuardrail
    $missingMigrationPrefixGuardrail = Invoke-Checker -Root $missingMigrationPrefixGuardrailRoot
    Assert-Equals `
        $missingMigrationPrefixGuardrail.ExitCode `
        1 `
        "missing migration prefix guardrail exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixGuardrail.Output `
        "index required guardrails missing 'scripts/ci/check-migration-version-prefixes.ps1'"

    $missingMigrationPrefixLefthookRoot = Join-Path $TempRoot "missing-migration-prefix-lefthook"
    Write-MinimalRepo -Root $missingMigrationPrefixLefthookRoot -OmitMigrationPrefixLefthook
    $missingMigrationPrefixLefthook = Invoke-Checker -Root $missingMigrationPrefixLefthookRoot
    Assert-Equals $missingMigrationPrefixLefthook.ExitCode 1 "missing migration prefix lefthook exit code mismatch"
    Assert-Contains `
        $missingMigrationPrefixLefthook.Output `
        "lefthook migration prefix gate"

    $expiredExceptionRoot = Join-Path $TempRoot "expired-exception"
    Write-MinimalRepo -Root $expiredExceptionRoot -ExpiredException
    $expiredException = Invoke-Checker -Root $expiredExceptionRoot
    Assert-Equals $expiredException.ExitCode 1 "expired exception exit code mismatch"
    Assert-Contains $expiredException.Output "expired_at"

    $missingDefaultDenyIdentityRuntimeRoot = Join-Path $TempRoot "missing-default-deny-identity-runtime"
    Write-MinimalRepo -Root $missingDefaultDenyIdentityRuntimeRoot -OmitDefaultDenyIdentityRuntime
    $missingDefaultDenyIdentityRuntime = Invoke-Checker -Root $missingDefaultDenyIdentityRuntimeRoot
    Assert-Equals $missingDefaultDenyIdentityRuntime.ExitCode 1 "missing default-deny identity runtime exit code mismatch"
    Assert-Contains $missingDefaultDenyIdentityRuntime.Output "default-deny identity runtime"

    $missingCatalogRuntimeSurfacesRoot = Join-Path $TempRoot "missing-catalog-runtime-surfaces"
    Write-MinimalRepo -Root $missingCatalogRuntimeSurfacesRoot -OmitCatalogRuntimeSurfaces
    $missingCatalogRuntimeSurfaces = Invoke-Checker -Root $missingCatalogRuntimeSurfacesRoot
    Assert-Equals $missingCatalogRuntimeSurfaces.ExitCode 1 "missing catalog runtime surfaces exit code mismatch"
    Assert-Contains $missingCatalogRuntimeSurfaces.Output "catalog runtime surface"

    $missingWorkloadIdentityTokenFileRoot = Join-Path $TempRoot "missing-workload-identity-token-file"
    Write-MinimalRepo -Root $missingWorkloadIdentityTokenFileRoot -OmitWorkloadIdentityTokenFileSupport
    $missingWorkloadIdentityTokenFile = Invoke-Checker -Root $missingWorkloadIdentityTokenFileRoot
    Assert-Equals $missingWorkloadIdentityTokenFile.ExitCode 1 "missing workload identity token file exit code mismatch"
    Assert-Contains $missingWorkloadIdentityTokenFile.Output "workload identity token file"

    Write-Host "platform-integration-policy-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
