Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-platform-core-anchor-inbox-db-approval.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-platform-core-anchor-inbox-db-approval-tests") `
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

function Write-ApprovalRequest {
    param([string] $Root, [switch] $Approved)

    $approvalRecord = if ($Approved) {
        @'

## Approval record

Approval status: approved

Approved migration: `migrations/30016_platform_core_event_inbox_anchor_import.sql`

Approved statement:
`30016 Platform Core anchor inbox/import DB schema migration creation is approved.`
'@
    } else {
        ""
    }

    Write-File $Root "docs\superpowers\handoff\2026-05-29-platform-core-anchor-inbox-db-schema-approval-request.md" @"
# Platform Core anchor inbox DB schema approval request

This is an approval request only. Do not create or apply the migration until the
user explicitly approves this DB schema change.

Reserved migration:
`migrations/30016_platform_core_event_inbox_anchor_import.sql`

Required user approval wording:
`30016 Platform Core anchor inbox/import DB schema migration creation is approved.`
$approvalRecord
"@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $absentRoot = Join-Path $TempRoot "absent"
    Write-ApprovalRequest $absentRoot
    $absent = Invoke-Checker $absentRoot
    if ($absent.ExitCode -ne 0) {
        throw "expected absent migration fixture to pass: $($absent.Output)"
    }
    Assert-Contains $absent.Output "platform-core-anchor-inbox-db-approval-ok migration=absent"

    $unapprovedRoot = Join-Path $TempRoot "unapproved"
    Write-ApprovalRequest $unapprovedRoot
    Write-File $unapprovedRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql" "-- unapproved"
    $unapproved = Invoke-Checker $unapprovedRoot
    if ($unapproved.ExitCode -eq 0) {
        throw "expected unapproved migration fixture to fail"
    }
    Assert-Contains $unapproved.Output "30016 anchor inbox migration requires explicit approval record"

    $approvedRoot = Join-Path $TempRoot "approved"
    Write-ApprovalRequest $approvedRoot -Approved
    Write-File $approvedRoot "migrations\30016_platform_core_event_inbox_anchor_import.sql" "-- approved"
    $approved = Invoke-Checker $approvedRoot
    if ($approved.ExitCode -ne 0) {
        throw "expected approved migration fixture to pass: $($approved.Output)"
    }
    Assert-Contains $approved.Output "platform-core-anchor-inbox-db-approval-ok migration=approved"
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force
}

Write-Host "check-platform-core-anchor-inbox-db-approval-tests-ok"
