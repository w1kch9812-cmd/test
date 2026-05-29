[CmdletBinding()]
param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$BoundaryRelativePath = "docs/architecture/platform-core-boundary.v1.json"
$PlatformCoreDependencyPaths = @{
    "parcel-domain" = "crates/domain/core/parcel"
    "building-domain" = "crates/domain/core/building"
    "industrial-complex-domain" = "crates/domain/core/industrial-complex"
    "manufacturer-domain" = "crates/domain/core/manufacturer"
    "vworld-client" = "crates/data-clients/vworld"
    "data-go-kr-client" = "crates/data-clients/data-go-kr"
    "raw-capture-client" = "crates/data-clients/raw-capture"
    "data-pipeline-control" = "crates/data-pipeline-control"
    "r2-public-data-client" = "crates/data-clients/r2-public-data"
}
$ForbiddenHighLevelDependencies = @{
    "services/api/Cargo.toml" = @(
        "building-domain",
        "data-go-kr-client",
        "parcel-domain",
        "vworld-client"
    )
    "crates/parcel-lookup/Cargo.toml" = @(
        "parcel-domain",
        "vworld-client"
    )
    "crates/db/Cargo.toml" = @(
        "data-pipeline-control",
        "raw-capture-client"
    )
}
$ForbiddenLayerDependencies = @{
    "crates/parcel-lookup/Cargo.toml" = @(
        "reqwest"
    )
}
$ForbiddenSourceImports = @(
    @{
        Path = "services/api/src"
        Tokens = @(
            "building_domain::",
            "data_go_kr_client::",
            "parcel_domain::",
            "vworld_client::"
        )
    },
    @{
        Path = "crates/parcel-lookup/src"
        Tokens = @(
            "reqwest::"
        )
    },
    @{
        Path = "crates/circuit-breaker/src"
        Tokens = @(
            "vworld_default",
            "data_go_kr_default",
            "r2_default"
        )
    }
)

function Resolve-RepoPath {
    param([string] $RootPath, [string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $RootPath $RelativePath))
}

function ConvertTo-RepoRelativePath {
    param([string] $RootPath, [string] $FullPath)

    $rootPrefix = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    $resolved = [System.IO.Path]::GetFullPath($FullPath)
    if ($resolved.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return ($resolved.Substring($rootPrefix.Length) -replace "\\", "/")
    }
    return ($resolved -replace "\\", "/")
}

function Normalize-RelativePath {
    param([string] $Path)

    $normalized = $Path -replace "\\", "/"
    if ($normalized.StartsWith("./", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(2)
    }
    while ($normalized.StartsWith("/", [System.StringComparison]::Ordinal)) {
        $normalized = $normalized.Substring(1)
    }
    return $normalized
}

function Get-PropertyValue {
    param([object] $Object, [string] $Name)

    if ($null -eq $Object -or $null -eq $Object.PSObject.Properties[$Name]) {
        return $null
    }
    return $Object.PSObject.Properties[$Name].Value
}

function Get-RequiredArray {
    param([object] $Object, [string] $Name)

    if ($null -eq $Object -or $null -eq $Object.PSObject.Properties[$Name]) {
        throw "platform-core-dependency-boundary: missing array '$Name'"
    }
    $value = $Object.PSObject.Properties[$Name].Value
    if ($null -eq $value) {
        return @()
    }
    return @($value)
}

function Get-OptionalArray {
    param([object] $Object, [string] $Name)

    $value = Get-PropertyValue -Object $Object -Name $Name
    if ($null -eq $value) {
        return @()
    }
    return @($value)
}

function Get-RequiredString {
    param([object] $Object, [string] $Name)

    $value = [string] (Get-PropertyValue -Object $Object -Name $Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "platform-core-dependency-boundary: missing string '$Name'"
    }
    return $value
}

function Get-CargoDependencies {
    param([string] $Path)

    $dependencies = New-Object System.Collections.Generic.HashSet[string]
    $inDependencySection = $false

    foreach ($line in Get-Content -LiteralPath $Path) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("#")) {
            continue
        }

        if ($trimmed -match '^\[([^\]]+)\]$') {
            $section = $Matches[1]
            $inDependencySection = $section -eq "dependencies" `
                -or $section -eq "dev-dependencies" `
                -or $section -eq "build-dependencies" `
                -or $section.EndsWith(".dependencies", [System.StringComparison]::Ordinal) `
                -or $section.EndsWith(".dev-dependencies", [System.StringComparison]::Ordinal) `
                -or $section.EndsWith(".build-dependencies", [System.StringComparison]::Ordinal)
            continue
        }

        if (!$inDependencySection) {
            continue
        }
        if ($line -match '^\s*([A-Za-z0-9_-]+)\s*=') {
            [void] $dependencies.Add($Matches[1])
        }
    }

    return @($dependencies | Sort-Object)
}

function Test-PathInsideBoundaryPath {
    param([string] $RelativePath, [string] $BoundaryPath)

    $relative = Normalize-RelativePath -Path $RelativePath
    $boundary = Normalize-RelativePath -Path $BoundaryPath
    return $relative -eq $boundary `
        -or $relative.StartsWith("$boundary/", [System.StringComparison]::OrdinalIgnoreCase)
}

function Find-OwnershipEntry {
    param([object[]] $Entries, [string] $RelativePath)

    $matches = @($Entries | Where-Object {
            Test-PathInsideBoundaryPath -RelativePath $RelativePath -BoundaryPath ([string] (Get-PropertyValue -Object $_ -Name "path"))
        } | Sort-Object { ([string] (Get-PropertyValue -Object $_ -Name "path")).Length } -Descending)
    if ($matches.Count -eq 0) {
        return $null
    }
    return $matches[0]
}

function Get-AllowanceKey {
    param([string] $Manifest, [string] $Dependency)

    return "$(Normalize-RelativePath -Path $Manifest)->$Dependency"
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$boundaryPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $BoundaryRelativePath
if (!(Test-Path -LiteralPath $boundaryPath -PathType Leaf)) {
    throw "platform-core-dependency-boundary: missing boundary SSOT: $BoundaryRelativePath"
}

$boundary = Get-Content -LiteralPath $boundaryPath -Raw | ConvertFrom-Json
$pathOwnership = Get-RequiredArray -Object $boundary -Name "path_ownership"
$allowedDependencies = Get-RequiredArray -Object $boundary -Name "allowed_transitional_dependencies"
$allowedSourceImports = Get-OptionalArray -Object $boundary -Name "allowed_transitional_source_imports"

$knownForbiddenSourceTokens = @(
    $ForbiddenSourceImports | ForEach-Object { @($_.Tokens) } | Sort-Object -Unique
)

$allowanceKeys = New-Object System.Collections.Generic.HashSet[string]
$allowancePairs = @()
foreach ($allowance in $allowedDependencies) {
    $manifest = Normalize-RelativePath -Path (Get-RequiredString -Object $allowance -Name "manifest")
    $dependency = Get-RequiredString -Object $allowance -Name "dependency"
    $owner = Get-RequiredString -Object $allowance -Name "owner"
    $untilPhase = Get-RequiredString -Object $allowance -Name "until_phase"
    $reason = Get-RequiredString -Object $allowance -Name "reason"
    $exitCriteria = Get-RequiredString -Object $allowance -Name "exit_criteria"

    if ($owner -ne "platform-core") {
        throw "platform-core-dependency-boundary: allowed_transitional_dependencies owner must be platform-core: $manifest -> $dependency"
    }
    if (!$PlatformCoreDependencyPaths.ContainsKey($dependency)) {
        throw "platform-core-dependency-boundary: unknown Platform Core transitional dependency in allowance: $manifest -> $dependency"
    }
    if (!$untilPhase.StartsWith("m", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "platform-core-dependency-boundary: allowance until_phase must be a migration phase: $manifest -> $dependency"
    }
    if ($reason.Length -lt 16) {
        throw "platform-core-dependency-boundary: allowance reason is too weak: $manifest -> $dependency"
    }
    if ($exitCriteria.Length -lt 16) {
        throw "platform-core-dependency-boundary: allowance exit_criteria is too weak: $manifest -> $dependency"
    }

    $key = Get-AllowanceKey -Manifest $manifest -Dependency $dependency
    if (!$allowanceKeys.Add($key)) {
        throw "platform-core-dependency-boundary: duplicate allowed_transitional_dependencies entry: $manifest -> $dependency"
    }

    $allowancePairs += [pscustomobject]@{
        Manifest = $manifest
        Dependency = $dependency
        Key = $key
    }
}

$sourceImportAllowanceKeys = New-Object System.Collections.Generic.HashSet[string]
$sourceImportAllowances = @()
foreach ($allowance in $allowedSourceImports) {
    $path = Normalize-RelativePath -Path (Get-RequiredString -Object $allowance -Name "path")
    $owner = Get-RequiredString -Object $allowance -Name "owner"
    $untilPhase = Get-RequiredString -Object $allowance -Name "until_phase"
    $reason = Get-RequiredString -Object $allowance -Name "reason"
    $exitCriteria = Get-RequiredString -Object $allowance -Name "exit_criteria"
    $tokens = @(Get-RequiredArray -Object $allowance -Name "tokens" | ForEach-Object { [string] $_ })

    if ($owner -ne "platform-core") {
        throw "platform-core-dependency-boundary: allowed_transitional_source_imports owner must be platform-core: $path"
    }
    if (!$untilPhase.StartsWith("m", [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "platform-core-dependency-boundary: source import allowance until_phase must be a migration phase: $path"
    }
    if ($reason.Length -lt 16) {
        throw "platform-core-dependency-boundary: source import allowance reason is too weak: $path"
    }
    if ($exitCriteria.Length -lt 16) {
        throw "platform-core-dependency-boundary: source import allowance exit_criteria is too weak: $path"
    }
    if ($tokens.Count -eq 0) {
        throw "platform-core-dependency-boundary: allowed_transitional_source_imports tokens must not be empty: $path"
    }

    foreach ($token in $tokens) {
        if (!($knownForbiddenSourceTokens -contains $token)) {
            throw "platform-core-dependency-boundary: unknown source import token in allowance: $path contains $token"
        }
        $key = "$path::$token"
        if (!$sourceImportAllowanceKeys.Add($key)) {
            throw "platform-core-dependency-boundary: duplicate allowed_transitional_source_imports entry: $path contains $token"
        }
        $sourceImportAllowances += [pscustomobject]@{
            Path = $path
            Token = $token
            Key = $key
        }
    }
}

$manifestDependencyMap = [System.Collections.Generic.Dictionary[string, object]]::new(
    [System.StringComparer]::OrdinalIgnoreCase
)
$cargoManifests = @(
    Get-ChildItem -LiteralPath $resolvedRoot -Recurse -Filter "Cargo.toml" |
        Where-Object { !$_.PSIsContainer }
)
foreach ($manifestFile in $cargoManifests) {
    $relativeManifest = ConvertTo-RepoRelativePath -RootPath $resolvedRoot -FullPath $manifestFile.FullName
    if ($relativeManifest -eq "target/Cargo.toml" -or $relativeManifest.StartsWith("target/", [System.StringComparison]::OrdinalIgnoreCase) -or $relativeManifest.Contains("/target/")) {
        continue
    }
    $manifestDependencyMap[$relativeManifest] = @(Get-CargoDependencies -Path $manifestFile.FullName)
}

$violations = @()
foreach ($manifest in $manifestDependencyMap.Keys) {
    $ownership = Find-OwnershipEntry -Entries $pathOwnership -RelativePath $manifest
    $owner = if ($null -eq $ownership) { "" } else { [string] (Get-PropertyValue -Object $ownership -Name "owner") }
    $classification = if ($null -eq $ownership) { "" } else { [string] (Get-PropertyValue -Object $ownership -Name "classification") }
    $isPlatformCoreExtractedManifest = $owner -eq "platform-core" -and (
        $classification -eq "transitional_catalog_asset" -or
        $classification -eq "transitional_catalog_etl_asset" -or
        $classification -eq "extracted_catalog_asset" -or
        $classification -eq "extracted_catalog_etl_asset"
    )
    if ($isPlatformCoreExtractedManifest) {
        $violations += "Platform Core-owned path must not contain a Gongzzang Cargo manifest after extraction: $manifest"
        continue
    }

    foreach ($dependency in @($manifestDependencyMap[$manifest])) {
        $normalizedManifest = Normalize-RelativePath -Path ([string] $manifest)
        if ($ForbiddenLayerDependencies.ContainsKey($normalizedManifest) -and @($ForbiddenLayerDependencies[$normalizedManifest]) -contains $dependency) {
            $violations += "$normalizedManifest must not depend on $dependency; keep Platform Core HTTP adapters in services/api"
            continue
        }
        if (!$PlatformCoreDependencyPaths.ContainsKey($dependency)) {
            continue
        }
        if ($ForbiddenHighLevelDependencies.ContainsKey($normalizedManifest) -and @($ForbiddenHighLevelDependencies[$normalizedManifest]) -contains $dependency) {
            $violations += "$normalizedManifest must not depend on $dependency; use Platform Core published contracts"
            continue
        }
        if ($owner -eq "gongzzang" -and $classification -eq "product_domain") {
            $violations += "forbidden Platform Core transitional dependency: $manifest -> $dependency"
            continue
        }

        $key = Get-AllowanceKey -Manifest $manifest -Dependency $dependency
        if (!$allowanceKeys.Contains($key)) {
            $violations += "missing allowed_transitional_dependencies entry: $manifest -> $dependency"
        }
    }
}

foreach ($allowance in $allowancePairs) {
    $matchingManifest = @($manifestDependencyMap.Keys | Where-Object {
            (Normalize-RelativePath -Path ([string] $_)) -eq (Normalize-RelativePath -Path ([string] $allowance.Manifest))
        })
    if ($matchingManifest.Count -eq 0) {
        $violations += "stale allowed_transitional_dependencies entry: missing manifest $($allowance.Manifest) -> $($allowance.Dependency)"
        continue
    }
    if (!(@($manifestDependencyMap[$matchingManifest[0]]) -contains $allowance.Dependency)) {
        $violations += "stale allowed_transitional_dependencies entry: $($allowance.Manifest) -> $($allowance.Dependency)"
    }
}

foreach ($rule in $ForbiddenSourceImports) {
    $relativePath = Normalize-RelativePath -Path ([string] $rule.Path)
    $path = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $relativePath
    if (!(Test-Path -LiteralPath $path)) {
        continue
    }
    $files = if (Test-Path -LiteralPath $path -PathType Leaf) {
        @(Get-Item -LiteralPath $path)
    } else {
        @(Get-ChildItem -LiteralPath $path -Recurse -File -Include "*.rs")
    }
    foreach ($file in $files) {
        $content = Get-Content -LiteralPath $file.FullName -Raw
        $fileRelativePath = ConvertTo-RepoRelativePath -RootPath $resolvedRoot -FullPath $file.FullName
        $normalizedFileRelativePath = Normalize-RelativePath -Path $fileRelativePath
        foreach ($token in @($rule.Tokens)) {
            if ($content.Contains($token)) {
                $sourceImportKey = "$normalizedFileRelativePath::$token"
                if (!$sourceImportAllowanceKeys.Contains($sourceImportKey)) {
                    $violations += "forbidden source import: $fileRelativePath contains $token; missing allowed_transitional_source_imports entry"
                }
            }
        }
    }
}

foreach ($allowance in $sourceImportAllowances) {
    $path = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $allowance.Path
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        $violations += "stale allowed_transitional_source_imports entry: missing file $($allowance.Path) contains $($allowance.Token)"
        continue
    }
    $content = Get-Content -LiteralPath $path -Raw
    if (!$content.Contains($allowance.Token)) {
        $violations += "stale allowed_transitional_source_imports entry: $($allowance.Path) contains $($allowance.Token)"
    }
}

if ($violations.Count -gt 0) {
    foreach ($violation in $violations) {
        [Console]::Error.WriteLine("platform-core-dependency-boundary: {0}", $violation)
    }
    exit 1
}

Write-Host "platform-core-dependency-boundary-ok manifests=$($manifestDependencyMap.Count) allowances=$($allowancePairs.Count) source_allowances=$($sourceImportAllowances.Count)"
