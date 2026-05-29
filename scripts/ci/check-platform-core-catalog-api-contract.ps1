[CmdletBinding()]
param(
    [string] $Root = "",
    [string] $PlatformCoreRoot = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $scriptRoot = $PSScriptRoot
    if ([string]::IsNullOrWhiteSpace($scriptRoot)) {
        $scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
    }
    $Root = Join-Path $scriptRoot "..\.."
}

$PinRelativePath = "docs/architecture/platform-core-catalog-api-contract.v1.pin.json"

function Resolve-RepoPath {
    param([string] $RootPath, [string] $RelativePath)

    return [System.IO.Path]::GetFullPath((Join-Path $RootPath $RelativePath))
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

    $value = Get-PropertyValue -Object $Object -Name $Name
    if ($null -eq $value) {
        throw "platform-core-catalog-api-contract: missing array '$Name'"
    }
    return @($value)
}

function Get-RequiredString {
    param([object] $Object, [string] $Name)

    $value = [string] (Get-PropertyValue -Object $Object -Name $Name)
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "platform-core-catalog-api-contract: missing string '$Name'"
    }
    return $value
}

function Read-JsonFile {
    param([string] $Path, [string] $Label)

    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "platform-core-catalog-api-contract: missing $Label"
    }
    return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
}

function Normalize-StringArray {
    param([object[]] $Values)

    return @($Values | ForEach-Object { [string] $_ } | Sort-Object -Unique)
}

function Normalize-RelativePath {
    param([string] $Path)

    return $Path.Replace("\", "/").TrimStart("./")
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "platform-core-catalog-api-contract: $Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-Contains {
    param([string[]] $Values, [string] $Expected, [string] $Message)

    if (!(@($Values) -contains $Expected)) {
        throw "platform-core-catalog-api-contract: $Message '$Expected'"
    }
}

function Get-PinnedContract {
    param([object] $Pin)

    $schema = Get-RequiredString -Object $Pin -Name "schema_version"
    if ($schema -ne "gongzzang.platform_core_catalog_api_contract_pin.v1") {
        throw "platform-core-catalog-api-contract: pin schema_version mismatch"
    }

    $endpoints = @(Get-RequiredArray -Object $Pin -Name "endpoints" | ForEach-Object {
        [pscustomobject]@{
            operation_id = Get-RequiredString -Object $_ -Name "operation_id"
            client_module = Get-RequiredString -Object $_ -Name "client_module"
            method = Get-RequiredString -Object $_ -Name "method"
            path_template = Get-RequiredString -Object $_ -Name "path_template"
            path_pattern_literal = Get-RequiredString -Object $_ -Name "path_pattern_literal"
            success_shape = Get-RequiredString -Object $_ -Name "success_shape"
            not_found_behavior = [string] (Get-PropertyValue -Object $_ -Name "not_found_behavior")
            required_response_fields = Normalize-StringArray -Values (Get-RequiredArray -Object $_ -Name "required_response_fields")
        }
    } | Sort-Object -Property operation_id)

    [pscustomobject]@{
        source_repo = Get-RequiredString -Object $Pin -Name "source_repo"
        source_path = Get-RequiredString -Object $Pin -Name "source_path"
        source_schema_version = Get-RequiredString -Object $Pin -Name "source_schema_version"
        consumer_slug = Get-RequiredString -Object $Pin -Name "consumer_slug"
        base_env = Get-RequiredString -Object $Pin -Name "base_env"
        allowed_reference_modules = Normalize-StringArray -Values (Get-RequiredArray -Object $Pin -Name "allowed_reference_modules")
        endpoints = $endpoints
    }
}

function Get-EndpointByOperation {
    param([object] $Contract, [string] $OperationId)

    $matches = @($Contract.endpoints | Where-Object { $_.operation_id -eq $OperationId })
    if ($matches.Count -ne 1) {
        throw "platform-core-catalog-api-contract: expected one endpoint for operation '$OperationId'"
    }
    return $matches[0]
}

function Assert-PinnedContractShape {
    param([object] $Contract)

    Assert-Equals $Contract.source_repo "platform-core" "pin source_repo mismatch"
    Assert-Equals $Contract.source_path "docs/openapi/catalog.v1.yaml" "pin source_path mismatch"
    Assert-Equals $Contract.consumer_slug "gongzzang" "pin consumer_slug mismatch"
    Assert-Equals $Contract.base_env "PLATFORM_CORE_API_BASE_URL" "pin base_env mismatch"

    Assert-Contains `
        -Values $Contract.allowed_reference_modules `
        -Expected "services/api/src/startup.rs" `
        -Message "missing allowed reference module"

    $operations = Normalize-StringArray -Values (@($Contract.endpoints | ForEach-Object { $_.operation_id }))
    Assert-Equals @($operations).Count 2 "pin endpoint count mismatch"
    Assert-Contains -Values $operations -Expected "getParcelByPnu" -Message "missing pinned operation"
    Assert-Contains -Values $operations -Expected "listParcelBuildingsByPnu" -Message "missing pinned operation"

    $parcel = Get-EndpointByOperation -Contract $Contract -OperationId "getParcelByPnu"
    Assert-Equals $parcel.method "GET" "parcel method mismatch"
    Assert-Equals $parcel.path_template "/catalog/v1/parcels/by-pnu/{pnu}" "parcel path_template mismatch"
    Assert-Equals $parcel.success_shape "object" "parcel success_shape mismatch"
    Assert-Equals $parcel.not_found_behavior "none" "parcel not_found_behavior mismatch"

    $buildings = Get-EndpointByOperation -Contract $Contract -OperationId "listParcelBuildingsByPnu"
    Assert-Equals $buildings.method "GET" "building method mismatch"
    Assert-Equals $buildings.path_template "/catalog/v1/parcels/by-pnu/{pnu}/buildings" "building path_template mismatch"
    Assert-Equals $buildings.success_shape "array" "building success_shape mismatch"
}

function Get-RepoRelativePath {
    param([string] $RootPath, [string] $FullPath)

    $rootPrefix = [System.IO.Path]::GetFullPath($RootPath).TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
    $full = [System.IO.Path]::GetFullPath($FullPath)
    if ($full.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        return Normalize-RelativePath -Path $full.Substring($rootPrefix.Length)
    }
    return Normalize-RelativePath -Path $full
}

function Assert-NoUnpinnedLocalCatalogApiConsumers {
    param([string] $RootPath, [object] $Contract)

    $allowed = New-Object System.Collections.Generic.HashSet[string]
    foreach ($endpoint in @($Contract.endpoints)) {
        [void] $allowed.Add((Normalize-RelativePath -Path $endpoint.client_module))
    }
    foreach ($referenceModule in @($Contract.allowed_reference_modules)) {
        [void] $allowed.Add((Normalize-RelativePath -Path $referenceModule))
    }

    $roots = @("apps", "services", "crates", "packages")
    $extensions = @(".rs", ".ts", ".tsx", ".js", ".jsx", ".md")
    foreach ($root in $roots) {
        $scanRoot = Resolve-RepoPath -RootPath $RootPath -RelativePath $root
        if (!(Test-Path -LiteralPath $scanRoot)) {
            continue
        }

        foreach ($file in Get-ChildItem -LiteralPath $scanRoot -Recurse -File) {
            if (!($extensions -contains $file.Extension)) {
                continue
            }
            $content = Get-Content -LiteralPath $file.FullName -Raw
            if (!$content.Contains("catalog/v1/parcels/by-pnu")) {
                continue
            }
            $relative = Get-RepoRelativePath -RootPath $RootPath -FullPath $file.FullName
            if (!$allowed.Contains($relative)) {
                throw "platform-core-catalog-api-contract: unpinned local Catalog API consumer in $relative"
            }
        }
    }
}

function Assert-LocalClientMatchesEndpoint {
    param([string] $RootPath, [object] $Contract, [object] $Endpoint)

    $clientPath = Resolve-RepoPath -RootPath $RootPath -RelativePath $Endpoint.client_module
    if (!(Test-Path -LiteralPath $clientPath -PathType Leaf)) {
        throw "platform-core-catalog-api-contract: missing local client module: $($Endpoint.client_module)"
    }

    $content = Get-Content -LiteralPath $clientPath -Raw
    if ($content.Contains("PLATFORM_CORE_DATABASE_URL")) {
        throw "platform-core-catalog-api-contract: local client must use $($Contract.base_env), not direct Platform Core database configuration: $($Endpoint.client_module)"
    }
    if (!$content.Contains($Contract.base_env)) {
        throw "platform-core-catalog-api-contract: local client must use $($Contract.base_env): $($Endpoint.client_module)"
    }
    if (!$content.Contains($Endpoint.path_pattern_literal)) {
        throw "platform-core-catalog-api-contract: missing local path literal '$($Endpoint.path_pattern_literal)' in $($Endpoint.client_module)"
    }
    if (![regex]::IsMatch($content, "\.get\s*\(")) {
        throw "platform-core-catalog-api-contract: local client must use HTTP GET in $($Endpoint.client_module)"
    }
    if (!$content.Contains("execute(")) {
        throw "platform-core-catalog-api-contract: local client must use circuit_breaker::execute in $($Endpoint.client_module)"
    }
    if (!$content.Contains("Policy::platform_core_default()")) {
        throw "platform-core-catalog-api-contract: local client must use Policy::platform_core_default() in $($Endpoint.client_module)"
    }

    foreach ($field in @($Endpoint.required_response_fields)) {
        $fieldPattern = "(?m)\b$([regex]::Escape($field))\s*:"
        if (![regex]::IsMatch($content, $fieldPattern)) {
            throw "platform-core-catalog-api-contract: missing local response field '$field' in $($Endpoint.client_module)"
        }
    }
}

function Get-OpenApiPathSection {
    param([string] $Content, [string] $PathTemplate)

    $pathRegex = [regex]::Escape($PathTemplate)
    $pattern = "(?ms)^\s{{2}}{0}:\s*(?<section>.*?)(?=^\s{{2}}/|\ncomponents:|\z)" -f $pathRegex
    $match = [regex]::Match($Content, $pattern)
    if (!$match.Success) {
        return $null
    }
    return $match.Groups["section"].Value
}

function Get-OpenApiRequiredFields {
    param([string] $Content, [string] $SchemaName)

    $schemaRegex = [regex]::Escape($SchemaName)
    $pattern = "(?ms)^\s{{4}}{0}:\s*(?<section>.*?)(?=^\s{{4}}[A-Za-z0-9_]+:\s*|\z)" -f $schemaRegex
    $schemaMatch = [regex]::Match($Content, $pattern)
    if (!$schemaMatch.Success) {
        throw "platform-core-catalog-api-contract: missing Platform Core OpenAPI schema '$SchemaName'"
    }
    $section = $schemaMatch.Groups["section"].Value
    $requiredMatch = [regex]::Match($section, "required:\s*\[(?<fields>[^\]]+)\]")
    if (!$requiredMatch.Success) {
        throw "platform-core-catalog-api-contract: Platform Core OpenAPI schema '$SchemaName' has no inline required field list"
    }

    return @(
        $requiredMatch.Groups["fields"].Value.Split(",") |
            ForEach-Object { ($_ -replace "['""\s]", "") } |
            Where-Object { ![string]::IsNullOrWhiteSpace($_) } |
            Sort-Object -Unique
    )
}

function Get-ResponseSchemaName {
    param([string] $OperationId)

    switch ($OperationId) {
        "getParcelByPnu" { return "ParcelResponse" }
        "listParcelBuildingsByPnu" { return "BuildingResponse" }
        default {
            throw "platform-core-catalog-api-contract: unknown pinned operation '$OperationId'"
        }
    }
}

function Assert-OpenApiMatchesEndpoint {
    param([string] $OpenApiContent, [object] $Endpoint)

    $pathSection = Get-OpenApiPathSection -Content $OpenApiContent -PathTemplate $Endpoint.path_template
    if ($null -eq $pathSection) {
        throw "platform-core-catalog-api-contract: missing Platform Core OpenAPI path '$($Endpoint.path_template)'"
    }
    if (!$pathSection.Contains("operationId: $($Endpoint.operation_id)")) {
        throw "platform-core-catalog-api-contract: Platform Core OpenAPI operationId mismatch for '$($Endpoint.path_template)'"
    }

    $schemaName = Get-ResponseSchemaName -OperationId $Endpoint.operation_id
    if (!$pathSection.Contains("#/components/schemas/$schemaName")) {
        throw "platform-core-catalog-api-contract: Platform Core OpenAPI path '$($Endpoint.path_template)' must return $schemaName"
    }
    if ($Endpoint.success_shape -eq "array" -and !$pathSection.Contains("type: array")) {
        throw "platform-core-catalog-api-contract: Platform Core OpenAPI path '$($Endpoint.path_template)' must return array"
    }
    if ($Endpoint.success_shape -eq "object" -and $pathSection.Contains("type: array")) {
        throw "platform-core-catalog-api-contract: Platform Core OpenAPI path '$($Endpoint.path_template)' must return object"
    }

    $requiredFields = Get-OpenApiRequiredFields -Content $OpenApiContent -SchemaName $schemaName
    foreach ($field in @($Endpoint.required_response_fields)) {
        if (!($requiredFields -contains $field)) {
            throw "platform-core-catalog-api-contract: missing Platform Core OpenAPI field '$field' in $schemaName"
        }
    }
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$pinPath = Resolve-RepoPath -RootPath $resolvedRoot -RelativePath $PinRelativePath
$pin = Read-JsonFile -Path $pinPath -Label $PinRelativePath
$pinnedContract = Get-PinnedContract -Pin $pin
Assert-PinnedContractShape -Contract $pinnedContract

foreach ($endpoint in @($pinnedContract.endpoints)) {
    Assert-LocalClientMatchesEndpoint -RootPath $resolvedRoot -Contract $pinnedContract -Endpoint $endpoint
}
Assert-NoUnpinnedLocalCatalogApiConsumers -RootPath $resolvedRoot -Contract $pinnedContract

if ([string]::IsNullOrWhiteSpace($PlatformCoreRoot)) {
    $PlatformCoreRoot = Join-Path $resolvedRoot "..\platform-core"
}
$sourcePath = Resolve-RepoPath -RootPath $PlatformCoreRoot -RelativePath $pinnedContract.source_path
$sourceChecked = $false
if (Test-Path -LiteralPath $sourcePath -PathType Leaf) {
    $openApiContent = Get-Content -LiteralPath $sourcePath -Raw
    foreach ($endpoint in @($pinnedContract.endpoints)) {
        Assert-OpenApiMatchesEndpoint -OpenApiContent $openApiContent -Endpoint $endpoint
    }
    $sourceChecked = $true
}

Write-Host "platform-core-catalog-api-contract-ok endpoints=$(@($pinnedContract.endpoints).Count) source_checked=$sourceChecked"
