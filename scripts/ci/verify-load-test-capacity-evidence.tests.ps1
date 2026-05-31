Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "verify-load-test-capacity-evidence.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path (Join-Path $RepoRoot "target\verify-load-test-capacity-evidence-tests") ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File([string] $Root, [string] $RelativePath, [string] $Content) {
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Verifier([string] $EvidenceRoot) {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -EvidenceRoot $EvidenceRoot 2>&1
        [pscustomobject]@{ ExitCode = $LASTEXITCODE; Output = ($output -join [Environment]::NewLine) }
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
}

function Write-Evidence(
    [string] $Root,
    [string] $Scenario = "api-read-mix",
    [string] $Environment = "perf",
    [string] $Profile = "baseline",
    [string] $TargetBaseUrl = "https://perf.gongzzang.internal",
    [int] $K6ExitCode = 0,
    [string] $Classification = "healthy",
    [switch] $OmitSummary
) {
    $runRoot = Join-Path $Root "target\audit\load-tests\2026-05-29\$Environment\$Scenario\20260529T142920+0900"
    $runRelativeRoot = "target\audit\load-tests\2026-05-29\$Environment\$Scenario\20260529T142920+0900"
    Write-File $Root "$runRelativeRoot\run.json" @"
{
  "schemaVersion": "gongzzang.load.run.v1",
  "scenario": "$Scenario",
  "profile": "$Profile",
  "environment": "$Environment",
  "targetBaseUrl": "$TargetBaseUrl",
  "startedAt": "2026-05-29T14:29:20.0000000+09:00",
  "finishedAt": "2026-05-29T14:39:20.0000000+09:00",
  "gitSha": "0123456789abcdef",
  "k6ExitCode": $K6ExitCode
}
"@
    Write-File $Root "$runRelativeRoot\spec.json" @"
{
  "schemaVersion": "gongzzang.load.spec.v1",
  "scenario": "$Scenario",
  "profile": "$Profile",
  "environment": "$Environment",
  "targetBaseUrl": "$TargetBaseUrl",
  "maxSafeRps": 50
}
"@
    Write-File $Root "$runRelativeRoot\thresholds.json" @"
{
  "schemaVersion": "gongzzang.load.thresholds.v1",
  "scenario": "$Scenario",
  "slo": {
    "apiP95Ms": 300,
    "apiP99Ms": 1000,
    "errorRate": 0.01
  },
  "maxSafeRps": 50
}
"@
    if (!$OmitSummary) {
        Write-File $Root "$runRelativeRoot\k6-summary.json" @'
{
  "metrics": {
    "http_req_duration": {
      "values": {
        "p(95)": 120.0,
        "p(99)": 240.0
      }
    },
    "http_req_failed": {
      "values": {
        "rate": 0.0
      }
    }
  }
}
'@
    }
    Write-File $Root "$runRelativeRoot\bottleneck.md" "# Bottleneck`n`nClassification: $Classification`n"
    Write-File $Root "$runRelativeRoot\recommendation.md" "# Recommendation`n`nClassification: $Classification`n"
    Write-File $Root "$runRelativeRoot\baseline-comparison.md" "# Baseline Comparison`n"
    return $runRoot
}

function Write-RequiredEvidenceMatrix(
    [string] $Root,
    [string] $Environment = "perf",
    [string] $TargetBaseUrl = "https://perf.gongzzang.internal"
) {
    foreach ($scenario in @("api-read-mix", "map-marker-mix", "platform-core-events")) {
        Write-Evidence -Root $Root -Scenario $scenario -Environment $Environment -TargetBaseUrl $TargetBaseUrl | Out-Null
    }
}

function Assert-Contains([string] $Text, [string] $Expected) {
    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-RequiredEvidenceMatrix $okRoot
    $ok = Invoke-Verifier $okRoot
    if ($ok.ExitCode -ne 0) { throw "expected valid evidence to pass: $($ok.Output)" }
    Assert-Contains $ok.Output "load-test-capacity-evidence-ok"

    $apiOnlyRoot = Join-Path $TempRoot "api-only"
    Write-Evidence $apiOnlyRoot | Out-Null
    $apiOnly = Invoke-Verifier $apiOnlyRoot
    if ($apiOnly.ExitCode -eq 0) { throw "expected api-only evidence to fail" }
    Assert-Contains $apiOnly.Output "missing required load-test capacity scenario"

    $unapprovedTargetRoot = Join-Path $TempRoot "unapproved-target"
    Write-RequiredEvidenceMatrix -Root $unapprovedTargetRoot -TargetBaseUrl "https://load-runner.example.net"
    $unapprovedTarget = Invoke-Verifier $unapprovedTargetRoot
    if ($unapprovedTarget.ExitCode -eq 0) { throw "expected unapproved target evidence to fail" }
    Assert-Contains $unapprovedTarget.Output "target host must match capacity evidence environment"

    $stagingRoot = Join-Path $TempRoot "staging"
    Write-RequiredEvidenceMatrix -Root $stagingRoot -Environment "staging" -TargetBaseUrl "https://staging.gongzzang.internal"
    $staging = Invoke-Verifier $stagingRoot
    if ($staging.ExitCode -ne 0) { throw "expected staging evidence matrix to pass: $($staging.Output)" }
    Assert-Contains $staging.Output "load-test-capacity-evidence-ok"

    $localRoot = Join-Path $TempRoot "local"
    Write-Evidence -Root $localRoot -Environment "local" -Profile "smoke" -TargetBaseUrl "http://127.0.0.1:3000" | Out-Null
    $local = Invoke-Verifier $localRoot
    if ($local.ExitCode -eq 0) { throw "expected local smoke evidence to fail" }
    Assert-Contains $local.Output "perf or staging"

    $smokeRoot = Join-Path $TempRoot "smoke"
    Write-Evidence -Root $smokeRoot -Profile "smoke" | Out-Null
    $smoke = Invoke-Verifier $smokeRoot
    if ($smoke.ExitCode -eq 0) { throw "expected smoke profile evidence to fail" }
    Assert-Contains $smoke.Output "profile must be baseline, stress, spike, or soak"

    $productionTargetRoot = Join-Path $TempRoot "production-target"
    Write-Evidence -Root $productionTargetRoot -TargetBaseUrl "https://gongzzang.com" | Out-Null
    $productionTarget = Invoke-Verifier $productionTargetRoot
    if ($productionTarget.ExitCode -eq 0) { throw "expected production target evidence to fail" }
    Assert-Contains $productionTarget.Output "production targets are not valid load-test capacity evidence"

    $errorRoot = Join-Path $TempRoot "error"
    Write-Evidence -Root $errorRoot -Classification "error breakpoint" | Out-Null
    $errorBreakpoint = Invoke-Verifier $errorRoot
    if ($errorBreakpoint.ExitCode -eq 0) { throw "expected error breakpoint evidence to fail" }
    Assert-Contains $errorBreakpoint.Output "Classification must be healthy"

    $missingSummaryRoot = Join-Path $TempRoot "missing-summary"
    Write-Evidence -Root $missingSummaryRoot -OmitSummary | Out-Null
    $missingSummary = Invoke-Verifier $missingSummaryRoot
    if ($missingSummary.ExitCode -eq 0) { throw "expected missing summary evidence to fail" }
    Assert-Contains $missingSummary.Output "k6-summary.json"

    Write-Output "verify-load-test-capacity-evidence-tests-ok"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
