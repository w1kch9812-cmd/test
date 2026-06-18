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

function Resolve-RepoPath {
    param([string] $RelativePath)

    [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-JsonObject {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "traffic-auth registry fragment is missing: $RelativePath"
    }

    Get-Content -LiteralPath $path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Get-RequiredProperty {
    param([object] $Object, [string] $Name, [string] $RelativePath)

    $property = $Object.PSObject.Properties[$Name]
    if ($null -eq $property) {
        throw "traffic-auth registry fragment '$RelativePath' missing '$Name'"
    }
    return $property.Value
}

$metadataPath = "docs/architecture/traffic-auth-policy-registry/00-metadata.json"
$metadata = Read-JsonObject -RelativePath $metadataPath

$registry = [ordered]@{
    schema_version             = Get-RequiredProperty -Object $metadata -Name "schema_version" -RelativePath $metadataPath
    repo_slug                  = Get-RequiredProperty -Object $metadata -Name "repo_slug" -RelativePath $metadataPath
    decision_sources           = @(Get-RequiredProperty -Object $metadata -Name "decision_sources" -RelativePath $metadataPath)
    policy_principles          = @(Get-RequiredProperty -Object $metadata -Name "policy_principles" -RelativePath $metadataPath)
    exposure_classes           = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/10-exposure-classes.json") -Name "exposure_classes" -RelativePath "10-exposure-classes.json")
    public_route_policies      = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/20-public-route-policies.json") -Name "public_route_policies" -RelativePath "20-public-route-policies.json")
    auth_route_policies        = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/30-auth-route-policies.json") -Name "auth_route_policies" -RelativePath "30-auth-route-policies.json")
    route_rate_profiles        = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/40-route-rate-profiles.json") -Name "route_rate_profiles" -RelativePath "40-route-rate-profiles.json")
    page_route_policies        = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/50-page-route-policies.json") -Name "page_route_policies" -RelativePath "50-page-route-policies.json")
    api_proxy_route_policies   = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/60-api-proxy-route-policies.json") -Name "api_proxy_route_policies" -RelativePath "60-api-proxy-route-policies.json")
    backend_route_policies     = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/70-backend-route-policies.json") -Name "backend_route_policies" -RelativePath "70-backend-route-policies.json")
    service_call_policies      = @(Get-RequiredProperty -Object (Read-JsonObject -RelativePath "docs/architecture/traffic-auth-policy-registry/80-service-call-policies.json") -Name "service_call_policies" -RelativePath "80-service-call-policies.json")
}

$outputPath = Resolve-RepoPath -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
$json = $registry | ConvertTo-Json -Depth 32
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText($outputPath, ($json + "`n"), $utf8NoBom)

Write-Host "traffic-auth-policy-registry-generated source=docs/architecture/traffic-auth-policy-registry output=docs/architecture/traffic-auth-policy-registry.v1.json"
