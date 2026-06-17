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
    param([string] $Root, [string] $AllowedCommands = "", [switch] $IncludeBootstrapAllowlist)

    $allowedEntries = @()
    $bootstrapAllowlist = if ($IncludeBootstrapAllowlist) {
        @"
    {
      "id": "bootstrap-pnpm-install",
      "pattern": "\\bpnpm\\s+install\\s+--frozen-lockfile\\b",
      "scope": ".github/workflows/*.yml",
      "owner": "build-platform",
      "reason": "Dependency bootstrap, not verification semantics.",
      "exit_target": "Keep until Bazel toolchain bootstrap covers package manager fetch.",
      "sunset": "2026-07-31"
    }
"@
    } else {
        ""
    }
    if (![string]::IsNullOrWhiteSpace($bootstrapAllowlist)) {
        $allowedEntries += $bootstrapAllowlist
    }
    if (![string]::IsNullOrWhiteSpace($AllowedCommands)) {
        $allowedEntries += $AllowedCommands
    }
    $allowedBlock = $allowedEntries -join ","

    Write-File $Root "docs\architecture\verification-control-plane.v1.json" @"
{
  "schema_version": 1,
  "forbidden_direct_verification_commands": [
    { "id": "pnpm-test", "pattern": "\\bpnpm\\s+test\\b" },
    { "id": "pnpm-build", "pattern": "\\bpnpm(?:\\s+--filter(?:=|\\s+)\\S+)?\\s+build\\b" },
    { "id": "pnpm-typecheck", "pattern": "\\bpnpm\\s+typecheck\\b" },
    { "id": "pnpm-lint", "pattern": "\\bpnpm\\s+lint\\b" },
    { "id": "cargo-clippy", "pattern": "\\bcargo\\s+clippy\\b" },
    { "id": "cargo-test", "pattern": "\\bcargo\\s+test\\b" },
    { "id": "cargo-build", "pattern": "\\bcargo\\s+build\\b" },
    { "id": "cargo-sqlx-prepare", "pattern": "\\bcargo\\s+sqlx\\s+prepare\\b" },
    { "id": "cargo-tarpaulin", "pattern": "\\bcargo\\s+tarpaulin\\b" },
    { "id": "sqlx-migrate", "pattern": "\\bsqlx\\s+migrate\\s+run\\b" },
    { "id": "migration-smoke-script", "pattern": "\\bbash\\s+tests/migrations/test_[^\\s]+\\.sh\\b" }
  ],
  "allowed_direct_commands": [
    $allowedBlock
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

    $directTarpaulinRoot = Join-Path $TempRoot "direct-tarpaulin-fails"
    Write-Policy -Root $directTarpaulinRoot
    Write-File $directTarpaulinRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: cargo tarpaulin --workspace --out Lcov
"@
    Assert-Failure -Result (Invoke-Checker -Root $directTarpaulinRoot) -ExpectedText "cargo tarpaulin"

    $directCargoBuildRoot = Join-Path $TempRoot "direct-cargo-build-fails"
    Write-Policy -Root $directCargoBuildRoot
    Write-File $directCargoBuildRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build -p api --release --locked
"@
    Assert-Failure -Result (Invoke-Checker -Root $directCargoBuildRoot) -ExpectedText "cargo build"

    $directSqlxPrepareRoot = Join-Path $TempRoot "direct-sqlx-prepare-fails"
    Write-Policy -Root $directSqlxPrepareRoot
    Write-File $directSqlxPrepareRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: cargo sqlx prepare --workspace --check
"@
    Assert-Failure -Result (Invoke-Checker -Root $directSqlxPrepareRoot) -ExpectedText "cargo sqlx prepare"

    $directSqlxMigrateRoot = Join-Path $TempRoot "direct-sqlx-migrate-fails"
    Write-Policy -Root $directSqlxMigrateRoot
    Write-File $directSqlxMigrateRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: sqlx migrate run --source migrations
"@
    Assert-Failure -Result (Invoke-Checker -Root $directSqlxMigrateRoot) -ExpectedText "sqlx migrate run"

    $directMigrationSmokeRoot = Join-Path $TempRoot "direct-migration-smoke-fails"
    Write-Policy -Root $directMigrationSmokeRoot
    Write-File $directMigrationSmokeRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: bash tests/migrations/test_v001_full.sh
"@
    Assert-Failure -Result (Invoke-Checker -Root $directMigrationSmokeRoot) -ExpectedText "bash tests/migrations/test_v001_full.sh"

    $allowRoot = Join-Path $TempRoot "allow-bootstrap"
    Write-Policy -Root $allowRoot -IncludeBootstrapAllowlist
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

    $unusedAllowlistRoot = Join-Path $TempRoot "unused-allowlist-fails"
    Write-Policy -Root $unusedAllowlistRoot -AllowedCommands @'
    {
      "id": "transition-unused-cargo-clippy",
      "pattern": "\\bcargo\\s+clippy\\b",
      "scope": "lefthook.yml",
      "owner": "build-platform",
      "reason": "Transition until Rust clippy is represented as a Bazel target.",
      "exit_target": "//:verify_pr",
      "sunset": "2026-07-31"
    }
'@
    Write-File $unusedAllowlistRoot ".github\workflows\ci.yml" @"
name: CI
on: [push]
jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - run: bazelisk test //:verify_pr --config=ci --verbose_failures
"@
    Assert-Failure -Result (Invoke-Checker -Root $unusedAllowlistRoot) -ExpectedText "unused allowlist entry"

    Write-Host "verification-control-plane-tests-ok"
    exit 0
} finally {
    Remove-Item -LiteralPath $TempRoot -Recurse -Force -ErrorAction SilentlyContinue
}
