[CmdletBinding()]
param(
    [string] $Root = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

$PolicyPath = "docs/architecture/platform-integration/lakehouse-registry-policy.v1.json"
$IndexPath = "docs/architecture/platform-integration/index.v1.json"
$BoundaryPath = "docs/architecture/platform-core-boundary.v1.json"
$AllowedCallMatrixPath = "docs/architecture/platform-integration/allowed-call-matrix.v1.json"
$ServiceAuthPolicyPath = "docs/architecture/platform-integration/service-auth-policy.v1.json"
$ThisGuardrail = "scripts/ci/check-lakehouse-registry-integration.ps1"
$RequiredContract = "lakehouse_registry_registration:gongzzang_to_platform_core"
$ExpectedBucket = "gongzzang-lakehouse-prod"
$LakehouseAllowedCallId = "gongzzang_pipeline_to_platform_core_lakehouse_registry"
$LakehouseServiceAuthPolicyId = "gongzzang_worker_to_platform_core_api"

function Resolve-RepoPath {
    param([string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Get-RepoRelativePath {
    param([string] $FullPath)

    $full = [System.IO.Path]::GetFullPath($FullPath)
    if ($full.StartsWith($resolvedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
        return ($full.Substring($resolvedRoot.Length).TrimStart([char[]] @("\", "/")) -replace "\\", "/")
    }
    return ($full -replace "\\", "/")
}

function Read-TextFile {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)

    return (Read-TextFile -RelativePath $RelativePath) | ConvertFrom-Json
}

function Get-JsonPropertyValue {
    param([object] $Object, [string] $Name)

    if ($null -eq $Object -or $null -eq $Object.PSObject.Properties[$Name]) {
        return $null
    }
    return $Object.PSObject.Properties[$Name].Value
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-ContainsString {
    param([string[]] $Values, [string] $Expected, [string] $Message)

    if (!($Values -contains $Expected)) {
        throw "$Message missing $Expected"
    }
}

function Assert-NotEmptyString {
    param([object] $Value, [string] $Message)

    if ([string]::IsNullOrWhiteSpace([string] $Value)) {
        throw "$Message must be set"
    }
}

function Assert-ValidPrefix {
    param([string] $Prefix, [string] $Message)

    if ([string]::IsNullOrWhiteSpace($Prefix)) {
        throw "$Message prefix must be set"
    }
    if (!$Prefix.EndsWith("/", [System.StringComparison]::Ordinal)) {
        throw "$Message prefix must end with /"
    }
    if ($Prefix.StartsWith("/", [System.StringComparison]::Ordinal) -or
        $Prefix.Contains("\") -or
        $Prefix.Contains("//") -or
        $Prefix.Contains("/../") -or
        $Prefix.StartsWith("../", [System.StringComparison]::Ordinal) -or
        $Prefix.Contains("/./") -or
        $Prefix.StartsWith("./", [System.StringComparison]::Ordinal)) {
        throw "$Message prefix is not normalized: $Prefix"
    }
}

function Assert-EnvAssignment {
    param([string] $Content, [string] $Name, [string] $Expected)

    $pattern = "(?m)^\s*$([regex]::Escape($Name))\s*=\s*$([regex]::Escape($Expected))\s*$"
    if (![regex]::IsMatch($Content, $pattern)) {
        throw "$Name must be $Expected"
    }
}

function Assert-PolicyShape {
    param([object] $Policy)

    Assert-Equals `
        -Actual $Policy.schema_version `
        -Expected "gongzzang.platform_integration.lakehouse_registry_policy.v1" `
        -Message "policy schema_version mismatch"
    Assert-Equals -Actual $Policy.repo_slug -Expected "gongzzang" -Message "policy repo_slug mismatch"
    Assert-Equals -Actual $Policy.owner_service -Expected "gongzzang" -Message "policy owner_service mismatch"

    $registry = Get-JsonPropertyValue -Object $Policy -Name "platform_core_registry"
    Assert-NotEmptyString -Value $registry.contract_ref -Message "platform_core_registry.contract_ref"
    Assert-Equals `
        -Actual $registry.allowed_call_id `
        -Expected $LakehouseAllowedCallId `
        -Message "platform_core_registry.allowed_call_id mismatch"
    foreach ($surface in @(
            "POST /internal/lakehouse/ingestion-runs",
            "POST /internal/lakehouse/artifacts",
            "POST /internal/lakehouse/quality-checks",
            "POST /internal/lakehouse/lineage",
            "POST /internal/lakehouse/promotions",
            "GET /internal/lakehouse/assets/{qualified_name}/active"
        )) {
        Assert-ContainsString `
            -Values @($registry.api_surfaces | ForEach-Object { [string] $_ }) `
            -Expected $surface `
            -Message "platform_core_registry.api_surfaces"
    }
}

function Assert-NamespacePolicy {
    param([object[]] $Namespaces)

    Assert-Equals -Actual @($Namespaces).Count -Expected 1 -Message "storage namespace count mismatch"
    $namespace = $Namespaces[0]
    Assert-Equals -Actual $namespace.id -Expected "gongzzang_r2_production" -Message "namespace id mismatch"
    Assert-Equals -Actual $namespace.provider -Expected "r2" -Message "namespace provider mismatch"
    Assert-Equals -Actual $namespace.environment -Expected "production" -Message "namespace environment mismatch"
    Assert-Equals -Actual $namespace.owner_service -Expected "gongzzang" -Message "namespace owner mismatch"
    Assert-Equals -Actual $namespace.bucket_name -Expected $ExpectedBucket -Message "namespace bucket mismatch"
    Assert-Equals -Actual $namespace.catalog_provider -Expected "r2_data_catalog" -Message "namespace catalog provider mismatch"
    Assert-Equals -Actual $namespace.status -Expected "active" -Message "namespace status mismatch"
    foreach ($prefix in @("bronze/", "silver/", "gold/", "media/", "__r2_data_catalog/")) {
        Assert-ContainsString `
            -Values @($namespace.allowed_root_prefixes | ForEach-Object { [string] $_ }) `
            -Expected $prefix `
            -Message "namespace allowed_root_prefixes"
    }
}

function Assert-AssetPolicy {
    param([object[]] $Assets)

    $requiredAssets = @{
        "gongzzang.bronze.onbid_sale" = @{
            Layer = "bronze"; Kind = "raw_object_set"; Prefix = "bronze/source=onbid-sale/"
        }
        "gongzzang.bronze.court_auction" = @{
            Layer = "bronze"; Kind = "raw_object_set"; Prefix = "bronze/source=court-auction/"
        }
        "gongzzang.gold.listing_marker_tiles" = @{
            Layer = "gold"; Kind = "pbf_tile_set"; Prefix = "gold/listing-marker-tiles/"
        }
        "gongzzang.gold.listing_marker_serving_index" = @{
            Layer = "gold"; Kind = "manifest"; Prefix = "gold/listing-marker-serving-index/"
        }
        "gongzzang.gold.listing_photo_media" = @{
            Layer = "gold"; Kind = "media_set"; Prefix = "media/listing-photo/"
        }
    }

    Assert-Equals -Actual @($Assets).Count -Expected $requiredAssets.Count -Message "governed asset count mismatch"
    $seen = New-Object System.Collections.Generic.HashSet[string]
    foreach ($asset in $Assets) {
        $qualifiedName = [string] $asset.qualified_name
        if (!$seen.Add($qualifiedName)) {
            throw "duplicate governed asset: $qualifiedName"
        }
        if (![regex]::IsMatch($qualifiedName, "^gongzzang\.(bronze|silver|gold)\.[a-z0-9_]+$")) {
            throw "invalid governed asset qualified_name: $qualifiedName"
        }
        if (!$requiredAssets.ContainsKey($qualifiedName)) {
            throw "unexpected governed asset: $qualifiedName"
        }
        $expected = $requiredAssets[$qualifiedName]
        Assert-Equals -Actual $asset.layer -Expected $expected.Layer -Message "asset layer mismatch for $qualifiedName"
        Assert-Equals -Actual $asset.asset_kind -Expected $expected.Kind -Message "asset kind mismatch for $qualifiedName"
        Assert-Equals -Actual $asset.namespace_id -Expected "gongzzang_r2_production" -Message "asset namespace mismatch for $qualifiedName"
        Assert-Equals -Actual $asset.registry_required -Expected $true -Message "asset registry_required mismatch for $qualifiedName"
        $prefixes = @($asset.allowed_object_prefixes | ForEach-Object { [string] $_ })
        Assert-ContainsString -Values $prefixes -Expected $expected.Prefix -Message "asset allowed_object_prefixes for $qualifiedName"
        foreach ($prefix in $prefixes) {
            Assert-ValidPrefix -Prefix $prefix -Message "asset $qualifiedName"
        }
    }
}

function Assert-IndexAndBoundary {
    param([object] $Index, [object] $Boundary)

    $component = @($Index.components | Where-Object {
            [string] $_.id -eq "platform_integration.lakehouse_registry"
        })
    Assert-Equals -Actual $component.Count -Expected 1 -Message "index lakehouse registry component count mismatch"
    Assert-Equals `
        -Actual $component[0].path `
        -Expected "docs/architecture/platform-integration/lakehouse-registry-policy.v1.json" `
        -Message "index lakehouse registry component path mismatch"
    Assert-Equals `
        -Actual $component[0].schema_version `
        -Expected "gongzzang.platform_integration.lakehouse_registry_policy.v1" `
        -Message "index lakehouse registry schema mismatch"
    Assert-ContainsString `
        -Values @($Index.required_guardrails | ForEach-Object { [string] $_ }) `
        -Expected $ThisGuardrail `
        -Message "index required_guardrails"

    $contracts = @($Boundary.allowed_integration_contracts | ForEach-Object {
            "$([string] $_.kind):$([string] $_.direction)"
        })
    Assert-ContainsString `
        -Values $contracts `
        -Expected $RequiredContract `
        -Message "missing allowed integration contract"
}

function Assert-AllowedCallMatrix {
    param([object] $Policy, [object] $AllowedCallMatrix)

    Assert-Equals `
        -Actual $AllowedCallMatrix.schema_version `
        -Expected "gongzzang.platform_integration.allowed_call_matrix.v1" `
        -Message "allowed call matrix schema_version mismatch"
    $allowedCallId = [string] $Policy.platform_core_registry.allowed_call_id
    $calls = @($AllowedCallMatrix.allowed_calls | Where-Object { [string] $_.id -eq $allowedCallId })
    Assert-Equals -Actual $calls.Count -Expected 1 -Message "allowed call matrix lakehouse registry call count mismatch"
    $call = $calls[0]
    if (!(@("planned", "active") -contains ([string] $call.status))) {
        throw "lakehouse registry allowed call must be planned or active"
    }
    Assert-Equals -Actual $call.source_repo -Expected "gongzzang" -Message "lakehouse registry source_repo mismatch"
    Assert-Equals -Actual $call.source_service -Expected "gongzzang-worker" -Message "lakehouse registry source_service mismatch"
    Assert-Equals -Actual $call.target_repo -Expected "platform-core" -Message "lakehouse registry target_repo mismatch"
    Assert-Equals -Actual $call.service_auth_policy_id -Expected $LakehouseServiceAuthPolicyId -Message "lakehouse registry service_auth_policy_id mismatch"
    foreach ($surface in @($Policy.platform_core_registry.api_surfaces | ForEach-Object { [string] $_ })) {
        Assert-ContainsString `
            -Values @($call.allowed_surfaces | ForEach-Object { [string] $_ }) `
            -Expected $surface `
            -Message "allowed call matrix lakehouse allowed_surfaces"
    }
    foreach ($control in @("registry_contract_defined", "object_checksum_verified", "no_direct_database")) {
        Assert-ContainsString `
            -Values @($call.current_required_controls | ForEach-Object { [string] $_ }) `
            -Expected $control `
            -Message "allowed call matrix lakehouse current_required_controls"
    }
}

function Assert-ServiceAuthPolicy {
    param([object] $Policy, [object] $ServiceAuthPolicy)

    Assert-Equals `
        -Actual $ServiceAuthPolicy.schema_version `
        -Expected "gongzzang.platform_integration.service_auth_policy.v1" `
        -Message "service auth policy schema_version mismatch"
    $identity = @($ServiceAuthPolicy.outbound_identities | Where-Object {
            [string] $_.id -eq $LakehouseServiceAuthPolicyId
        })
    Assert-Equals -Actual $identity.Count -Expected 1 -Message "lakehouse registry service auth identity count mismatch"
    $identity = $identity[0]
    Assert-Equals -Actual $identity.source_service -Expected "gongzzang-worker" -Message "lakehouse registry auth source_service mismatch"
    Assert-Equals -Actual $identity.target_service -Expected "platform-core-api" -Message "lakehouse registry auth target_service mismatch"
    Assert-Equals -Actual $identity.token_metadata.required_scope -Expected "lakehouse:write" -Message "lakehouse registry auth required_scope mismatch"
    Assert-Equals -Actual $identity.authorization_policy.default_decision -Expected "deny" -Message "lakehouse registry auth default decision mismatch"
    Assert-Equals `
        -Actual $identity.authorization_policy.allowed_call_id `
        -Expected ([string] $Policy.platform_core_registry.allowed_call_id) `
        -Message "lakehouse registry auth allowed_call_id mismatch"
    foreach ($runtimeFile in @(
            "services/api/src/platform_core_auth.rs",
            "services/api/src/platform_core_lakehouse_registry.rs"
        )) {
        Assert-ContainsString `
            -Values @($identity.runtime_files | ForEach-Object { [string] $_ }) `
            -Expected $runtimeFile `
            -Message "lakehouse registry auth runtime_files"
    }
}

function Assert-EnvContract {
    param([object] $Policy)

    $envExample = Read-TextFile -RelativePath ".env.example"
    $envContract = Get-JsonPropertyValue -Object $Policy -Name "required_env_contract"
    Assert-EnvAssignment `
        -Content $envExample `
        -Name "GONGZZANG_LAKEHOUSE_R2_BUCKET" `
        -Expected ([string] $envContract.GONGZZANG_LAKEHOUSE_R2_BUCKET)
    Assert-EnvAssignment `
        -Content $envExample `
        -Name "LISTING_PHOTO_R2_BUCKET" `
        -Expected ([string] $envContract.LISTING_PHOTO_R2_BUCKET)
    foreach ($required in @(
            "GONGZZANG_LAKEHOUSE_R2_ACCOUNT_ID=",
            "GONGZZANG_LAKEHOUSE_R2_ACCESS_KEY=",
            "GONGZZANG_LAKEHOUSE_R2_SECRET_KEY=",
            "LISTING_PHOTO_R2_ACCOUNT_ID=",
            "LISTING_PHOTO_R2_ACCESS_KEY=",
            "LISTING_PHOTO_R2_SECRET_KEY="
        )) {
        if (!$envExample.Contains($required)) {
            throw ".env.example missing $required"
        }
    }
}

function Get-ScannedFiles {
    $roots = @("apps", "services", "crates", "scripts", ".github", "migrations", "infrastructure", "infra")
    foreach ($root in $roots) {
        $path = Resolve-RepoPath -RelativePath $root
        if (!(Test-Path -LiteralPath $path -PathType Container)) {
            continue
        }
        Get-ChildItem -LiteralPath $path -Recurse -File | Where-Object {
            $relative = Get-RepoRelativePath -FullPath $_.FullName
            !$relative.StartsWith("target/", [System.StringComparison]::Ordinal) -and
            $relative.IndexOf("/node_modules/", [System.StringComparison]::Ordinal) -lt 0 -and
            $relative.IndexOf("/.next/", [System.StringComparison]::Ordinal) -lt 0 -and
            $relative.IndexOf(".tests.", [System.StringComparison]::Ordinal) -lt 0
        }
    }
}

function Assert-NoUnmanagedSharedRootWrites {
    $forbiddenPatterns = @(
        "s3://gongzzang/(bronze|silver|gold)/",
        "gongzzang/(bronze|silver|gold)/"
    )
    foreach ($file in Get-ScannedFiles) {
        $relative = Get-RepoRelativePath -FullPath $file.FullName
        $content = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        if ($null -eq $content) {
            $content = ""
        }
        foreach ($pattern in $forbiddenPatterns) {
            $match = [regex]::Match($content, $pattern)
            if ($match.Success) {
                throw "unmanaged shared-root lakehouse write in ${relative}: $($match.Value)"
            }
        }
    }
}

function Assert-ListingPhotoNamespace {
    foreach ($file in Get-ScannedFiles) {
        $relative = Get-RepoRelativePath -FullPath $file.FullName
        if (!$relative.StartsWith("services/", [System.StringComparison]::Ordinal) -and
            !$relative.StartsWith("crates/", [System.StringComparison]::Ordinal) -and
            !$relative.StartsWith("apps/", [System.StringComparison]::Ordinal)) {
            continue
        }
        $content = Get-Content -LiteralPath $file.FullName -Raw -Encoding UTF8
        if ($null -eq $content) {
            $content = ""
        }
        $bad = [regex]::Match($content, "(?<!media/)listing-photo/listings/")
        if ($bad.Success) {
            throw "listing photo object keys must stay under media/listing-photo/ in $relative"
        }
    }
}

function Assert-DocsLinked {
    $adr = Read-TextFile -RelativePath "docs/adr/0039-service-owned-lakehouse-registry-integration.md"
    $spec = Read-TextFile -RelativePath "docs/superpowers/specs/2026-06-05-gongzzang-service-owned-lakehouse-integration-design.md"
    $benchmark = "2026-06-07-enterprise-lakehouse-media-registry-benchmark.md"
    if (!$adr.Contains($benchmark)) {
        throw "ADR 0039 must reference enterprise lakehouse benchmark"
    }
    if (!$spec.Contains($benchmark)) {
        throw "service-owned lakehouse spec must reference enterprise lakehouse benchmark"
    }
}

$policy = Read-JsonFile -RelativePath $PolicyPath
$index = Read-JsonFile -RelativePath $IndexPath
$boundary = Read-JsonFile -RelativePath $BoundaryPath
$allowedCallMatrix = Read-JsonFile -RelativePath $AllowedCallMatrixPath
$serviceAuthPolicy = Read-JsonFile -RelativePath $ServiceAuthPolicyPath

Assert-PolicyShape -Policy $policy
Assert-NamespacePolicy -Namespaces @($policy.storage_namespaces)
Assert-AssetPolicy -Assets @($policy.governed_assets)
Assert-IndexAndBoundary -Index $index -Boundary $boundary
Assert-AllowedCallMatrix -Policy $policy -AllowedCallMatrix $allowedCallMatrix
Assert-ServiceAuthPolicy -Policy $policy -ServiceAuthPolicy $serviceAuthPolicy
Assert-EnvContract -Policy $policy
Assert-NoUnmanagedSharedRootWrites
Assert-ListingPhotoNamespace
Assert-DocsLinked

$mediaSets = @($policy.governed_assets | Where-Object { [string] $_.asset_kind -eq "media_set" })
Write-Host "lakehouse-registry-integration-ok namespaces=$(@($policy.storage_namespaces).Count) assets=$(@($policy.governed_assets).Count) media_sets=$($mediaSets.Count)"
