[CmdletBinding()]
param(
    [string] $Root = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}
$ResolvedRoot = [System.IO.Path]::GetFullPath($Root)

function Resolve-BiomeExecutable {
    param([string] $RootPath)

    $candidates = @(
        "node_modules/.bin/biome.CMD",
        "node_modules/.bin/biome.cmd",
        "node_modules/.bin/biome",
        "node_modules/.bin/biome.ps1"
    )
    foreach ($candidate in $candidates) {
        $path = [System.IO.Path]::GetFullPath((Join-Path $RootPath ($candidate -replace "/", "\")))
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            return $path
        }
    }

    throw "Biome executable is missing; run package install before generating traffic/auth policy outputs."
}

& (Join-Path $PSScriptRoot "generate-traffic-auth-policy-registry.ps1") -Root $Root

$ModuleRoot = Join-Path $PSScriptRoot "traffic-auth-policy-generator"
. (Join-Path $ModuleRoot "shared.ps1")
. (Join-Path $ModuleRoot "phase-01-load-registry.ps1")
. (Join-Path $ModuleRoot "phase-02-web-route-policy.ps1")
. (Join-Path $ModuleRoot "phase-03-api-proxy-client.ps1")
. (Join-Path $ModuleRoot "rust-edge-shared.ps1")
. (Join-Path $ModuleRoot "phase-04-rust-policies.ps1")
. (Join-Path $ModuleRoot "phase-05-edge-projection.ps1")
. (Join-Path $ModuleRoot "phase-06-aws-waf-manifest.ps1")

$formatTargets = @(
    "apps/web/lib/api/api-proxy-client.generated.ts",
    "apps/web/lib/policies/traffic-auth-policy.generated.ts",
    "docs/architecture/traffic-auth-policy-registry.v1.json",
    "infrastructure/security/aws-wafv2-edge-policy.generated.json",
    "infrastructure/security/traffic-auth-edge-policy.generated.json"
)
$biome = Resolve-BiomeExecutable -RootPath $ResolvedRoot
& $biome format --write @formatTargets | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "Biome formatting failed for generated traffic/auth policy outputs."
}

Write-Host "traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts rust=services/api/src/listing_marker_policy.rs,services/api/src/traffic_auth_policy.rs edge=infrastructure/security/traffic-auth-edge-policy.generated.json aws_wafv2=infrastructure/security/aws-wafv2-edge-policy.generated.json"
