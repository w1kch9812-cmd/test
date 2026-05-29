Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-dependency-boundary.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-dependency-boundary-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
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

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function New-BoundaryJson {
    param(
        [bool] $StaleAllowance = $false,
        [bool] $OmitAllowanceReason = $false
    )

    $traceFields = if ($OmitAllowanceReason) {
        ""
    } else {
        ',"reason":"legacy catalog read path until Platform Core cutover","exit_criteria":"replace dependency with Platform Core published contract"'
    }
    $allowances = @()
    $staleAllowanceEntry = if ($StaleAllowance) {
        $allowances += '{"manifest":"services/api/Cargo.toml","dependency":"manufacturer-domain","owner":"platform-core","until_phase":"m3_4"' + $traceFields + '}'
    }
    $allowancesJson = $allowances -join ",`n    "
    return @"
{
  "schema_version": "gongzzang.platform_core_boundary.v1",
  "repo_slug": "gongzzang",
  "phase": "m3_2_cutover_preparation",
  "path_ownership": [
    {"path":"crates/domain/core/listing","owner":"gongzzang","classification":"product_domain"},
    {"path":"crates/domain/core/parcel","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/building","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/industrial-complex","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/domain/core/manufacturer","owner":"platform-core","classification":"extracted_catalog_asset"},
    {"path":"crates/data-clients/vworld","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"crates/data-clients/data-go-kr","owner":"platform-core","classification":"extracted_catalog_etl_asset"},
    {"path":"crates/data-clients/raw-capture","owner":"platform-core","classification":"extracted_catalog_raw_asset"},
    {"path":"crates/data-pipeline-control","owner":"platform-core","classification":"extracted_catalog_etl_asset"}
  ],
  "allowed_transitional_dependencies": [
    $allowancesJson
  ]
}
"@
}

function Write-Manifest {
    param([string] $Root, [string] $RelativePath, [string] $PackageName, [string[]] $Dependencies)

    $deps = @($Dependencies | ForEach-Object {
        "$_ = { path = `"../placeholder`", version = `"0.1.0`" }"
    }) -join [Environment]::NewLine

    Write-File -Root $Root -RelativePath $RelativePath -Content @"
[package]
name = "$PackageName"
version = "0.1.0"

[dependencies]
$deps
"@
}

function Write-Fixture {
    param(
        [string] $Root,
        [bool] $ProductDependsOnParcel = $false,
        [bool] $ApiDependsOnParcel = $false,
        [bool] $ApiDependsOnVworld = $false,
        [bool] $ApiDependsOnBuildingDomain = $false,
        [bool] $ApiDependsOnDataGoKr = $false,
        [bool] $ApiDependsOnIndustrialComplex = $false,
        [bool] $DbDependsOnPipelineControl = $false,
        [bool] $DbDependsOnRawCapture = $false,
        [bool] $LegacyToolDependsOnPipelineControl = $false,
        [bool] $ParcelLookupDependsOnParcel = $false,
        [bool] $ParcelLookupDependsOnVworld = $false,
        [bool] $ParcelLookupDependsOnReqwest = $false,
        [bool] $ParcelLookupSourceUsesReqwest = $false,
        [bool] $CircuitBreakerCatalogPolicyPresent = $false,
        [bool] $PlatformCoreOwnedManifestPresent = $false,
        [bool] $StaleAllowance = $false,
        [bool] $OmitAllowanceReason = $false
    )

    Write-File -Root $Root -RelativePath "docs\architecture\platform-core-boundary.v1.json" -Content (New-BoundaryJson -StaleAllowance $StaleAllowance -OmitAllowanceReason $OmitAllowanceReason)

    $listingDeps = @("shared-kernel")
    if ($ProductDependsOnParcel) {
        $listingDeps += "parcel-domain"
    }
    Write-Manifest -Root $Root -RelativePath "crates\domain\core\listing\Cargo.toml" -PackageName "listing-domain" -Dependencies $listingDeps

    $apiDeps = @("listing-domain")
    if ($ApiDependsOnParcel) {
        $apiDeps += "parcel-domain"
    }
    if ($ApiDependsOnVworld) {
        $apiDeps += "vworld-client"
    }
    if ($ApiDependsOnBuildingDomain) {
        $apiDeps += "building-domain"
    }
    if ($ApiDependsOnDataGoKr) {
        $apiDeps += "data-go-kr-client"
    }
    if ($ApiDependsOnIndustrialComplex) {
        $apiDeps += "industrial-complex-domain"
    }
    Write-Manifest -Root $Root -RelativePath "services\api\Cargo.toml" -PackageName "api" -Dependencies $apiDeps

    $parcelLookupDeps = @("shared-kernel")
    if ($ParcelLookupDependsOnParcel) {
        $parcelLookupDeps += "parcel-domain"
    }
    if ($ParcelLookupDependsOnVworld) {
        $parcelLookupDeps += "vworld-client"
    }
    if ($ParcelLookupDependsOnReqwest) {
        $parcelLookupDeps += "reqwest"
    }
    Write-Manifest -Root $Root -RelativePath "crates\parcel-lookup\Cargo.toml" -PackageName "parcel-lookup" -Dependencies $parcelLookupDeps
    $parcelLookupSource = if ($ParcelLookupSourceUsesReqwest) {
        "use reqwest::Client;"
    } else {
        "pub trait ParcelInfoLookup {}"
    }
    Write-File -Root $Root -RelativePath "crates\parcel-lookup\src\lib.rs" -Content $parcelLookupSource
    $dbDeps = @("shared-kernel")
    if ($DbDependsOnPipelineControl) {
        $dbDeps += "data-pipeline-control"
    }
    if ($DbDependsOnRawCapture) {
        $dbDeps += "raw-capture-client"
    }
    Write-Manifest -Root $Root -RelativePath "crates\db\Cargo.toml" -PackageName "db" -Dependencies $dbDeps
    if ($LegacyToolDependsOnPipelineControl) {
        Write-Manifest -Root $Root -RelativePath "crates\legacy-tool\Cargo.toml" -PackageName "legacy-tool" -Dependencies @("data-pipeline-control")
    }
    if ($PlatformCoreOwnedManifestPresent) {
        Write-Manifest -Root $Root -RelativePath "crates\domain\core\parcel\Cargo.toml" -PackageName "parcel-domain" -Dependencies @("shared-kernel")
        Write-Manifest -Root $Root -RelativePath "crates\data-clients\vworld\Cargo.toml" -PackageName "vworld-client" -Dependencies @("parcel-domain")
    }
    Write-File -Root $Root -RelativePath "services\api\src\building_reader.rs" -Content @'
use reqwest::Client;
'@
    Write-File -Root $Root -RelativePath "services\api\src\routes\buildings.rs" -Content @'
use crate::building_reader::BuildingRegisterRecord;
'@
    Write-File -Root $Root -RelativePath "services\api\src\startup.rs" -Content @'
use crate::routes::buildings::BuildingRegisterReader;
'@
    $policyContent = if ($CircuitBreakerCatalogPolicyPresent) {
        @'
pub struct Policy;
impl Policy {
    pub const fn vworld_default() -> Self { Self }
    pub const fn data_go_kr_default() -> Self { Self }
    pub const fn r2_default() -> Self { Self }
}
'@
    } else {
        @'
pub struct Policy;
impl Policy {
    pub const fn platform_core_default() -> Self { Self }
}
'@
    }
    Write-File -Root $Root -RelativePath "crates\circuit-breaker\src\policy.rs" -Content $policyContent
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

$cleanRoot = Join-Path $TempRoot "clean"
Write-Fixture -Root $cleanRoot
$clean = Invoke-Checker -Root $cleanRoot
Assert-Equals $clean.ExitCode 0 "Clean dependency boundary check exit code mismatch"
Assert-Contains $clean.Output "platform-core-dependency-boundary-ok"

$platformCoreOwnedManifestRoot = Join-Path $TempRoot "platform-core-owned-manifest"
Write-Fixture -Root $platformCoreOwnedManifestRoot -PlatformCoreOwnedManifestPresent $true
$platformCoreOwnedManifest = Invoke-Checker -Root $platformCoreOwnedManifestRoot
Assert-Equals $platformCoreOwnedManifest.ExitCode 1 "Platform Core-owned manifest violation exit code mismatch"
Assert-Contains $platformCoreOwnedManifest.Output "Platform Core-owned path must not contain a Gongzzang Cargo manifest"
Assert-Contains $platformCoreOwnedManifest.Output "crates/domain/core/parcel/Cargo.toml"

$productRoot = Join-Path $TempRoot "product-depends-on-parcel"
Write-Fixture -Root $productRoot -ProductDependsOnParcel $true
$product = Invoke-Checker -Root $productRoot
Assert-Equals $product.ExitCode 1 "Product domain dependency violation exit code mismatch"
Assert-Contains $product.Output "forbidden Platform Core transitional dependency"
Assert-Contains $product.Output "crates/domain/core/listing/Cargo.toml"

$apiParcelRoot = Join-Path $TempRoot "api-depends-on-parcel"
Write-Fixture -Root $apiParcelRoot -ApiDependsOnParcel $true
$apiParcel = Invoke-Checker -Root $apiParcelRoot
Assert-Equals $apiParcel.ExitCode 1 "API parcel dependency violation exit code mismatch"
Assert-Contains $apiParcel.Output "services/api/Cargo.toml must not depend on parcel-domain"

$apiVworldRoot = Join-Path $TempRoot "api-depends-on-vworld"
Write-Fixture -Root $apiVworldRoot -ApiDependsOnVworld $true
$apiVworld = Invoke-Checker -Root $apiVworldRoot
Assert-Equals $apiVworld.ExitCode 1 "API V-World dependency violation exit code mismatch"
Assert-Contains $apiVworld.Output "services/api/Cargo.toml must not depend on vworld-client"

$apiBuildingDomainRoot = Join-Path $TempRoot "api-depends-on-building-domain"
Write-Fixture -Root $apiBuildingDomainRoot -ApiDependsOnBuildingDomain $true
$apiBuildingDomain = Invoke-Checker -Root $apiBuildingDomainRoot
Assert-Equals $apiBuildingDomain.ExitCode 1 "API building-domain dependency violation exit code mismatch"
Assert-Contains $apiBuildingDomain.Output "services/api/Cargo.toml must not depend on building-domain"

$apiDataGoKrRoot = Join-Path $TempRoot "api-depends-on-data-go-kr"
Write-Fixture -Root $apiDataGoKrRoot -ApiDependsOnDataGoKr $true
$apiDataGoKr = Invoke-Checker -Root $apiDataGoKrRoot
Assert-Equals $apiDataGoKr.ExitCode 1 "API data-go-kr-client dependency violation exit code mismatch"
Assert-Contains $apiDataGoKr.Output "services/api/Cargo.toml must not depend on data-go-kr-client"

$parcelLookupParcelRoot = Join-Path $TempRoot "parcel-lookup-depends-on-parcel"
Write-Fixture -Root $parcelLookupParcelRoot -ParcelLookupDependsOnParcel $true
$parcelLookupParcel = Invoke-Checker -Root $parcelLookupParcelRoot
Assert-Equals $parcelLookupParcel.ExitCode 1 "Parcel lookup parcel-domain dependency violation exit code mismatch"
Assert-Contains $parcelLookupParcel.Output "crates/parcel-lookup/Cargo.toml must not depend on parcel-domain"

$parcelLookupVworldRoot = Join-Path $TempRoot "parcel-lookup-depends-on-vworld"
Write-Fixture -Root $parcelLookupVworldRoot -ParcelLookupDependsOnVworld $true
$parcelLookupVworld = Invoke-Checker -Root $parcelLookupVworldRoot
Assert-Equals $parcelLookupVworld.ExitCode 1 "Parcel lookup vworld-client dependency violation exit code mismatch"
Assert-Contains $parcelLookupVworld.Output "crates/parcel-lookup/Cargo.toml must not depend on vworld-client"

$parcelLookupReqwestRoot = Join-Path $TempRoot "parcel-lookup-depends-on-reqwest"
Write-Fixture -Root $parcelLookupReqwestRoot -ParcelLookupDependsOnReqwest $true
$parcelLookupReqwest = Invoke-Checker -Root $parcelLookupReqwestRoot
Assert-Equals $parcelLookupReqwest.ExitCode 1 "Parcel lookup reqwest dependency violation exit code mismatch"
Assert-Contains $parcelLookupReqwest.Output "crates/parcel-lookup/Cargo.toml must not depend on reqwest"

$parcelLookupReqwestSourceRoot = Join-Path $TempRoot "parcel-lookup-source-uses-reqwest"
Write-Fixture -Root $parcelLookupReqwestSourceRoot -ParcelLookupSourceUsesReqwest $true
$parcelLookupReqwestSource = Invoke-Checker -Root $parcelLookupReqwestSourceRoot
Assert-Equals $parcelLookupReqwestSource.ExitCode 1 "Parcel lookup reqwest source import violation exit code mismatch"
Assert-Contains $parcelLookupReqwestSource.Output "forbidden source import"
Assert-Contains $parcelLookupReqwestSource.Output "crates/parcel-lookup/src/lib.rs"

$catalogPolicyRoot = Join-Path $TempRoot "catalog-policy-in-circuit-breaker"
Write-Fixture -Root $catalogPolicyRoot -CircuitBreakerCatalogPolicyPresent $true
$catalogPolicy = Invoke-Checker -Root $catalogPolicyRoot
Assert-Equals $catalogPolicy.ExitCode 1 "Circuit breaker Catalog policy violation exit code mismatch"
Assert-Contains $catalogPolicy.Output "forbidden source import"
Assert-Contains $catalogPolicy.Output "crates/circuit-breaker/src/policy.rs"

$dbPipelineRoot = Join-Path $TempRoot "db-depends-on-pipeline-control"
Write-Fixture -Root $dbPipelineRoot -DbDependsOnPipelineControl $true
$dbPipeline = Invoke-Checker -Root $dbPipelineRoot
Assert-Equals $dbPipeline.ExitCode 1 "DB pipeline-control dependency violation exit code mismatch"
Assert-Contains $dbPipeline.Output "crates/db/Cargo.toml must not depend on data-pipeline-control"

$dbRawCaptureRoot = Join-Path $TempRoot "db-depends-on-raw-capture"
Write-Fixture -Root $dbRawCaptureRoot -DbDependsOnRawCapture $true
$dbRawCapture = Invoke-Checker -Root $dbRawCaptureRoot
Assert-Equals $dbRawCapture.ExitCode 1 "DB raw-capture dependency violation exit code mismatch"
Assert-Contains $dbRawCapture.Output "crates/db/Cargo.toml must not depend on raw-capture-client"

$routeCatalogTypeRoot = Join-Path $TempRoot "route-catalog-type"
Write-Fixture -Root $routeCatalogTypeRoot
Write-File -Root $routeCatalogTypeRoot -RelativePath "services\api\src\routes\buildings.rs" -Content @'
use building_domain::entity::Building;
'@
$routeCatalogType = Invoke-Checker -Root $routeCatalogTypeRoot
Assert-Equals $routeCatalogType.ExitCode 1 "Route catalog type dependency violation exit code mismatch"
Assert-Contains $routeCatalogType.Output "forbidden source import"
Assert-Contains $routeCatalogType.Output "services/api/src/routes/buildings.rs"

$apiHelperCatalogTypeRoot = Join-Path $TempRoot "api-helper-catalog-type"
Write-Fixture -Root $apiHelperCatalogTypeRoot
Write-File -Root $apiHelperCatalogTypeRoot -RelativePath "services\api\src\accidental_catalog.rs" -Content @'
use data_go_kr_client::DataGoKrClient;
'@
$apiHelperCatalogType = Invoke-Checker -Root $apiHelperCatalogTypeRoot
Assert-Equals $apiHelperCatalogType.ExitCode 1 "API helper catalog type dependency violation exit code mismatch"
Assert-Contains $apiHelperCatalogType.Output "forbidden source import"
Assert-Contains $apiHelperCatalogType.Output "services/api/src/accidental_catalog.rs"

$missingAllowanceRoot = Join-Path $TempRoot "missing-allowance"
Write-Fixture -Root $missingAllowanceRoot -LegacyToolDependsOnPipelineControl $true
$missingAllowance = Invoke-Checker -Root $missingAllowanceRoot
Assert-Equals $missingAllowance.ExitCode 1 "Missing transitional allowance exit code mismatch"
Assert-Contains $missingAllowance.Output "missing allowed_transitional_dependencies entry"
Assert-Contains $missingAllowance.Output "crates/legacy-tool/Cargo.toml -> data-pipeline-control"

$missingTraceRoot = Join-Path $TempRoot "missing-trace"
Write-Fixture -Root $missingTraceRoot -StaleAllowance $true -OmitAllowanceReason $true
$missingTrace = Invoke-Checker -Root $missingTraceRoot
Assert-Equals $missingTrace.ExitCode 1 "Missing transitional allowance traceability exit code mismatch"
Assert-Contains $missingTrace.Output "missing string 'reason'"

$newCatalogDepRoot = Join-Path $TempRoot "new-catalog-dep"
Write-Fixture -Root $newCatalogDepRoot -ApiDependsOnIndustrialComplex $true
$newCatalogDep = Invoke-Checker -Root $newCatalogDepRoot
Assert-Equals $newCatalogDep.ExitCode 1 "New unapproved catalog dependency exit code mismatch"
Assert-Contains $newCatalogDep.Output "services/api/Cargo.toml -> industrial-complex-domain"

$staleRoot = Join-Path $TempRoot "stale-allowance"
Write-Fixture -Root $staleRoot -StaleAllowance $true
$stale = Invoke-Checker -Root $staleRoot
Assert-Equals $stale.ExitCode 1 "Stale transitional allowance exit code mismatch"
Assert-Contains $stale.Output "stale allowed_transitional_dependencies entry"
Assert-Contains $stale.Output "services/api/Cargo.toml -> manufacturer-domain"

Remove-Item -LiteralPath $TempRoot -Recurse -Force
Write-Host "check-platform-core-dependency-boundary-tests-ok"
