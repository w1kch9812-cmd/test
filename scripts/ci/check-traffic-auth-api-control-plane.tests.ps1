Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-traffic-auth-policy-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }
$FixturePath = Join-Path $RepoRoot "apps\web\lib\api\raw-api-transport-fixture.ts"

function Invoke-Checker {
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $output = & $PowerShellExe `
        -NoProfile `
        -ExecutionPolicy Bypass `
        -File $ScriptPath `
        -Root $RepoRoot `
        2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
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

try {
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $FixturePath) | Out-Null
    Set-Content -LiteralPath $FixturePath -Encoding UTF8 -Value @'
import { api } from "@/lib/api";

export async function rawApiTransportFixture(): Promise<unknown> {
  return api.get("listings").json<unknown>();
}
'@

    $result = Invoke-Checker
    Assert-Equals $result.ExitCode 1 "direct API transport fixture exit code mismatch"
    Assert-Contains $result.Output "direct API transport usage"
    Assert-Contains $result.Output "apps/web/lib/api/raw-api-transport-fixture.ts"

    Write-Host "traffic-auth-api-control-plane-tests-ok"
    exit 0
} finally {
    if (Test-Path -LiteralPath $FixturePath -PathType Leaf) {
        Remove-Item -LiteralPath $FixturePath -Force
    }
}
