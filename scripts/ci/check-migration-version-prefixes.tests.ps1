Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-migration-version-prefixes.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-migration-version-prefixes-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
        [pscustomobject]@{
            ExitCode = $LASTEXITCODE
            Output = ($output -join [Environment]::NewLine)
        }
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    $actualCompact = $Text -replace "\s+", ""
    $expectedCompact = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$actualCompact.Contains($expectedCompact)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-File $okRoot "migrations\30015_drop_platform_core_legacy_schema.sql" "-- ok"
    Write-File $okRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql" "-- ok"
    $ok = Invoke-Checker $okRoot
    if ($ok.ExitCode -ne 0) {
        throw "expected unique migration fixture to pass: $($ok.Output)"
    }
    Assert-Contains $ok.Output "migration-version-prefixes-ok files=2"

    $duplicateRoot = Join-Path $TempRoot "duplicate"
    Write-File $duplicateRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql" "-- one"
    Write-File $duplicateRoot "migrations\30016_another_change.sql" "-- two"
    $duplicate = Invoke-Checker $duplicateRoot
    if ($duplicate.ExitCode -eq 0) {
        throw "expected duplicate migration fixture to fail"
    }
    Assert-Contains $duplicate.Output "duplicate migration version prefix '30016'"

    $badNameRoot = Join-Path $TempRoot "bad-name"
    Write-File $badNameRoot "migrations\not_a_version.sql" "-- bad"
    $badName = Invoke-Checker $badNameRoot
    if ($badName.ExitCode -eq 0) {
        throw "expected malformed migration fixture to fail"
    }
    Assert-Contains $badName.Output "migration filename must start with a five digit version prefix"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force
}

Write-Host "check-migration-version-prefixes-tests-ok"
