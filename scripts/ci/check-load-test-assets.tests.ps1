Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-load-test-assets.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path (Join-Path $RepoRoot "target\check-load-test-assets-tests") ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File([string] $Root, [string] $RelativePath, [string] $Content) {
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker([string] $Root) {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
        [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($output -join [Environment]::NewLine) }
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
}

function Write-MinimalLoadAssets(
    [string] $Root,
    [switch] $UnsafeProductionTarget,
    [switch] $EmptyScenarios,
    [switch] $MissingSafetyRule,
    [switch] $MissingOperatorControls,
    [switch] $MissingCiGuardrail,
    [switch] $UnsafeBboxScenario,
    [switch] $MissingWebhookSigning
) {
    $target = if ($UnsafeProductionTarget) { "https://gongzzang.com" } else { "https://perf.gongzzang.internal" }
    $scenarios = if ($EmptyScenarios) {
        "[]"
    } else {
        @"
[
    {"id":"api-read-mix","file":"tests/load/scenarios/api-read-mix.js","maxSafeRps":50},
    {"id":"map-marker-mix","file":"tests/load/scenarios/map-marker-mix.js","maxSafeRps":50},
    {"id":"capacity-stress","file":"tests/load/scenarios/capacity-stress.js","maxSafeRps":800},
    {"id":"platform-core-events","file":"tests/load/scenarios/platform-core-events.js","maxSafeRps":50}
  ]
"@
    }
    $safetyRules = @(
        "- Do not run stress, spike, or soak tests against production user traffic paths.",
        "- Do not run tests that consume VWorld or OpenDataPortal quota from Gongzzang.",
        "- Do not test with production PII.",
        "- Do not log Authorization, Cookie, Set-Cookie, Platform Core service tokens, or webhook secrets.",
        "- Do not claim a launch spec without evidence under ``target/audit/load-tests``."
    )
    if ($MissingSafetyRule) {
        $safetyRules = @($safetyRules | Where-Object { $_ -notmatch "production PII" })
    }
    $operatorControls = @(
        "LOAD_APPROVED_TARGET_HOSTS",
        "LOAD_AUTH_BEARER_TOKEN",
        "maxSafeRps"
    )
    if ($MissingOperatorControls) {
        $operatorControls = @($operatorControls | Where-Object { $_ -ne "LOAD_AUTH_BEARER_TOKEN" })
    }

    Write-File $Root "docs\testing\load.md" "# Load Testing`n`n## Safety Rules`n`n$($safetyRules -join "`n")`n`n## Enterprise Gates`n`n$($operatorControls -join "`n")`n"
    Write-File $Root "tests\load\README.md" "# Load Scenarios`n"
    Write-File $Root "tests\load\scenarios.v1.json" @"
{
  "schemaVersion": "gongzzang.load.scenarios.v1",
  "defaultTargetBaseUrl": "$target",
  "scenarios": $scenarios
}
"@
    Write-File $Root "tests\load\lib\env.js" "production targets are forbidden for load tests`nTARGET_BASE_URL`n"
    Write-File $Root "tests\load\lib\http.js" "allowedTagKeys`nmaxTagValueLength`nsanitizeTags`nexpectedStatuses`n409`n429`n"
    Write-File $Root "tests\load\scenarios\api-read-mix.js" "/healthz`n/listings`n/api/parcels/`nLOAD_AUTH_BEARER_TOKEN`n"
    $mapScenario = if ($UnsafeBboxScenario) {
        "/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?bbox=unsafe`n"
    } else {
        "/api/proxy/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf`n/api/proxy/map/v1/marker-counts/listing`n/api/proxy/map/v1/marker-filters/listing`n/api/proxy/map/v1/marker-masks/listing`n"
    }
    Write-File $Root "tests\load\scenarios\map-marker-mix.js" $mapScenario
    Write-File $Root "tests\load\scenarios\capacity-stress.js" "ALLOW_STRESS`n/listings`nLOAD_AUTH_BEARER_TOKEN`n50`n100`n200`n300`n400`n600`n800`n"
    $webhookScenario = if ($MissingWebhookSigning) {
        "/platform-core/events`ncatalog.industrial_complex.gold_pointer.published.v1`ncomplex_id`ncurrent_version`nsource_snapshot_id`niceberg_snapshot_id`n"
    } else {
        "/platform-core/events`ncatalog.industrial_complex.gold_pointer.published.v1`ncomplex_id`ncurrent_version`nsource_snapshot_id`niceberg_snapshot_id`nx-platform-core-event-id`nx-platform-core-event-type`nx-platform-core-outbox-scope`n"
    }
    Write-File $Root "tests\load\scenarios\platform-core-events.js" $webhookScenario
    Write-File $Root "scripts\load\run-k6.ps1" "target\audit\load-tests`nAssert-ApprovedTarget`nAssert-MaxSafeRps`nLOAD_APPROVED_TARGET_HOSTS`nLOAD_AUTH_BEARER_TOKEN`nIsDefaultPort`nnon-local load-test targets must use the default https port`nsummary-export`nnormalize-k6-summary.ps1`n"
    Write-File $Root "scripts\load\normalize-k6-summary.ps1" "bottleneck.md`nrecommendation.md`nbaseline-comparison.md`nhealthy`nlatency breakpoint`nerror breakpoint`nexit 1`n"
    Write-File $Root ".github\workflows\load-test-capacity.yml" "workflow_dispatch`nruns-on: [self-hosted, load-test]`nupload-artifact`ntarget/audit/load-tests`nLOAD_INPUT_TARGET`n`$env:LOAD_INPUT_TARGET`nLOAD_INPUT_SCENARIO`n`$env:LOAD_INPUT_SCENARIO`n"
    $ciContent = if ($MissingCiGuardrail) {
        "name: CI`n"
    } else {
        "name: CI`nrun: ./scripts/ci/check-load-test-assets.ps1`nrun: ./scripts/ci/check-load-test-assets.tests.ps1`n"
    }
    Write-File $Root ".github\workflows\ci.yml" $ciContent
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-MinimalLoadAssets $okRoot
    $ok = Invoke-Checker $okRoot
    if ($ok.ExitCode -ne 0) { throw "expected ok fixture to pass: $($ok.Output)" }
    if (!$ok.Output.Contains("check-load-test-assets-ok scenarios=4")) {
        throw "expected ok fixture scenario count, got: $($ok.Output)"
    }

    $unsafeRoot = Join-Path $TempRoot "unsafe"
    Write-MinimalLoadAssets $unsafeRoot -UnsafeProductionTarget
    $unsafe = Invoke-Checker $unsafeRoot
    if ($unsafe.ExitCode -eq 0) { throw "expected production target fixture to fail" }
    if (!$unsafe.Output.Contains("defaultTargetBaseUrl must not be production")) {
        throw "expected production target error, got: $($unsafe.Output)"
    }

    $emptyRoot = Join-Path $TempRoot "empty"
    Write-MinimalLoadAssets $emptyRoot -EmptyScenarios
    $empty = Invoke-Checker $emptyRoot
    if ($empty.ExitCode -eq 0) { throw "expected empty scenarios fixture to fail" }
    if (!$empty.Output.Contains("scenario registry must contain exactly 4 scenarios")) {
        throw "expected scenario count error, got: $($empty.Output)"
    }

    $missingSafetyRoot = Join-Path $TempRoot "missing-safety"
    Write-MinimalLoadAssets $missingSafetyRoot -MissingSafetyRule
    $missingSafety = Invoke-Checker $missingSafetyRoot
    if ($missingSafety.ExitCode -eq 0) { throw "expected missing safety rule fixture to fail" }
    if (!$missingSafety.Output.Contains("missing load testing safety rule")) {
        throw "expected safety rule error, got: $($missingSafety.Output)"
    }

    $missingOperatorControlsRoot = Join-Path $TempRoot "missing-operator-controls"
    Write-MinimalLoadAssets $missingOperatorControlsRoot -MissingOperatorControls
    $missingOperatorControls = Invoke-Checker $missingOperatorControlsRoot
    if ($missingOperatorControls.ExitCode -eq 0) { throw "expected missing operator controls fixture to fail" }
    if (!$missingOperatorControls.Output.Contains("load testing manual missing operator control")) {
        throw "expected operator control error, got: $($missingOperatorControls.Output)"
    }

    $missingCiRoot = Join-Path $TempRoot "missing-ci"
    Write-MinimalLoadAssets $missingCiRoot -MissingCiGuardrail
    $missingCi = Invoke-Checker $missingCiRoot
    if ($missingCi.ExitCode -eq 0) { throw "expected missing CI guardrail fixture to fail" }
    if (!$missingCi.Output.Contains("CI workflow must run check-load-test-assets.ps1")) {
        throw "expected CI guardrail error, got: $($missingCi.Output)"
    }

    $unsafeBboxRoot = Join-Path $TempRoot "unsafe-bbox"
    Write-MinimalLoadAssets $unsafeBboxRoot -UnsafeBboxScenario
    $unsafeBbox = Invoke-Checker $unsafeBboxRoot
    if ($unsafeBbox.ExitCode -eq 0) { throw "expected unsafe bbox fixture to fail" }
    if (!$unsafeBbox.Output.Contains("bbox") -or !$unsafeBbox.Output.Contains("request shapes")) {
        throw "expected bbox guardrail error, got: $($unsafeBbox.Output)"
    }

    $missingWebhookHeadersRoot = Join-Path $TempRoot "missing-webhook-headers"
    Write-MinimalLoadAssets $missingWebhookHeadersRoot -MissingWebhookSigning
    $missingWebhookHeaders = Invoke-Checker $missingWebhookHeadersRoot
    if ($missingWebhookHeaders.ExitCode -eq 0) { throw "expected missing webhook headers fixture to fail" }
    if (!$missingWebhookHeaders.Output.Contains("platform-core event scenario missing required token")) {
        throw "expected webhook header error, got: $($missingWebhookHeaders.Output)"
    }

    Write-Output "check-load-test-assets-tests-ok"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
