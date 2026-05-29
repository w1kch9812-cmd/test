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

function Write-MinimalLoadAssets([string] $Root, [switch] $UnsafeProductionTarget) {
    $target = if ($UnsafeProductionTarget) { "https://gongzzang.com" } else { "https://perf.gongzzang.internal" }
    Write-File $Root "docs\testing\load.md" "# Load Testing`n"
    Write-File $Root "tests\load\README.md" "# Load Scenarios`n"
    Write-File $Root "tests\load\scenarios.v1.json" @"
{
  "schemaVersion": "gongzzang.load.scenarios.v1",
  "defaultTargetBaseUrl": "$target",
  "scenarios": [
    {"id":"api-read-mix","file":"tests/load/scenarios/api-read-mix.js","maxSafeRps":50},
    {"id":"map-marker-mix","file":"tests/load/scenarios/map-marker-mix.js","maxSafeRps":50},
    {"id":"capacity-stress","file":"tests/load/scenarios/capacity-stress.js","maxSafeRps":800},
    {"id":"platform-core-events","file":"tests/load/scenarios/platform-core-events.js","maxSafeRps":50}
  ]
}
"@
    foreach ($file in @(
        "tests\load\lib\env.js",
        "tests\load\lib\http.js",
        "tests\load\scenarios\api-read-mix.js",
        "tests\load\scenarios\map-marker-mix.js",
        "tests\load\scenarios\capacity-stress.js",
        "tests\load\scenarios\platform-core-events.js",
        "scripts\load\run-k6.ps1",
        "scripts\load\normalize-k6-summary.ps1",
        ".github\workflows\load-test-capacity.yml"
    )) {
        Write-File $Root $file "asset"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-MinimalLoadAssets $okRoot
    $ok = Invoke-Checker $okRoot
    if ($ok.ExitCode -ne 0) { throw "expected ok fixture to pass: $($ok.Output)" }

    $unsafeRoot = Join-Path $TempRoot "unsafe"
    Write-MinimalLoadAssets $unsafeRoot -UnsafeProductionTarget
    $unsafe = Invoke-Checker $unsafeRoot
    if ($unsafe.ExitCode -eq 0) { throw "expected production target fixture to fail" }
    if (!$unsafe.Output.Contains("defaultTargetBaseUrl must not be production")) {
        throw "expected production target error, got: $($unsafe.Output)"
    }

    Write-Output "check-load-test-assets-tests-ok"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
