Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-verification-task-registry.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-verification-task-registry-tests") `
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
        [switch] $MissingRegistry,
        [switch] $MissingScript,
        [switch] $MissingBazelTarget,
        [switch] $MissingRootSuiteLabel,
        [switch] $MissingRunGuardrailCase,
        [switch] $MissingLefthookPrePush,
        [switch] $MissingCiStep,
        [switch] $ExtraBazelTarget,
        [switch] $ExtraRootSuiteLabel,
        [switch] $ExtraRunGuardrailCase
    )

    if (!$MissingRegistry) {
        Write-File -Root $Root -RelativePath "docs\architecture\verification-task-registry.v1.json" -Content @'
{
  "schema_version": "gongzzang.verification_task_registry.v1",
  "repo_slug": "gongzzang",
  "tasks": [
    {
      "id": "alpha-guardrail",
      "owner": "build-platform",
      "reason": "fixture",
      "bazel_target": "//tools/bazel:guardrail_alpha",
      "bazel_suite": "policy",
      "script": "scripts/ci/check-alpha.ps1",
      "shell": "powershell",
      "root_argument": true,
      "lefthook": { "pre_commit": true, "pre_push": true },
      "ci": { "required": true, "run": "./scripts/ci/check-alpha.ps1" }
    },
    {
      "id": "alpha-guardrail-tests",
      "owner": "build-platform",
      "reason": "fixture",
      "bazel_target": "//tools/bazel:guardrail_alpha_tests",
      "bazel_suite": "policy_tests",
      "script": "scripts/ci/check-alpha.tests.ps1",
      "shell": "powershell",
      "root_argument": false,
      "lefthook": { "pre_commit": false, "pre_push": true },
      "ci": { "required": true, "run": "./scripts/ci/check-alpha.tests.ps1" }
    }
  ]
}
'@
    }

    if (!$MissingScript) {
        Write-File -Root $Root -RelativePath "scripts\ci\check-alpha.ps1" -Content "Write-Host alpha-ok"
        Write-File -Root $Root -RelativePath "scripts\ci\check-alpha.tests.ps1" -Content "Write-Host alpha-tests-ok"
    }

    $alphaTarget = if ($MissingBazelTarget) {
        ""
    } else {
        @'
transition_shell_test(
    name = "guardrail_alpha",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["alpha-guardrail"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)

transition_shell_test(
    name = "guardrail_alpha_tests",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["alpha-guardrail-tests"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)
'@
    }
    $orphanTarget = if ($ExtraBazelTarget) {
        @'

transition_shell_test(
    name = "guardrail_orphan",
    srcs = ["run_guardrail_task.sh"],
    script_args = ["orphan-guardrail"],
    tags = GUARDRAIL_TRANSITION_TAGS,
)
'@
    } else {
        ""
    }
    Write-File -Root $Root -RelativePath "tools\bazel\BUILD.bazel" -Content @"
load("//tools/bazel:shell_test_compat.bzl", "transition_shell_test")

GUARDRAIL_TRANSITION_TAGS = [
    "manual",
    "local",
    "no-sandbox",
    "no-cache",
    "external",
]

$alphaTarget
$orphanTarget
"@

    $policySuiteAlpha = if ($MissingRootSuiteLabel) { "" } else { '        "//tools/bazel:guardrail_alpha",' }
    $orphanSuiteLabel = if ($ExtraRootSuiteLabel) { '        "//tools/bazel:guardrail_orphan",' } else { "" }
    Write-File -Root $Root -RelativePath "BUILD.bazel" -Content @"
test_suite(
    name = "guardrails_policy",
    tests = [
$policySuiteAlpha
$orphanSuiteLabel
    ],
)

test_suite(
    name = "guardrails_policy_tests",
    tests = [
        "//tools/bazel:guardrail_alpha_tests",
    ],
)
"@

    $runnerCase = if ($MissingRunGuardrailCase) {
        ""
    } else {
        @'
  alpha-guardrail)
    run_pwsh scripts/ci/check-alpha.ps1 -Root "$repo_root"
    ;;
  alpha-guardrail-tests)
    run_pwsh scripts/ci/check-alpha.tests.ps1
    ;;
'@
    }
    $orphanRunnerCase = if ($ExtraRunGuardrailCase) {
        @'
  orphan-guardrail)
    run_pwsh scripts/ci/check-alpha.ps1 -Root "$repo_root"
    ;;
'@
    } else {
        ""
    }
    Write-File -Root $Root -RelativePath "tools\bazel\run_guardrail_task.sh" -Content @"
case "`$task" in
$runnerCase
$orphanRunnerCase
esac
"@

    $prePushAlpha = if ($MissingLefthookPrePush) { "" } else { @'
    alpha-guardrail:
      run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-alpha.ps1
    alpha-guardrail-tests:
      run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-alpha.tests.ps1
'@ }
    Write-File -Root $Root -RelativePath "lefthook.yml" -Content @"
pre-commit:
  commands:
    alpha-guardrail:
      run: powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-alpha.ps1

pre-push:
  commands:
$prePushAlpha
"@

    $ciAlpha = if ($MissingCiStep) { "" } else { @'
      - name: Alpha guardrail
        shell: pwsh
        run: ./scripts/ci/check-alpha.ps1
      - name: Alpha guardrail tests
        shell: pwsh
        run: ./scripts/ci/check-alpha.tests.ps1
'@ }
    Write-File -Root $Root -RelativePath ".github\workflows\ci.yml" -Content @"
name: CI
jobs:
  lint-format:
    steps:
$ciAlpha
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
    Assert-Contains $success.Output "verification-task-registry-ok"

    $missingRegistryRoot = Join-Path $TempRoot "missing-registry"
    Write-MinimalRepo -Root $missingRegistryRoot -MissingRegistry
    $missingRegistry = Invoke-Checker -Root $missingRegistryRoot
    Assert-Equals $missingRegistry.ExitCode 1 "missing registry exit code mismatch"
    Assert-Contains $missingRegistry.Output "verification task registry is missing"

    $missingScriptRoot = Join-Path $TempRoot "missing-script"
    Write-MinimalRepo -Root $missingScriptRoot -MissingScript
    $missingScript = Invoke-Checker -Root $missingScriptRoot
    Assert-Equals $missingScript.ExitCode 1 "missing script exit code mismatch"
    Assert-Contains $missingScript.Output "verification task script is missing"

    $missingBazelTargetRoot = Join-Path $TempRoot "missing-bazel-target"
    Write-MinimalRepo -Root $missingBazelTargetRoot -MissingBazelTarget
    $missingBazelTarget = Invoke-Checker -Root $missingBazelTargetRoot
    Assert-Equals $missingBazelTarget.ExitCode 1 "missing Bazel target exit code mismatch"
    Assert-Contains $missingBazelTarget.Output "Bazel guardrail target is missing"

    $missingRootSuiteLabelRoot = Join-Path $TempRoot "missing-root-suite-label"
    Write-MinimalRepo -Root $missingRootSuiteLabelRoot -MissingRootSuiteLabel
    $missingRootSuiteLabel = Invoke-Checker -Root $missingRootSuiteLabelRoot
    Assert-Equals $missingRootSuiteLabel.ExitCode 1 "missing root suite label exit code mismatch"
    Assert-Contains $missingRootSuiteLabel.Output "root guardrail suite is missing"

    $missingRunGuardrailCaseRoot = Join-Path $TempRoot "missing-run-guardrail-case"
    Write-MinimalRepo -Root $missingRunGuardrailCaseRoot -MissingRunGuardrailCase
    $missingRunGuardrailCase = Invoke-Checker -Root $missingRunGuardrailCaseRoot
    Assert-Equals $missingRunGuardrailCase.ExitCode 1 "missing runner case exit code mismatch"
    Assert-Contains $missingRunGuardrailCase.Output "run_guardrail_task.sh missing task case"

    $missingLefthookPrePushRoot = Join-Path $TempRoot "missing-lefthook-pre-push"
    Write-MinimalRepo -Root $missingLefthookPrePushRoot -MissingLefthookPrePush
    $missingLefthookPrePush = Invoke-Checker -Root $missingLefthookPrePushRoot
    Assert-Equals $missingLefthookPrePush.ExitCode 1 "missing lefthook pre-push exit code mismatch"
    Assert-Contains $missingLefthookPrePush.Output "lefthook pre-push is missing"

    $missingCiStepRoot = Join-Path $TempRoot "missing-ci-step"
    Write-MinimalRepo -Root $missingCiStepRoot -MissingCiStep
    $missingCiStep = Invoke-Checker -Root $missingCiStepRoot
    Assert-Equals $missingCiStep.ExitCode 1 "missing CI step exit code mismatch"
    Assert-Contains $missingCiStep.Output "CI workflow is missing"

    $extraBazelTargetRoot = Join-Path $TempRoot "extra-bazel-target"
    Write-MinimalRepo -Root $extraBazelTargetRoot -ExtraBazelTarget
    $extraBazelTarget = Invoke-Checker -Root $extraBazelTargetRoot
    Assert-Equals $extraBazelTarget.ExitCode 1 "extra Bazel target exit code mismatch"
    Assert-Contains $extraBazelTarget.Output "Bazel guardrail target is not registered"

    $extraRootSuiteLabelRoot = Join-Path $TempRoot "extra-root-suite-label"
    Write-MinimalRepo -Root $extraRootSuiteLabelRoot -ExtraRootSuiteLabel
    $extraRootSuiteLabel = Invoke-Checker -Root $extraRootSuiteLabelRoot
    Assert-Equals $extraRootSuiteLabel.ExitCode 1 "extra root suite label exit code mismatch"
    Assert-Contains $extraRootSuiteLabel.Output "root guardrail suite has unregistered target"

    $extraRunGuardrailCaseRoot = Join-Path $TempRoot "extra-run-guardrail-case"
    Write-MinimalRepo -Root $extraRunGuardrailCaseRoot -ExtraRunGuardrailCase
    $extraRunGuardrailCase = Invoke-Checker -Root $extraRunGuardrailCaseRoot
    Assert-Equals $extraRunGuardrailCase.ExitCode 1 "extra runner case exit code mismatch"
    Assert-Contains $extraRunGuardrailCase.Output "run_guardrail_task.sh has unregistered task case"

    Write-Host "verification-task-registry-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
