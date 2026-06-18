param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ModuleRoot = Join-Path $PSScriptRoot "transition-ratchet-bazel"
. (Join-Path $ModuleRoot "shared.ps1")

. (Join-Path $ModuleRoot "phase-01-policy-registry.ps1")
. (Join-Path $ModuleRoot "phase-02-registry-indexes.ps1")
. (Join-Path $ModuleRoot "phase-03-policy-targets.ps1")
. (Join-Path $ModuleRoot "phase-04-bazel-targets.ps1")
. (Join-Path $ModuleRoot "phase-05-runner-and-ci-wiring.ps1")

Write-Host "bazel-transition-ratchet-ok targets=$($actualTargets.Count) ci_refs=$($ciReferences.Count)"
