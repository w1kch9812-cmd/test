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
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)
    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-RequiredText {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required approval gate file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$approvalRequestPath = "docs/superpowers/handoff/2026-05-29-platform-core-anchor-inbox-db-schema-approval-request.md"
$migrationPath = "migrations/30016_platform_core_event_inbox_anchor_import.sql"
$approvalText = Read-RequiredText -RelativePath $approvalRequestPath

foreach ($requiredToken in @(
    "This is an approval request only",
    "Reserved migration:",
    $migrationPath,
    "Required user approval wording:",
    "30016 Platform Core anchor inbox/import DB schema migration creation is approved."
)) {
    if (!$approvalText.Contains($requiredToken)) {
        throw "30016 anchor inbox approval request missing required token: $requiredToken"
    }
}

$migrationFullPath = Resolve-RepoPath -RelativePath $migrationPath
if (!(Test-Path -LiteralPath $migrationFullPath -PathType Leaf)) {
    Write-Host "platform-core-anchor-inbox-db-approval-ok migration=absent"
    exit 0
}

$requiredApprovalRecordTokens = @(
    "## Approval record",
    "Approval status: approved",
    "Approved migration: ``$migrationPath``",
    "Approved statement:",
    '`30016 Platform Core anchor inbox/import DB schema migration creation is approved.`'
)
foreach ($token in $requiredApprovalRecordTokens) {
    if (!$approvalText.Contains($token)) {
        throw "30016 anchor inbox migration requires explicit approval record before creating $migrationPath"
    }
}

Write-Host "platform-core-anchor-inbox-db-approval-ok migration=approved"
