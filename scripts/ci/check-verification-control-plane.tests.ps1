Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-verification-control-plane.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-verification-control-plane-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function Write-Policy {
    param([string] $Root, [string] $AllowedCommands = "")

    $allowedBlock = if ([string]::IsNullOrWhiteSpace($AllowedCommands)) {
        ""
    } else {
        "," + $AllowedCommands
    }

    Write-File $Root "docs\architecture\verification-control-plane.v1.json" @"
{
  "schema_version": 1,
  "forbidden_direct_verification_commands": [
    { "id": "pnpm-test", "pattern": "\\bpnpm\\s+test\\b" },
    { "id": "pnpm-build", "pattern": "\\bpnpm(?:\\s+--filter(?:=|\\s+)\\S+)?\\s+build\\b" },
    { "id": "pnpm-typecheck", "pattern": "\\bpnpm\\s+typecheck\\b" },
    { "id": "pnpm-lint", "pattern": "\\bpnpm\\s+lint\\b" },
    { "id": "cargo-clippy", "pattern": "\\bcargo\\s+clippy\\b" }
  ],
  "allowed_direct_commands": [
    {
      "id": "bootstrap-pnpm-install",
      "pattern": "\\bpnpm\\s+install\\s+--frozen-lockfile\\b",
      "scope": ".github/workflows/*.yml",
      "owner": "build-platform",
      "reason": "Dependency bootstrap, not verification semantics.",
      "exit_target": "Keep until Bazel toolchain bootstrap covers package manager fetch.",
      "sunset": "2026-07-31"
    }$allowedBlock
  ]
}
"@
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

    $actualCompact = ($Text -replace "\s+", "")
    $expectedCompact = ($Expected -replace "\s+", "")
    if (!$Text.Contains($Expected) -and !$actualCompact.Contains($expectedCompact)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Assert-Success {
    param([object] $Result, [string] $ExpectedText)

    if ($Result.ExitCode -ne 0) {
        throw "expected success, exit=$($Result.ExitCode), output=$($Result.Output)"
    }
    Assert-Contains -Text $Result.Output -Expected $ExpectedText
}

function Assert-Failure {
    param([object] $Result, [string] $ExpectedText)

    if ($Result.ExitCode -eq 0) {
        throw "expected failure containing '$ExpectedText'"
    }
    Assert-Contains -Text $Result.Output -Expected $ExpectedText
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $bazelRoot = Join-Path $TempRoot "bazel-ok"
    Write-Policy -Root $bazelRoot
    Write-File $bazelRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: bazelisk test //:verify_pr --config=ci --verbose_failures
"@
    Assert-Success -Result (Invoke-Checker -Root $bazelRoot) -ExpectedText "verification-control-plane-ok"

    $pnpmRoot = Join-Path $TempRoot "pnpm-test-fails"
    Write-Policy -Root $pnpmRoot
    Write-File $pnpmRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: pnpm test
"@
    Assert-Failure -Result (Invoke-Checker -Root $pnpmRoot) -ExpectedText "forbidden direct verification command"

    $cargoRoot = Join-Path $TempRoot "cargo-clippy-fails"
    Write-Policy -Root $cargoRoot
    Write-File $cargoRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: cargo clippy --workspace --all-features
"@
    Assert-Failure -Result (Invoke-Checker -Root $cargoRoot) -ExpectedText "cargo clippy"

    $allowRoot = Join-Path $TempRoot "allow-bootstrap"
    Write-Policy -Root $allowRoot
    Write-File $allowRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: pnpm install --frozen-lockfile
"@
    Assert-Success -Result (Invoke-Checker -Root $allowRoot) -ExpectedText "allowlisted=1"

    $allowTransitionRoot = Join-Path $TempRoot "allow-transition"
    Write-Policy -Root $allowTransitionRoot -AllowedCommands @'
    {
      "id": "transition-cargo-clippy",
      "pattern": "\\bcargo\\s+clippy\\b",
      "scope": "lefthook.yml",
      "owner": "build-platform",
      "reason": "Transition until Rust clippy is represented as a Bazel target.",
      "exit_target": "//:verify_pr",
      "sunset": "2026-07-31"
    }
'@
    Write-File $allowTransitionRoot "lefthook.yml" @"
pre-push:
  commands:
    cargo-clippy:
      run: cargo clippy --workspace --all-features --all-targets -- -D warnings
"@
    Assert-Success -Result (Invoke-Checker -Root $allowTransitionRoot) -ExpectedText "allowlisted=1"

    Write-Host "verification-control-plane-tests-ok"
    exit 0
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
