Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-coverage-transition-ssot.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-coverage-transition-ssot-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Assert-FileLineCountAtMost {
    param([string] $Path, [int] $MaxLines)

    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        return
    }
    $lineCount = (Get-Content -LiteralPath $Path | Measure-Object -Line).Lines
    if ($lineCount -gt $MaxLines) {
        throw "$Path line count $lineCount exceeds $MaxLines"
    }
}

Assert-FileLineCountAtMost -Path $PSCommandPath -MaxLines 600
Assert-FileLineCountAtMost -Path $ScriptPath -MaxLines 600

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $HardcodedFailUnder,
        [switch] $HardcodedOut,
        [switch] $MissingFailUnderConfig
    )

    $failUnderFlag = if ($HardcodedFailUnder) { " --fail-under 90" } else { "" }
    $outFlag = if ($HardcodedOut) { " --out Lcov" } else { "" }
    $failUnderConfig = if ($MissingFailUnderConfig) { "" } else { "fail-under = 90" }
    Write-File -Root $Root -RelativePath "tools\bazel\run_ci_transition_task.sh" -Content @"
run_coverage_tarpaulin() {
  require_command cargo
  require_command cargo-tarpaulin
  cargo tarpaulin --workspace$failUnderFlag$outFlag
}
"@
    Write-File -Root $Root -RelativePath "tarpaulin.toml" -Content @"
[default]
skip-clean = true
out = ["Html", "Lcov"]
$failUnderConfig
exclude-files = ["target/**", "**/tests/**"]
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
            Output   = ($output -join [Environment]::NewLine)
        }
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-Equals $success.ExitCode 0 "success exit code mismatch"
    Assert-Contains $success.Output "coverage-transition-ssot-ok"

    $hardcodedFailUnderRoot = Join-Path $TempRoot "hardcoded-fail-under"
    Write-MinimalRepo -Root $hardcodedFailUnderRoot -HardcodedFailUnder
    $hardcodedFailUnder = Invoke-Checker -Root $hardcodedFailUnderRoot
    Assert-Equals $hardcodedFailUnder.ExitCode 1 "hardcoded fail-under exit code mismatch"
    Assert-Contains $hardcodedFailUnder.Output "coverage transition must not hard-code --fail-under"

    $hardcodedOutRoot = Join-Path $TempRoot "hardcoded-out"
    Write-MinimalRepo -Root $hardcodedOutRoot -HardcodedOut
    $hardcodedOut = Invoke-Checker -Root $hardcodedOutRoot
    Assert-Equals $hardcodedOut.ExitCode 1 "hardcoded out exit code mismatch"
    Assert-Contains $hardcodedOut.Output "coverage transition must not hard-code --out"

    $missingFailUnderRoot = Join-Path $TempRoot "missing-fail-under-config"
    Write-MinimalRepo -Root $missingFailUnderRoot -MissingFailUnderConfig
    $missingFailUnder = Invoke-Checker -Root $missingFailUnderRoot
    Assert-Equals $missingFailUnder.ExitCode 1 "missing fail-under config exit code mismatch"
    Assert-Contains $missingFailUnder.Output "tarpaulin.toml must declare fail-under"

    Write-Host "coverage-transition-ssot-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
