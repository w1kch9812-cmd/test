[CmdletBinding()]
param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)

    [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-TextFile {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "required file is missing: $RelativePath"
    }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$runnerPath = "tools/bazel/run_ci_transition_task.sh"
$tarpaulinConfigPath = "tarpaulin.toml"
$runnerContent = Read-TextFile -RelativePath $runnerPath
$tarpaulinConfig = Read-TextFile -RelativePath $tarpaulinConfigPath

if ($runnerContent -notmatch '(?m)^\s*cargo\s+tarpaulin\b') {
    throw "coverage transition must invoke cargo tarpaulin"
}
if ($runnerContent -match '(?m)^\s*cargo\s+tarpaulin\b[^\r\n]*(^|\s)--fail-under(?:\s|=|$)') {
    throw "coverage transition must not hard-code --fail-under"
}
if ($runnerContent -match '(?m)^\s*cargo\s+tarpaulin\b[^\r\n]*(^|\s)--out(?:\s|=|$)') {
    throw "coverage transition must not hard-code --out"
}
if ($runnerContent -match '(?m)^\s*cargo\s+tarpaulin\b[^\r\n]*(^|\s)--skip-clean(?:\s|=|$)') {
    throw "coverage transition must not hard-code --skip-clean"
}
if ($runnerContent -match '(?m)^\s*cargo\s+tarpaulin\b[^\r\n]*(^|\s)--exclude-files(?:\s|=|$)') {
    throw "coverage transition must not hard-code --exclude-files"
}
if ($tarpaulinConfig -notmatch '(?m)^\s*fail-under\s*=') {
    throw "tarpaulin.toml must declare fail-under"
}
if ($tarpaulinConfig -notmatch '(?m)^\s*out\s*=') {
    throw "tarpaulin.toml must declare out"
}
if ($tarpaulinConfig -notmatch '(?m)^\s*skip-clean\s*=') {
    throw "tarpaulin.toml must declare skip-clean"
}
if ($tarpaulinConfig -notmatch '(?m)^\s*exclude-files\s*=') {
    throw "tarpaulin.toml must declare exclude-files"
}

Write-Host "coverage-transition-ssot-ok"
