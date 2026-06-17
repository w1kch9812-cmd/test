Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-bazel-transition-ratchet.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-bazel-transition-ratchet-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)

    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Checker {
    param([string] $Root)

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $ScriptPath, "-Root", $Root)
    $output = & $PowerShellExe @arguments 2>&1
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

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitNodeAuditPolicy,
        [switch] $AddStalePolicy,
        [switch] $ExpiredSunset,
        [switch] $MissingExternalCollectionFlag,
        [switch] $RetiredRustfmtTransition,
        [switch] $UntrackedCiTransition
    )

    Write-File -Root $Root -RelativePath "tools\bazel\BUILD.bazel" -Content @'
load("//tools/bazel:shell_test_compat.bzl", "transition_shell_test")

transition_shell_test(
    name = "ci_node_audit_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["node-audit"],
)

transition_shell_test(
    name = "ci_rust_check_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["rust-check"],
)

transition_shell_test(
    name = "ci_rustfmt_transition",
    srcs = ["run_ci_transition_task.sh"],
    script_args = ["rustfmt-check"],
)
'@

    $sunset = if ($ExpiredSunset) { "2020-01-01" } else { "2026-07-31" }
    $nodeAuditPolicy = if ($OmitNodeAuditPolicy) {
        ""
    } else {
        $externalCollection = if ($MissingExternalCollectionFlag) { "false" } else { "true" }
        @"
    {
      "bazel_target": "//tools/bazel:ci_node_audit_transition",
      "category": "external-advisory-sca",
      "owner": "build-platform",
      "reason": "pnpm audit still shells out until advisory SCA is represented by a pinned Bazel evidence target.",
      "exit_target": "//:dependency_sca_evidence",
      "sunset": "$sunset",
      "external_collection_approval_required": $externalCollection
    },
"@
    }
    $stalePolicy = if ($AddStalePolicy) {
        @'
    {
      "bazel_target": "//tools/bazel:deleted_transition",
      "category": "stale-fixture",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//:deleted",
      "sunset": "2026-07-31",
      "external_collection_approval_required": false
    },
'@
    } else {
        ""
    }
    $retiredTargets = if ($RetiredRustfmtTransition) {
        @'
  "retired_transition_targets": [
    "//tools/bazel:ci_rustfmt_transition"
  ],
'@
    } else {
        @'
  "retired_transition_targets": [],
'@
    }
    Write-File -Root $Root -RelativePath "docs\architecture\verification-transition-ratchet.v1.json" -Content @"
{
  "schema_version": "gongzzang.verification_transition_ratchet.v1",
  "repo_slug": "gongzzang",
  "default_decision": "deny_new_transition_without_policy",
$retiredTargets
  "transition_targets": [
$nodeAuditPolicy$stalePolicy    {
      "bazel_target": "//tools/bazel:ci_rust_check_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "cargo check transition until Rust check is a native Bazel rule target.",
      "exit_target": "//:rust_verification",
      "sunset": "$sunset",
      "external_collection_approval_required": false
    },
    {
      "bazel_target": "//tools/bazel:ci_rustfmt_transition",
      "category": "rust-verification",
      "owner": "build-platform",
      "reason": "fixture",
      "exit_target": "//tools/bazel:rustfmt_check",
      "sunset": "$sunset",
      "external_collection_approval_required": false
    }
  ]
}
"@

    $extraCi = if ($UntrackedCiTransition) {
        "      - run: bazelisk test //tools/bazel:unknown_transition --config=ci"
    } else {
        ""
    }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
jobs:
  verify:
    steps:
      - run: bazelisk test //tools/bazel:ci_node_audit_transition --config=ci
      - run: bazelisk test //tools/bazel:ci_rust_check_transition --config=ci
$extraCi
"@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $success = Invoke-Checker -Root $successRoot
    Assert-Equals $success.ExitCode 0 "success exit code mismatch"
    Assert-Contains $success.Output "bazel-transition-ratchet-ok"

    $missingPolicyRoot = Join-Path $TempRoot "missing-policy"
    Write-MinimalRepo -Root $missingPolicyRoot -OmitNodeAuditPolicy
    $missingPolicy = Invoke-Checker -Root $missingPolicyRoot
    Assert-Equals $missingPolicy.ExitCode 1 "missing policy exit code mismatch"
    Assert-Contains $missingPolicy.Output "missing transition policy"

    $stalePolicyRoot = Join-Path $TempRoot "stale-policy"
    Write-MinimalRepo -Root $stalePolicyRoot -AddStalePolicy
    $stalePolicy = Invoke-Checker -Root $stalePolicyRoot
    Assert-Equals $stalePolicy.ExitCode 1 "stale policy exit code mismatch"
    Assert-Contains $stalePolicy.Output "stale transition policy"

    $expiredSunsetRoot = Join-Path $TempRoot "expired-sunset"
    Write-MinimalRepo -Root $expiredSunsetRoot -ExpiredSunset
    $expiredSunset = Invoke-Checker -Root $expiredSunsetRoot
    Assert-Equals $expiredSunset.ExitCode 1 "expired sunset exit code mismatch"
    Assert-Contains $expiredSunset.Output "expired transition sunset"

    $missingExternalFlagRoot = Join-Path $TempRoot "missing-external-collection-flag"
    Write-MinimalRepo -Root $missingExternalFlagRoot -MissingExternalCollectionFlag
    $missingExternalFlag = Invoke-Checker -Root $missingExternalFlagRoot
    Assert-Equals $missingExternalFlag.ExitCode 1 "missing external collection flag exit code mismatch"
    Assert-Contains $missingExternalFlag.Output "external advisory collection transition must require approval"

    $retiredRustfmtRoot = Join-Path $TempRoot "retired-rustfmt"
    Write-MinimalRepo -Root $retiredRustfmtRoot -RetiredRustfmtTransition
    $retiredRustfmt = Invoke-Checker -Root $retiredRustfmtRoot
    Assert-Equals $retiredRustfmt.ExitCode 1 "retired rustfmt transition exit code mismatch"
    Assert-Contains $retiredRustfmt.Output "retired transition target still exists"

    $untrackedCiRoot = Join-Path $TempRoot "untracked-ci"
    Write-MinimalRepo -Root $untrackedCiRoot -UntrackedCiTransition
    $untrackedCi = Invoke-Checker -Root $untrackedCiRoot
    Assert-Equals $untrackedCi.ExitCode 1 "untracked CI transition exit code mismatch"
    Assert-Contains $untrackedCi.Output "CI references transition target without policy"

    Write-Host "bazel-transition-ratchet-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
