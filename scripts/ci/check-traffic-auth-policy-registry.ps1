[CmdletBinding()]
param(
    [string] $Root = "",
    [switch] $IncludeProductionEdge
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ExpectedSchemaVersion = "gongzzang.traffic_auth_policy_registry.v1"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

$ModuleRoot = Join-Path $PSScriptRoot "traffic-auth-policy-registry"
. (Join-Path $ModuleRoot "shared.ps1")

. (Join-Path $ModuleRoot "phase-01-registry-and-source-contract.ps1")
. (Join-Path $ModuleRoot "phase-02-production-edge-policy.ps1")
. (Join-Path $ModuleRoot "phase-03-runtime-binding-contract.ps1")
. (Join-Path $ModuleRoot "phase-04-auth-route-policies.ps1")
. (Join-Path $ModuleRoot "phase-05-page-route-policies.ps1")
. (Join-Path $ModuleRoot "phase-06-backend-route-policies.ps1")
. (Join-Path $ModuleRoot "phase-07-api-proxy-route-policies.ps1")
. (Join-Path $ModuleRoot "phase-08-public-map-route-policies.ps1")
. (Join-Path $ModuleRoot "phase-09-service-boundary-policies.ps1")

Write-Host "traffic-auth-policy-registry-ok routes=$($publicRoutes.Count) service_policies=$($servicePolicies.Count)"
