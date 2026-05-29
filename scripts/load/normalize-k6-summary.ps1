param(
    [string] $RunDirectory,
    [string] $SummaryPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($RunDirectory)) {
    if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
        $RunDirectory = (Get-Location).Path
    } else {
        $RunDirectory = Split-Path -Parent ([System.IO.Path]::GetFullPath($SummaryPath))
    }
}

$RunDirectory = [System.IO.Path]::GetFullPath($RunDirectory)
if ([string]::IsNullOrWhiteSpace($SummaryPath)) {
    $SummaryPath = Join-Path $RunDirectory "k6-summary.json"
}

$SummaryPath = [System.IO.Path]::GetFullPath($SummaryPath)
$ThresholdsPath = Join-Path $RunDirectory "thresholds.json"
$RunPath = Join-Path $RunDirectory "run.json"
$BottleneckPath = Join-Path $RunDirectory "bottleneck.md"
$RecommendationPath = Join-Path $RunDirectory "recommendation.md"
$BaselinePath = Join-Path $RunDirectory "baseline-comparison.md"

function Read-JsonFile([string] $Path) {
    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $null
    }
    try {
        return Get-Content -LiteralPath $Path -Raw | ConvertFrom-Json
    } catch {
        return [pscustomobject]@{
            __invalidJson = $true
            __path = $Path
        }
    }
}

function Test-InvalidJson([object] $Value) {
    if ($null -eq $Value) {
        return $false
    }
    $property = $Value.PSObject.Properties["__invalidJson"]
    return $null -ne $property -and $property.Value -eq $true
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

function ConvertTo-OptionalDouble([object] $Value) {
    if ($null -eq $Value) {
        return [pscustomobject]@{
            Present = $false
            Valid = $false
            Value = $null
        }
    }

    $parsed = 0.0
    $stringValue = [string] $Value
    if ([double]::TryParse($stringValue, [System.Globalization.NumberStyles]::Float, [System.Globalization.CultureInfo]::InvariantCulture, [ref] $parsed)) {
        if ([double]::IsNaN($parsed) -or [double]::IsInfinity($parsed)) {
            return [pscustomobject]@{
                Present = $true
                Valid = $false
                Value = $null
            }
        }

        return [pscustomobject]@{
            Present = $true
            Valid = $true
            Value = $parsed
        }
    }

    return [pscustomobject]@{
        Present = $true
        Valid = $false
        Value = $null
    }
}

function Get-MetricValue([object] $Summary, [string] $MetricName, [string] $ValueName) {
    $metrics = Get-PropertyValue $Summary "metrics"
    if ($null -eq $metrics) {
        return ConvertTo-OptionalDouble $null
    }

    $metric = Get-PropertyValue $metrics $MetricName
    if ($null -eq $metric) {
        return ConvertTo-OptionalDouble $null
    }

    $values = Get-PropertyValue $metric "values"
    if ($null -ne $values) {
        $nestedValue = Get-PropertyValue $values $ValueName
        if ($null -ne $nestedValue) {
            return ConvertTo-OptionalDouble $nestedValue
        }
    }

    $directValue = Get-PropertyValue $metric $ValueName
    if ($null -ne $directValue) {
        return ConvertTo-OptionalDouble $directValue
    }

    if ($ValueName -eq "rate") {
        $legacyValue = Get-PropertyValue $metric "value"
        if ($null -ne $legacyValue) {
            return ConvertTo-OptionalDouble $legacyValue
        }
    }

    return ConvertTo-OptionalDouble $null
}

function Format-Value([Nullable[double]] $Value, [string] $Suffix) {
    if ($null -eq $Value) {
        return "missing"
    }
    return ("{0:N3}{1}" -f $Value, $Suffix)
}

function Write-Lines([string] $Path, [string[]] $Lines) {
    $Lines | Set-Content -LiteralPath $Path -Encoding UTF8
}

$summary = Read-JsonFile $SummaryPath
$thresholds = Read-JsonFile $ThresholdsPath
$run = Read-JsonFile $RunPath

$scenarioValue = Get-PropertyValue $run "scenario"
$profileValue = Get-PropertyValue $run "profile"
$environmentValue = Get-PropertyValue $run "environment"
$scenario = if ($null -ne $scenarioValue -and ![string]::IsNullOrWhiteSpace([string] $scenarioValue)) { [string] $scenarioValue } else { "unknown" }
$profile = if ($null -ne $profileValue -and ![string]::IsNullOrWhiteSpace([string] $profileValue)) { [string] $profileValue } else { "unknown" }
$environment = if ($null -ne $environmentValue -and ![string]::IsNullOrWhiteSpace([string] $environmentValue)) { [string] $environmentValue } else { "unknown" }

$apiP95Limit = 300.0
$apiP99Limit = 1000.0
$errorRateLimit = 0.01
$slo = Get-PropertyValue $thresholds "slo"
if ($null -ne $slo) {
    $apiP95Value = Get-PropertyValue $slo "apiP95Ms"
    $apiP99Value = Get-PropertyValue $slo "apiP99Ms"
    $errorRateValue = Get-PropertyValue $slo "errorRate"
    $apiP95Parsed = ConvertTo-OptionalDouble $apiP95Value
    $apiP99Parsed = ConvertTo-OptionalDouble $apiP99Value
    $errorRateParsed = ConvertTo-OptionalDouble $errorRateValue
    if ($apiP95Parsed.Valid) { $apiP95Limit = $apiP95Parsed.Value }
    if ($apiP99Parsed.Valid) { $apiP99Limit = $apiP99Parsed.Value }
    if ($errorRateParsed.Valid) { $errorRateLimit = $errorRateParsed.Value }
}

$p95Metric = Get-MetricValue $summary "http_req_duration" "p(95)"
$p99Metric = Get-MetricValue $summary "http_req_duration" "p(99)"
$errorRateMetric = Get-MetricValue $summary "http_req_failed" "rate"
$p95 = $p95Metric.Value
$p99 = $p99Metric.Value
$errorRate = $errorRateMetric.Value

$summaryMissing = $null -eq $summary
$summaryInvalid = Test-InvalidJson $summary
$classification = "healthy"
$reasons = New-Object System.Collections.Generic.List[string]

if ($summaryInvalid) {
    $classification = "error breakpoint"
    $reasons.Add("summary_json_invalid")
} elseif ($summaryMissing) {
    $classification = "error breakpoint"
    $reasons.Add("k6-summary.json was not produced.")
} else {
    if (($p95Metric.Present -and !$p95Metric.Valid) -or ($p99Metric.Present -and !$p99Metric.Valid) -or ($errorRateMetric.Present -and !$errorRateMetric.Valid)) {
        $classification = "error breakpoint"
        $reasons.Add("summary_metric_invalid")
    }

    if ($null -eq $errorRate) {
        $classification = "error breakpoint"
        $reasons.Add("http_req_failed rate is missing.")
    } elseif ($errorRate -ge $errorRateLimit) {
        $classification = "error breakpoint"
        $reasons.Add("http_req_failed rate breached the SLO.")
    }

    if ($null -eq $p95 -or $null -eq $p99) {
        if ($classification -eq "healthy") {
            $classification = "latency breakpoint"
        }
        $reasons.Add("http_req_duration percentile metrics are missing.")
    } elseif ($p95 -ge $apiP95Limit -or $p99 -ge $apiP99Limit) {
        if ($classification -eq "healthy") {
            $classification = "latency breakpoint"
        }
        $reasons.Add("http_req_duration breached the latency SLO.")
    }
}

if ($reasons.Count -eq 0) {
    $reasons.Add("http_req_duration and http_req_failed stayed inside the first-version SLO gates.")
}

$metricLines = @(
    "- Scenario: $scenario",
    "- Profile: $profile",
    "- Environment: $environment",
    "- Classification: $classification",
    "- http_req_duration p95: $(Format-Value $p95 " ms") (limit $apiP95Limit ms)",
    "- http_req_duration p99: $(Format-Value $p99 " ms") (limit $apiP99Limit ms)",
    "- http_req_failed rate: $(Format-Value $errorRate ([string]::Empty)) (limit $errorRateLimit)"
)

Write-Lines $BottleneckPath (@(
    "# Bottleneck",
    "",
    "Classification: $classification",
    "",
    "## Metrics"
) + $metricLines + @(
    "",
    "## First Analysis"
) + @($reasons | ForEach-Object { "- $_" }))

$recommendation = switch ($classification) {
    "healthy" { "Keep this run as a candidate baseline only after resource telemetry is reviewed." }
    "latency breakpoint" { "Inspect route latency, queueing, slow spans, and database query evidence before increasing capacity." }
    "error breakpoint" { "Fix connection failures, target availability, rejected requests, or dependency errors before using this run for launch sizing." }
    default { "Review the evidence before making a sizing decision." }
}

Write-Lines $RecommendationPath @(
    "# Recommendation",
    "",
    "Classification: $classification",
    "",
    $recommendation,
    "",
    "This first-version normalizer uses k6 http_req_duration and http_req_failed only."
)

Write-Lines $BaselinePath (@(
    "# Baseline Comparison",
    "",
    "No accepted baseline lookup is implemented in this first version.",
    "",
    "## Current Run"
) + $metricLines)

Write-Output "normalize-k6-summary-ok classification=$classification"
