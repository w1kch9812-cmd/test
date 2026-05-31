Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-github-actions-node-runtime.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-github-actions-node-runtime-tests") `
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

    $ansiPattern = "$([regex]::Escape([string] [char] 27))\[[0-9;?]*[ -/]*[@-~]"
    $actualPlain = $Text -replace $ansiPattern, ""
    $actualCompact = $actualPlain -replace "\s+", ""
    $expectedCompact = $Expected -replace "\s+", ""
    if (!$actualPlain.Contains($Expected) -and !$actualCompact.Contains($expectedCompact)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Write-MinimalWorkflowRepo {
    param(
        [string] $Root,
        [switch] $MissingNode24OptIn,
        [switch] $AllowsUnsecureNodeVersion,
        [switch] $MissingCiGate
    )

    $nodeRuntimeEnv = if ($MissingNode24OptIn) {
        ""
    } else {
        "  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: `"true`"`n"
    }
    $unsecureOptOut = if ($AllowsUnsecureNodeVersion) {
        "  ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION: `"true`"`n"
    } else {
        ""
    }
    $ciGate = if ($MissingCiGate) {
        ""
    } else {
        "      - run: ./scripts/ci/check-github-actions-node-runtime.ps1`n      - run: ./scripts/ci/check-github-actions-node-runtime.tests.ps1`n"
    }

    Write-File $Root ".github\workflows\ci.yml" @"
name: CI

on:
  push:
    branches: [main]

env:
$nodeRuntimeEnv$unsecureOptOut
jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683
$ciGate
"@

    Write-File $Root ".github\workflows\frontend.yml" @"
name: Frontend

on:
  pull_request:
    branches: [main]

env:
$nodeRuntimeEnv$unsecureOptOut
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/setup-node@39370e3970a6d050c480ffad4ff0ed4d3fdee5af
        with:
          node-version: "20.19.0"
"@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $okRoot = Join-Path $TempRoot "ok"
    Write-MinimalWorkflowRepo -Root $okRoot
    $ok = Invoke-Checker -Root $okRoot
    if ($ok.ExitCode -ne 0) {
        throw "expected Node 24 workflow fixture to pass: $($ok.Output)"
    }
    Assert-Contains $ok.Output "github-actions-node-runtime-ok workflows=2"

    $missingNode24Root = Join-Path $TempRoot "missing-node24"
    Write-MinimalWorkflowRepo -Root $missingNode24Root -MissingNode24OptIn
    $missingNode24 = Invoke-Checker -Root $missingNode24Root
    if ($missingNode24.ExitCode -eq 0) {
        throw "expected missing Node 24 opt-in fixture to fail"
    }
    Assert-Contains $missingNode24.Output "missing FORCE_JAVASCRIPT_ACTIONS_TO_NODE24"

    $unsecureRoot = Join-Path $TempRoot "unsecure-optout"
    Write-MinimalWorkflowRepo -Root $unsecureRoot -AllowsUnsecureNodeVersion
    $unsecure = Invoke-Checker -Root $unsecureRoot
    if ($unsecure.ExitCode -eq 0) {
        throw "expected unsecure Node runtime opt-out fixture to fail"
    }
    Assert-Contains $unsecure.Output "ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION"

    $missingCiGateRoot = Join-Path $TempRoot "missing-ci-gate"
    Write-MinimalWorkflowRepo -Root $missingCiGateRoot -MissingCiGate
    $missingCiGate = Invoke-Checker -Root $missingCiGateRoot
    if ($missingCiGate.ExitCode -eq 0) {
        throw "expected missing CI gate fixture to fail"
    }
    Assert-Contains $missingCiGate.Output "CI workflow must run check-github-actions-node-runtime.ps1"

    Write-Host "github-actions-node-runtime-tests-ok"
    exit 0
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
