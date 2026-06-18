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
if (!(Test-Path -LiteralPath $resolvedRoot)) {
    throw "Root does not exist: $resolvedRoot"
}

$ModuleRoot = Join-Path $PSScriptRoot "pnu-anchor-pbf-marker-contract"
try {
    $contracts = @()
    . (Join-Path $ModuleRoot "contracts-01-docs.ps1")
    . (Join-Path $ModuleRoot "contracts-02-migrations.ps1")
    . (Join-Path $ModuleRoot "contracts-03-domain-db.ps1")
    . (Join-Path $ModuleRoot "contracts-04-api-runtime.ps1")
    . (Join-Path $ModuleRoot "contracts-05-frontend-tests.ps1")
    . (Join-Path $ModuleRoot "phase-01-validate-contracts.ps1")

    Write-Host "pnu-anchor-pbf-marker-contract-ok files=$checkedFiles"
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}
