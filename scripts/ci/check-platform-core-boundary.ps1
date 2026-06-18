param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ModuleRoot = Join-Path $PSScriptRoot "platform-core-boundary"

. (Join-Path $ModuleRoot "constants.ps1")
. (Join-Path $ModuleRoot "shared.ps1")
. (Join-Path $ModuleRoot "phase-01-policy-schema.ps1")
. (Join-Path $ModuleRoot "phase-02-ownership-contracts.ps1")
. (Join-Path $ModuleRoot "phase-03-documentation-ci.ps1")
. (Join-Path $ModuleRoot "phase-04-legacy-migration.ps1")
. (Join-Path $ModuleRoot "phase-05-code-env-boundary.ps1")

Write-Host "platform-core-boundary-ok entries=$($entries.Count) contracts=$($contracts.Count) gates=$($gates.Count) legacy_schema_allowances=$($legacySchemaAllowances.Count)"
