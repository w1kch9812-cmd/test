function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe -NoProfile -ExecutionPolicy Bypass -File $ScriptPath -Root $Root 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)

    if (!$Text.Contains($Expected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

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
    Set-Content -LiteralPath $path -Value $Content -Encoding UTF8
}

function Write-ContractFiles {
    param([string] $Root)

    $FixtureRoot = Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract.tests"
    . (Join-Path $FixtureRoot "fixture-docs-contracts.ps1")
    . (Join-Path $FixtureRoot "fixture-migrations.ps1")
    . (Join-Path $FixtureRoot "fixture-domain-db.ps1")
    . (Join-Path $FixtureRoot "fixture-api-anchor-import.ps1")
    . (Join-Path $FixtureRoot "fixture-frontend-map.ps1")
    . (Join-Path $FixtureRoot "fixture-migration-smoke-doc.ps1")
}
