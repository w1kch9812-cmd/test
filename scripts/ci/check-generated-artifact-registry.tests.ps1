Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-generated-artifact-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-generated-artifact-registry-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Assert-FileLineCountAtMost {
    param(
        [string] $Path,
        [int] $MaxLines
    )

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

function Write-Lines {
    param([string] $Root, [string] $RelativePath, [int] $Count)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    $content = 1..$Count | ForEach-Object { "{`"line`": $_}" }
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $content
}

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitRegistry,
        [switch] $OmitGenerator,
        [switch] $OversizedSource,
        [switch] $UnregisteredLargeGeneratedJson
    )

    Write-Lines -Root $Root -RelativePath "docs\architecture\traffic-auth-policy-registry.v1.json" -Count 550
    Write-Lines -Root $Root -RelativePath "docs\architecture\traffic-auth-policy-registry\20-public-route-policies.json" -Count ($(if ($OversizedSource) { 601 } else { 20 }))
    Write-File -Root $Root -RelativePath "scripts\ci\check-traffic-auth-policy-registry.ps1" -Content "Write-Host ok"
    if (!$OmitGenerator) {
        Write-File -Root $Root -RelativePath "scripts\ci\generate-traffic-auth-policy-registry.ps1" -Content "Write-Host ok"
    }
    if ($UnregisteredLargeGeneratedJson) {
        Write-Lines -Root $Root -RelativePath "infrastructure\security\unregistered.generated.json" -Count 550
    }
    if (!$OmitRegistry) {
        Write-File -Root $Root -RelativePath "docs\architecture\generated-artifacts.v1.json" -Content @'
{
  "schema_version": "gongzzang.generated_artifacts.v1",
  "repo_slug": "gongzzang",
  "artifacts": [
    {
      "path": "docs/architecture/traffic-auth-policy-registry.v1.json",
      "kind": "compatibility_aggregate",
      "owner": "build-platform",
      "reason": "Compatibility aggregate generated from traffic/auth policy fragments.",
      "generator": "scripts/ci/generate-traffic-auth-policy-registry.ps1",
      "verifier": "scripts/ci/check-traffic-auth-policy-registry.ps1",
      "source_paths": ["docs/architecture/traffic-auth-policy-registry"],
      "max_artifact_lines": 1200,
      "max_source_lines": 600
    }
  ]
}
'@
    }
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

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    $actualCompact = ($Text -replace "\s+", "")
    $expectedCompact = ($Expected -replace "\s+", "")
    if (!$Text.Contains($Expected) -and !$actualCompact.Contains($expectedCompact)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Assert-ExitCode {
    param([object] $Result, [int] $Expected, [string] $Message)

    if ($Result.ExitCode -ne $Expected) {
        throw "$Message expected='$Expected' actual='$($Result.ExitCode)' output=$($Result.Output)"
    }
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null
try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-ExitCode -Result $success -Expected 0 -Message "success exit code mismatch"
    Assert-Contains -Text $success.Output -Expected "generated-artifact-registry-ok"

    $missingRegistryRoot = Join-Path $TempRoot "missing-registry"
    Write-MinimalRepo -Root $missingRegistryRoot -OmitRegistry
    $missingRegistry = Invoke-Checker -Root $missingRegistryRoot
    Assert-ExitCode -Result $missingRegistry -Expected 1 -Message "missing registry exit code mismatch"
    Assert-Contains -Text $missingRegistry.Output -Expected "generated artifact registry is missing"

    $missingGeneratorRoot = Join-Path $TempRoot "missing-generator"
    Write-MinimalRepo -Root $missingGeneratorRoot -OmitGenerator
    $missingGenerator = Invoke-Checker -Root $missingGeneratorRoot
    Assert-ExitCode -Result $missingGenerator -Expected 1 -Message "missing generator exit code mismatch"
    Assert-Contains -Text $missingGenerator.Output -Expected "generator is missing"

    $oversizedSourceRoot = Join-Path $TempRoot "oversized-source"
    Write-MinimalRepo -Root $oversizedSourceRoot -OversizedSource
    $oversizedSource = Invoke-Checker -Root $oversizedSourceRoot
    Assert-ExitCode -Result $oversizedSource -Expected 1 -Message "oversized source exit code mismatch"
    Assert-Contains -Text $oversizedSource.Output -Expected "source file exceeds max_source_lines"

    $unregisteredLargeJsonRoot = Join-Path $TempRoot "unregistered-large-json"
    Write-MinimalRepo -Root $unregisteredLargeJsonRoot -UnregisteredLargeGeneratedJson
    $unregisteredLargeJson = Invoke-Checker -Root $unregisteredLargeJsonRoot
    Assert-ExitCode -Result $unregisteredLargeJson -Expected 1 -Message "unregistered large generated JSON exit code mismatch"
    Assert-Contains -Text $unregisteredLargeJson.Output -Expected "large generated JSON artifact must be registered"

    Write-Host "generated-artifact-registry-tests-ok"
    exit 0
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
