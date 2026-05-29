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

function Read-Text([string] $RelativePath) {
    Assert-File $RelativePath
    return Get-Content -LiteralPath (Join-Path $Root $RelativePath) -Raw
}

function Assert-Contains([string] $RelativePath, [string] $Needle, [string] $Message) {
    $content = Read-Text $RelativePath
    if (!$content.Contains($Needle)) {
        throw $Message
    }
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
    ".github/workflows/load-test-capacity.yml",
    ".github/workflows/ci.yml"
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

$requiredSafetyRules = @(
    '- Do not run stress, spike, or soak tests against production user traffic paths.',
    '- Do not run tests that consume VWorld or OpenDataPortal quota from Gongzzang.',
    '- Do not test with production PII.',
    '- Do not log Authorization, Cookie, Set-Cookie, Platform Core service tokens, or webhook secrets.',
    '- Do not claim a launch spec without evidence under `target/audit/load-tests`.'
)
$loadManual = Read-Text "docs/testing/load.md"
foreach ($rule in $requiredSafetyRules) {
    if (!$loadManual.Contains($rule)) {
        throw "missing load testing safety rule"
    }
}
foreach ($operatorControl in @("LOAD_APPROVED_TARGET_HOSTS", "LOAD_AUTH_BEARER_TOKEN", "maxSafeRps")) {
    if (!$loadManual.Contains($operatorControl)) {
        throw "load testing manual missing operator control: $operatorControl"
    }
}

Assert-Contains `
    -RelativePath ".github/workflows/ci.yml" `
    -Needle "check-load-test-assets.ps1" `
    -Message "CI workflow must run check-load-test-assets.ps1"
Assert-Contains `
    -RelativePath ".github/workflows/ci.yml" `
    -Needle "check-load-test-assets.tests.ps1" `
    -Message "CI workflow must run check-load-test-assets.tests.ps1"

$manualWorkflow = Read-Text ".github/workflows/load-test-capacity.yml"
foreach ($needle in @("workflow_dispatch", "self-hosted", "load-test", "upload-artifact", "target/audit/load-tests")) {
    if (!$manualWorkflow.Contains($needle)) {
        throw "load-test capacity workflow missing required token: $needle"
    }
}
if ($manualWorkflow.Contains('${{ secrets.')) {
    throw "load-test workflow must not reference GitHub secrets"
}

$envLib = Read-Text "tests/load/lib/env.js"
foreach ($needle in @("TARGET_BASE_URL", "production targets are forbidden for load tests")) {
    if (!$envLib.Contains($needle)) {
        throw "load env helper missing required token: $needle"
    }
}

$httpLib = Read-Text "tests/load/lib/http.js"
foreach ($needle in @("allowedTagKeys", "maxTagValueLength", "sanitizeTags")) {
    if (!$httpLib.Contains($needle)) {
        throw "load http helper missing redaction-safe token: $needle"
    }
}
if ($httpLib -match "expectedStatuses[^\r\n]*404" -or $httpLib -match "status\s*===\s*404") {
    throw "load http helper must not count 404 as success"
}

$apiScenario = Read-Text "tests/load/scenarios/api-read-mix.js"
foreach ($needle in @("/healthz", "/listings", "/api/parcels/", "LOAD_AUTH_BEARER_TOKEN")) {
    if (!$apiScenario.Contains($needle)) {
        throw "api-read-mix scenario missing required route: $needle"
    }
}

$mapScenario = Read-Text "tests/load/scenarios/map-marker-mix.js"
if ($mapScenario -match "(?i)(\?|&)(bbox|bounds)=") {
    throw "load-test marker scenario must not use public bbox or bounds request shapes"
}
foreach ($needle in @("marker-tiles/listing", "marker-counts/listing", "marker-filters/listing", "marker-masks/listing")) {
    if (!$mapScenario.Contains($needle)) {
        throw "map-marker-mix scenario missing required marker path: $needle"
    }
}

$stressScenario = Read-Text "tests/load/scenarios/capacity-stress.js"
foreach ($needle in @("ALLOW_STRESS", "/listings", "LOAD_AUTH_BEARER_TOKEN", "50", "100", "200", "300", "400", "600", "800")) {
    if (!$stressScenario.Contains($needle)) {
        throw "capacity-stress scenario missing required token: $needle"
    }
}

$eventScenario = Read-Text "tests/load/scenarios/platform-core-events.js"
foreach ($needle in @(
    "/platform-core/events",
    "PLATFORM_CORE_WEBHOOK_SECRET",
    "catalog.industrial_complex.gold_pointer.published.v1",
    "complex_id",
    "current_version",
    "source_snapshot_id",
    "iceberg_snapshot_id"
)) {
    if (!$eventScenario.Contains($needle)) {
        throw "platform-core event scenario missing required token: $needle"
    }
}
if (!$eventScenario.Contains("x-platform-core-signature") -or $eventScenario -notmatch "crypto\.hmac|hmac\(") {
    throw "platform-core event scenario must sign webhook requests"
}

$launcher = Read-Text "scripts/load/run-k6.ps1"
foreach ($needle in @("target\audit\load-tests", "Assert-ApprovedTarget", "Assert-MaxSafeRps", "LOAD_APPROVED_TARGET_HOSTS", "summary-export", "normalize-k6-summary.ps1")) {
    if (!$launcher.Contains($needle)) {
        throw "load-test launcher missing required token: $needle"
    }
}
if (!$launcher.Contains("LOAD_AUTH_BEARER_TOKEN")) {
    throw "load-test launcher must pass approved auth bearer token env only"
}

$normalizer = Read-Text "scripts/load/normalize-k6-summary.ps1"
foreach ($needle in @("bottleneck.md", "recommendation.md", "baseline-comparison.md", "healthy", "latency breakpoint", "error breakpoint", "exit 1")) {
    if (!$normalizer.Contains($needle)) {
        throw "load-test normalizer missing required token: $needle"
    }
}

Write-Output "check-load-test-assets-ok scenarios=$($scenarios.Count)"
