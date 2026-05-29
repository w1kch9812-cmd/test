[CmdletBinding()]
param(
    [string] $Root = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $scriptRoot = $PSScriptRoot
    if ([string]::IsNullOrWhiteSpace($scriptRoot)) {
        $scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
    }
    $Root = Join-Path $scriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
$migrationRoot = Join-Path $resolvedRoot "migrations"
if (!(Test-Path -LiteralPath $migrationRoot -PathType Container)) {
    throw "migration directory is missing: migrations"
}

$versions = @{}
$files = @(Get-ChildItem -LiteralPath $migrationRoot -File -Filter "*.sql" | Sort-Object -Property Name)
foreach ($file in $files) {
    if ($file.Name -notmatch "^(?<version>\d{5})_[a-z0-9_]+\.sql$") {
        throw "migration filename must start with a five digit version prefix: migrations/$($file.Name)"
    }

    $version = $Matches["version"]
    if (!$versions.ContainsKey($version)) {
        $versions[$version] = New-Object System.Collections.Generic.List[string]
    }
    $versions[$version].Add("migrations/$($file.Name)")
}

foreach ($version in @($versions.Keys | Sort-Object)) {
    $paths = @($versions[$version])
    if ($paths.Count -gt 1) {
        throw "duplicate migration version prefix '$version': $($paths -join ', ')"
    }
}

Write-Host "migration-version-prefixes-ok files=$($files.Count)"
