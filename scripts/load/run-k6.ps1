param(
    [Parameter(Mandatory = $true)]
    [string] $Scenario,

    [string] $TargetBaseUrl,

    [ValidateSet("smoke", "baseline", "stress", "spike", "soak")]
    [string] $Profile = "smoke",

    [ValidateSet("perf", "staging", "local", "ci")]
    [string] $Environment,

    [string] $OutRoot,

    [switch] $AllowStress
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$RegistryPath = Join-Path $RepoRoot "tests\load\scenarios.v1.json"
$NormalizerPath = Join-Path $PSScriptRoot "normalize-k6-summary.ps1"

function ConvertTo-JsonFile([string] $Path, [object] $Value) {
    $Value | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath $Path -Encoding UTF8
}

function Get-GitSha {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & git -C $RepoRoot rev-parse HEAD 2>$null
        if ($LASTEXITCODE -eq 0 -and $output) {
            return [string] $output
        }
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    return "unknown"
}

function Resolve-TargetBaseUrl([string] $Url) {
    $candidate = $Url.Trim()
    if ([string]::IsNullOrWhiteSpace($candidate)) {
        throw "TargetBaseUrl is required"
    }
    if ($candidate -match "\s") {
        throw "TargetBaseUrl must be a valid URL"
    }

    try {
        $uri = [System.Uri] $candidate
    } catch {
        throw "TargetBaseUrl must be a valid URL"
    }

    if (!$uri.IsAbsoluteUri -or ($uri.Scheme -ne "http" -and $uri.Scheme -ne "https")) {
        throw "TargetBaseUrl must be an absolute http or https URL"
    }

    if (![string]::IsNullOrEmpty($uri.UserInfo) -or ![string]::IsNullOrEmpty($uri.Query) -or ![string]::IsNullOrEmpty($uri.Fragment) -or $uri.AbsolutePath -ne "/") {
        throw "TargetBaseUrl must not include path, userinfo, query, or fragment"
    }

    $safeHost = $uri.Host.ToLowerInvariant().TrimEnd(".")
    if ([string]::IsNullOrWhiteSpace($safeHost)) {
        throw "TargetBaseUrl must be a valid URL"
    }

    $hostForUrl = if ($safeHost.Contains(":") -and !$safeHost.StartsWith("[")) { "[$safeHost]" } else { $safeHost }
    $portForUrl = if ($uri.IsDefaultPort) { "" } else { ":$($uri.Port)" }
    return [pscustomobject]@{
        Host = $safeHost
        NormalizedUrl = "$($uri.Scheme)://$hostForUrl$portForUrl"
    }
}

function Assert-NonProductionTarget([string] $SafeHost) {
    $safeHost = $SafeHost
    if ($safeHost -eq "perf.gongzzang.internal") {
        return
    }
    if ($safeHost -eq "gongzzang.com" -or $safeHost -eq "www.gongzzang.com" -or $safeHost.EndsWith(".gongzzang.com")) {
        throw "production targets are forbidden for load tests"
    }
}

function Get-ProfileConfig([string] $ProfileName) {
    switch ($ProfileName) {
        "smoke" {
            return [ordered]@{
                LOAD_RPS = "2"
                LOAD_DURATION = "15s"
                LOAD_PRE_ALLOCATED_VUS = "2"
                LOAD_MAX_VUS = "10"
                LOAD_STRESS_STAGE_DURATION = "10s"
            }
        }
        "baseline" {
            return [ordered]@{
                LOAD_RPS = "20"
                LOAD_DURATION = "10m"
                LOAD_PRE_ALLOCATED_VUS = "40"
                LOAD_MAX_VUS = "200"
                LOAD_STRESS_STAGE_DURATION = "1m"
            }
        }
        "stress" {
            return [ordered]@{
                LOAD_RPS = "50"
                LOAD_DURATION = "15m"
                LOAD_PRE_ALLOCATED_VUS = "100"
                LOAD_MAX_VUS = "2000"
                LOAD_STRESS_STAGE_DURATION = "2m"
            }
        }
        "spike" {
            return [ordered]@{
                LOAD_RPS = "100"
                LOAD_DURATION = "3m"
                LOAD_PRE_ALLOCATED_VUS = "100"
                LOAD_MAX_VUS = "1000"
                LOAD_STRESS_STAGE_DURATION = "30s"
            }
        }
        "soak" {
            return [ordered]@{
                LOAD_RPS = "20"
                LOAD_DURATION = "6h"
                LOAD_PRE_ALLOCATED_VUS = "40"
                LOAD_MAX_VUS = "200"
                LOAD_STRESS_STAGE_DURATION = "5m"
            }
        }
        default {
            throw "unknown load profile: $ProfileName"
        }
    }
}

function Invoke-K6WithEnvironment([string] $Executable, [string[]] $Arguments, [hashtable] $EnvironmentValues, [string] $WorkingDirectory) {
    $startInfo = New-Object System.Diagnostics.ProcessStartInfo
    $startInfo.FileName = $Executable
    $startInfo.Arguments = ($Arguments | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join " "
    $startInfo.WorkingDirectory = $WorkingDirectory
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $false
    $startInfo.RedirectStandardError = $false
    $startInfo.CreateNoWindow = $true
    $startInfo.EnvironmentVariables.Clear()

    foreach ($key in @("SystemRoot", "WINDIR", "TEMP", "TMP", "PATH")) {
        $value = [Environment]::GetEnvironmentVariable($key, "Process")
        if (![string]::IsNullOrWhiteSpace($value)) {
            $startInfo.EnvironmentVariables[$key] = $value
        }
    }
    foreach ($key in $EnvironmentValues.Keys) {
        $startInfo.EnvironmentVariables[$key] = [string] $EnvironmentValues[$key]
    }

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $startInfo
    [void] $process.Start()
    $process.WaitForExit()

    return [pscustomobject]@{
        ExitCode = [int] $process.ExitCode
        Output = ""
    }
}

function Get-PowerShellExecutable {
    try {
        $currentProcess = Get-Process -Id $PID -ErrorAction Stop
        if (![string]::IsNullOrWhiteSpace($currentProcess.Path) -and (Test-Path -LiteralPath $currentProcess.Path -PathType Leaf)) {
            return $currentProcess.Path
        }
    } catch {
    }

    $pwsh = Get-Command pwsh -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $pwsh) {
        return $pwsh.Source
    }

    $windowsPowerShell = Get-Command powershell.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($null -ne $windowsPowerShell) {
        return $windowsPowerShell.Source
    }

    throw "PowerShell executable not found"
}

if (!(Test-Path -LiteralPath $RegistryPath -PathType Leaf)) {
    throw "scenario registry not found: tests/load/scenarios.v1.json"
}

$registry = Get-Content -LiteralPath $RegistryPath -Raw | ConvertFrom-Json
if ([string] $registry.schemaVersion -ne "gongzzang.load.scenarios.v1") {
    throw "scenario registry schemaVersion mismatch"
}

if ([string]::IsNullOrWhiteSpace($TargetBaseUrl)) {
    $TargetBaseUrl = [string] $registry.defaultTargetBaseUrl
}
$targetInfo = Resolve-TargetBaseUrl $TargetBaseUrl
Assert-NonProductionTarget $targetInfo.Host
$TargetBaseUrl = $targetInfo.NormalizedUrl

if ([string]::IsNullOrWhiteSpace($Environment)) {
    $targetHost = $targetInfo.Host
    $Environment = if ($targetHost -eq "localhost" -or $targetHost -eq "127.0.0.1" -or $targetHost -eq "::1") { "local" } else { "perf" }
}

$matchingScenarios = @($registry.scenarios | Where-Object { [string] $_.id -eq $Scenario })
if ($matchingScenarios.Count -ne 1) {
    throw "unknown load scenario: $Scenario"
}
$scenarioSpec = $matchingScenarios[0]
$scenarioScript = [System.IO.Path]::GetFullPath((Join-Path $RepoRoot ([string] $scenarioSpec.file)))
if (!(Test-Path -LiteralPath $scenarioScript -PathType Leaf)) {
    throw "scenario script not found: $($scenarioSpec.file)"
}

if (($Scenario -eq "capacity-stress" -or @("stress", "spike", "soak") -contains $Profile) -and !$AllowStress) {
    throw "AllowStress is required for stress, spike, soak, or capacity-stress runs"
}

if ([string]::IsNullOrWhiteSpace($OutRoot)) {
    $OutRoot = Join-Path $RepoRoot "target\audit\load-tests"
}
$outRootFull = [System.IO.Path]::GetFullPath($OutRoot)

$startedAt = [DateTimeOffset]::Now
$datePart = $startedAt.ToString("yyyy-MM-dd")
$timestamp = $startedAt.ToString("yyyyMMddTHHmmssK").Replace(":", "")
$runDir = Join-Path (Join-Path (Join-Path (Join-Path $outRootFull $datePart) $Environment) $Scenario) $timestamp
New-Item -ItemType Directory -Force -Path $runDir | Out-Null

$gitSha = Get-GitSha
$profileConfig = Get-ProfileConfig $Profile
$slo = $registry.slo

$runPath = Join-Path $runDir "run.json"
$specPath = Join-Path $runDir "spec.json"
$thresholdsPath = Join-Path $runDir "thresholds.json"
$summaryPath = Join-Path $runDir "k6-summary.json"

$runRecord = [ordered]@{
    schemaVersion = "gongzzang.load.run.v1"
    scenario = $Scenario
    profile = $Profile
    environment = $Environment
    targetBaseUrl = $TargetBaseUrl
    startedAt = $startedAt.ToString("o")
    finishedAt = $null
    gitSha = $gitSha
    runDirectory = $runDir
    k6ExitCode = $null
}
ConvertTo-JsonFile $runPath $runRecord

$hostInfo = [ordered]@{
    machineName = [Environment]::MachineName
    userDomainName = [Environment]::UserDomainName
    osVersion = [Environment]::OSVersion.VersionString
    processorCount = [Environment]::ProcessorCount
    powershellVersion = $PSVersionTable.PSVersion.ToString()
}

ConvertTo-JsonFile $specPath ([ordered]@{
    schemaVersion = "gongzzang.load.spec.v1"
    scenario = $Scenario
    profile = $Profile
    environment = $Environment
    targetBaseUrl = $TargetBaseUrl
    gitSha = $gitSha
    scriptPath = [string] $scenarioSpec.file
    maxSafeRps = [int] $scenarioSpec.maxSafeRps
    profileConfig = $profileConfig
    host = $hostInfo
})

ConvertTo-JsonFile $thresholdsPath ([ordered]@{
    schemaVersion = "gongzzang.load.thresholds.v1"
    scenario = $Scenario
    slo = $slo
    maxSafeRps = [int] $scenarioSpec.maxSafeRps
})

$k6ExitCode = 1
$k6OutputPath = Join-Path $runDir "k6-output.log"
$envValues = @{}
$envValues["TARGET_BASE_URL"] = $TargetBaseUrl
$envValues["LOAD_PROFILE"] = $Profile
$envValues["LOAD_ENVIRONMENT"] = $Environment
$envValues["GIT_SHA"] = $gitSha
foreach ($key in $profileConfig.Keys) {
    $envValues[$key] = $profileConfig[$key]
}
if ($AllowStress) {
    $envValues["ALLOW_STRESS"] = "true"
}
foreach ($approvedSecretName in @("PLATFORM_CORE_WEBHOOK_SECRET")) {
    $approvedSecretValue = [Environment]::GetEnvironmentVariable($approvedSecretName, "Process")
    if (![string]::IsNullOrEmpty($approvedSecretValue)) {
        $envValues[$approvedSecretName] = $approvedSecretValue
    }
}

try {
    $k6Command = @(Get-Command k6 -CommandType Application -ErrorAction Stop)[0]
    $k6Result = Invoke-K6WithEnvironment `
        -Executable $k6Command.Source `
        -Arguments @("run", "--summary-export", $summaryPath, $scenarioScript) `
        -EnvironmentValues $envValues `
        -WorkingDirectory $RepoRoot
    $k6ExitCode = $k6Result.ExitCode
    if (![string]::IsNullOrWhiteSpace($k6Result.Output)) {
        $k6Result.Output | Set-Content -LiteralPath $k6OutputPath -Encoding UTF8
    }
} catch {
    $k6ExitCode = 1
    $_ | Out-String | Set-Content -LiteralPath $k6OutputPath -Encoding UTF8
}

$finishedAt = [DateTimeOffset]::Now
$runRecord.finishedAt = $finishedAt.ToString("o")
$runRecord.k6ExitCode = $k6ExitCode
ConvertTo-JsonFile $runPath $runRecord

$powerShellExecutable = Get-PowerShellExecutable
& $powerShellExecutable -NoProfile -ExecutionPolicy Bypass -File $NormalizerPath -RunDirectory $runDir
$normalizerExitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int] $LASTEXITCODE }
if ($normalizerExitCode -ne 0 -and $k6ExitCode -eq 0) {
    exit $normalizerExitCode
}

Write-Output "load-test-evidence=$runDir"
exit $k6ExitCode
