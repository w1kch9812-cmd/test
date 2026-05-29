param([string] $Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-File([string] $RelativePath) {
    $path = Join-Path $Root $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing required load-test asset: $RelativePath" }
}

function Read-Json([string] $RelativePath) {
    Get-Content -LiteralPath (Join-Path $Root $RelativePath) -Raw | ConvertFrom-Json
}

$requiredFiles = @(
    "docs/testing/load.md",
    "tests/load/README.md",
    "tests/load/scenarios.v1.json",
    "tests/load/lib/env.js",
    "tests/load/lib/http.js",
    "tests/load/scenarios/api-read-mix.js",
    "tests/load/scenarios/map-marker-mix.js",
    "tests/load/scenarios/capacity-stress.js",
    "tests/load/scenarios/platform-core-events.js",
    "scripts/load/run-k6.ps1",
    "scripts/load/normalize-k6-summary.ps1",
    ".github/workflows/load-test-capacity.yml"
)
$requiredFiles | ForEach-Object { Assert-File $_ }

$registry = Read-Json "tests/load/scenarios.v1.json"
if ($registry.schemaVersion -ne "gongzzang.load.scenarios.v1") { throw "scenario registry schemaVersion mismatch" }
if ([string] $registry.defaultTargetBaseUrl -match "gongzzang\.com|api\.gongzzang\.com") {
    throw "defaultTargetBaseUrl must not be production"
}
$requiredScenarios = [ordered]@{
    "api-read-mix" = "tests/load/scenarios/api-read-mix.js"
    "map-marker-mix" = "tests/load/scenarios/map-marker-mix.js"
    "capacity-stress" = "tests/load/scenarios/capacity-stress.js"
    "platform-core-events" = "tests/load/scenarios/platform-core-events.js"
}
$scenarios = @($registry.scenarios)
if ($scenarios.Count -ne $requiredScenarios.Count) {
    throw "scenario registry must contain exactly $($requiredScenarios.Count) scenarios"
}
foreach ($requiredId in $requiredScenarios.Keys) {
    $matchingScenarios = @($scenarios | Where-Object { [string] $_.id -eq $requiredId })
    if ($matchingScenarios.Count -ne 1) { throw "scenario registry missing required scenario: $requiredId" }

    $scenario = $matchingScenarios[0]
    $requiredFile = $requiredScenarios[$requiredId]
    if ([string] $scenario.file -ne $requiredFile) {
        throw "scenario registry file mismatch for $requiredId"
    }
}
foreach ($scenario in $scenarios) {
    Assert-File ([string] $scenario.file)
    if ([int] $scenario.maxSafeRps -lt 1) { throw "scenario maxSafeRps must be positive: $($scenario.id)" }
}

Write-Output "check-load-test-assets-ok scenarios=$($scenarios.Count)"
