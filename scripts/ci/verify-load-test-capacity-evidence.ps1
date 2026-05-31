[CmdletBinding()]
param(
    [string] $EvidenceRoot = "",

    [string] $RunDirectory = "",

    [string[]] $RequiredScenarios = @("api-read-mix", "map-marker-mix", "platform-core-events")
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-InputPath([string] $Path, [string] $FallbackRelativePath) {
    if ([string]::IsNullOrWhiteSpace($Path)) {
        $Path = Join-Path (Get-Location).Path $FallbackRelativePath
    }
    return [System.IO.Path]::GetFullPath($Path)
}

function Assert-File([string] $Path, [string] $Name) {
    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "$Name is required: $Path"
    }
}

function Read-JsonFile([string] $Path, [string] $Name) {
    Assert-File -Path $Path -Name $Name
    try {
        return Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    } catch {
        throw "$Name must be valid JSON: $Path"
    }
}

function Get-PropertyValue([object] $Value, [string] $Name) {
    if ($null -eq $Value) {
        return $null
    }
    $property = $Value.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function ConvertTo-RequiredDouble([object] $Value, [string] $Name) {
    if ($null -eq $Value) {
        throw "$Name is missing"
    }
    $parsed = 0.0
    if (![double]::TryParse(
            [string] $Value,
            [System.Globalization.NumberStyles]::Float,
            [System.Globalization.CultureInfo]::InvariantCulture,
            [ref] $parsed
        )) {
        throw "$Name must be numeric"
    }
    if ([double]::IsNaN($parsed) -or [double]::IsInfinity($parsed)) {
        throw "$Name must be finite"
    }
    return $parsed
}

function Get-MetricValue([object] $Summary, [string] $MetricName, [string] $ValueName) {
    $metrics = Get-PropertyValue -Value $Summary -Name "metrics"
    $metric = Get-PropertyValue -Value $metrics -Name $MetricName
    $values = Get-PropertyValue -Value $metric -Name "values"
    $value = Get-PropertyValue -Value $values -Name $ValueName
    if ($null -eq $value -and $ValueName -eq "rate") {
        $value = Get-PropertyValue -Value $metric -Name "rate"
    }
    if ($null -eq $value -and $ValueName -eq "rate") {
        $value = Get-PropertyValue -Value $metric -Name "value"
    }
    return ConvertTo-RequiredDouble -Value $value -Name "$MetricName $ValueName"
}

function Assert-ApprovedCapacityTarget([string] $TargetBaseUrl, [string] $EnvironmentName) {
    if ([string]::IsNullOrWhiteSpace($TargetBaseUrl)) {
        throw "targetBaseUrl is required"
    }
    try {
        $uri = [System.Uri] $TargetBaseUrl
    } catch {
        throw "targetBaseUrl must be a valid URL"
    }
    if (!$uri.IsAbsoluteUri -or $uri.Scheme -ne "https") {
        throw "capacity evidence targetBaseUrl must be https"
    }
    if (![string]::IsNullOrWhiteSpace($uri.UserInfo) -or ![string]::IsNullOrWhiteSpace($uri.Query) -or ![string]::IsNullOrWhiteSpace($uri.Fragment)) {
        throw "capacity evidence targetBaseUrl must not contain userinfo, query, or fragment"
    }
    $targetHost = $uri.Host.ToLowerInvariant().TrimEnd(".")
    if ($targetHost -eq "localhost" -or $targetHost -eq "127.0.0.1" -or $targetHost -eq "::1") {
        throw "local targets are not valid load-test capacity evidence"
    }
    if ($targetHost -eq "gongzzang.com" -or $targetHost -eq "www.gongzzang.com" -or $targetHost.EndsWith(".gongzzang.com")) {
        throw "production targets are not valid load-test capacity evidence"
    }
    $expectedTargetHost = switch ($EnvironmentName) {
        "perf" { "perf.gongzzang.internal" }
        "staging" { "staging.gongzzang.internal" }
        default { "" }
    }
    if ($targetHost -ne $expectedTargetHost) {
        throw "target host must match capacity evidence environment: environment=$EnvironmentName expected=$expectedTargetHost actual=$targetHost"
    }
}

function Assert-SummaryWithinSlo([object] $Summary, [object] $Thresholds) {
    $slo = Get-PropertyValue -Value $Thresholds -Name "slo"
    $apiP95Limit = ConvertTo-RequiredDouble -Value (Get-PropertyValue -Value $slo -Name "apiP95Ms") -Name "slo.apiP95Ms"
    $apiP99Limit = ConvertTo-RequiredDouble -Value (Get-PropertyValue -Value $slo -Name "apiP99Ms") -Name "slo.apiP99Ms"
    $errorRateLimit = ConvertTo-RequiredDouble -Value (Get-PropertyValue -Value $slo -Name "errorRate") -Name "slo.errorRate"

    $p95 = Get-MetricValue -Summary $Summary -MetricName "http_req_duration" -ValueName "p(95)"
    $p99 = Get-MetricValue -Summary $Summary -MetricName "http_req_duration" -ValueName "p(99)"
    $errorRate = Get-MetricValue -Summary $Summary -MetricName "http_req_failed" -ValueName "rate"

    if ($p95 -gt $apiP95Limit) {
        throw "http_req_duration p95 exceeds launch SLO: actual=$p95 limit=$apiP95Limit"
    }
    if ($p99 -gt $apiP99Limit) {
        throw "http_req_duration p99 exceeds launch SLO: actual=$p99 limit=$apiP99Limit"
    }
    if ($errorRate -gt $errorRateLimit) {
        throw "http_req_failed rate exceeds launch SLO: actual=$errorRate limit=$errorRateLimit"
    }
}

function Test-CapacityEvidenceRun([string] $CandidateRunDirectory) {
    $runPath = Join-Path $CandidateRunDirectory "run.json"
    $specPath = Join-Path $CandidateRunDirectory "spec.json"
    $thresholdsPath = Join-Path $CandidateRunDirectory "thresholds.json"
    $summaryPath = Join-Path $CandidateRunDirectory "k6-summary.json"
    $bottleneckPath = Join-Path $CandidateRunDirectory "bottleneck.md"
    $recommendationPath = Join-Path $CandidateRunDirectory "recommendation.md"
    $baselinePath = Join-Path $CandidateRunDirectory "baseline-comparison.md"

    $run = Read-JsonFile -Path $runPath -Name "run.json"
    $spec = Read-JsonFile -Path $specPath -Name "spec.json"
    $thresholds = Read-JsonFile -Path $thresholdsPath -Name "thresholds.json"
    $summary = Read-JsonFile -Path $summaryPath -Name "k6-summary.json"
    Assert-File -Path $bottleneckPath -Name "bottleneck.md"
    Assert-File -Path $recommendationPath -Name "recommendation.md"
    Assert-File -Path $baselinePath -Name "baseline-comparison.md"

    if ([string] (Get-PropertyValue -Value $run -Name "schemaVersion") -ne "gongzzang.load.run.v1") {
        throw "run.json schemaVersion mismatch"
    }
    if ([string] (Get-PropertyValue -Value $spec -Name "schemaVersion") -ne "gongzzang.load.spec.v1") {
        throw "spec.json schemaVersion mismatch"
    }
    if ([string] (Get-PropertyValue -Value $thresholds -Name "schemaVersion") -ne "gongzzang.load.thresholds.v1") {
        throw "thresholds.json schemaVersion mismatch"
    }

    $scenario = [string] (Get-PropertyValue -Value $run -Name "scenario")
    if ([string]::IsNullOrWhiteSpace($scenario)) {
        throw "run.json scenario is required"
    }
    if ([string] (Get-PropertyValue -Value $spec -Name "scenario") -ne $scenario) {
        throw "spec.json scenario must match run.json scenario"
    }
    if ([string] (Get-PropertyValue -Value $thresholds -Name "scenario") -ne $scenario) {
        throw "thresholds.json scenario must match run.json scenario"
    }

    $environment = [string] (Get-PropertyValue -Value $run -Name "environment")
    if ($environment -ne "perf" -and $environment -ne "staging") {
        throw "capacity evidence environment must be perf or staging"
    }

    $profile = [string] (Get-PropertyValue -Value $run -Name "profile")
    if (@("baseline", "stress", "spike", "soak") -notcontains $profile) {
        throw "capacity evidence profile must be baseline, stress, spike, or soak"
    }

    $targetBaseUrl = [string] (Get-PropertyValue -Value $run -Name "targetBaseUrl")
    Assert-ApprovedCapacityTarget -TargetBaseUrl $targetBaseUrl -EnvironmentName $environment

    $k6ExitCode = [int] (Get-PropertyValue -Value $run -Name "k6ExitCode")
    if ($k6ExitCode -ne 0) {
        throw "k6ExitCode must be 0 for production capacity evidence"
    }

    $bottleneck = Get-Content -LiteralPath $bottleneckPath -Raw -Encoding UTF8
    if (!$bottleneck.Contains("Classification: healthy")) {
        throw "Classification must be healthy"
    }

    Assert-SummaryWithinSlo -Summary $summary -Thresholds $thresholds

    return [pscustomobject]@{
        RunDirectory = $CandidateRunDirectory
        Scenario = $scenario
        Profile = $profile
        Environment = $environment
        TargetBaseUrl = $targetBaseUrl
    }
}

$candidateRunDirectories = @()
if (![string]::IsNullOrWhiteSpace($RunDirectory)) {
    $candidateRunDirectories = @(Resolve-InputPath -Path $RunDirectory -FallbackRelativePath ".")
} else {
    $resolvedEvidenceRoot = Resolve-InputPath -Path $EvidenceRoot -FallbackRelativePath "target\audit\load-tests"
    if (!(Test-Path -LiteralPath $resolvedEvidenceRoot -PathType Container)) {
        throw "EvidenceRoot does not exist: $resolvedEvidenceRoot"
    }
    $candidateRunDirectories = @(
        Get-ChildItem -LiteralPath $resolvedEvidenceRoot -Recurse -Filter "run.json" -File |
            ForEach-Object { Split-Path -Parent $_.FullName }
    )
}

if ($candidateRunDirectories.Count -eq 0) {
    throw "No load-test run.json files found under evidence root"
}

$validEvidenceByScenario = @{}
$failures = New-Object System.Collections.Generic.List[string]
foreach ($candidate in $candidateRunDirectories) {
    try {
        $result = Test-CapacityEvidenceRun -CandidateRunDirectory $candidate
        if (!$validEvidenceByScenario.ContainsKey($result.Scenario)) {
            $validEvidenceByScenario[$result.Scenario] = $result
        }
    } catch {
        $failures.Add("$candidate :: $($_.Exception.Message)")
    }
}

$missingScenarios = @($RequiredScenarios | Where-Object { !$validEvidenceByScenario.ContainsKey([string] $_) })
if ($missingScenarios.Count -gt 0) {
    $failureText = if ($failures.Count -gt 0) { " Failures: $($failures -join '; ')" } else { "" }
    throw "missing required load-test capacity scenario: $($missingScenarios -join ',').$failureText"
}

$accepted = @($RequiredScenarios | ForEach-Object {
    $scenario = [string] $_
    $result = $validEvidenceByScenario[$scenario]
    "$scenario/$($result.Profile)/$($result.Environment)"
})
Write-Output "load-test-capacity-evidence-ok required_scenarios=$($accepted -join ',')"
