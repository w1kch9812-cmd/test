[CmdletBinding()]
param(
    [string] $Root = "",
    [switch] $IncludeProductionPromotion
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

$ModuleRoot = Join-Path $PSScriptRoot "integration-policy-platform"
. (Join-Path $ModuleRoot "shared.ps1")

. (Join-Path $ModuleRoot "phase-01-index-schema-contract.ps1")
. (Join-Path $ModuleRoot "phase-02-route-call-service-auth.ps1")
. (Join-Path $ModuleRoot "phase-03-webhook-exception-policy.ps1")
. (Join-Path $ModuleRoot "phase-04-supply-chain-policy.ps1")
. (Join-Path $ModuleRoot "phase-05-operations-ci-policy.ps1")

Write-Host "platform-integration-policy-ok components=$($components.Count) route_surfaces=$(@($routePolicy.surfaces).Count)"
