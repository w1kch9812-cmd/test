[CmdletBinding()]
param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)

    [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-TextFile {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "required file is missing: $RelativePath"
    }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)

    Read-TextFile -RelativePath $RelativePath | ConvertFrom-Json
}

function Assert-String {
    param([object] $Value, [string] $Message)

    if ([string]::IsNullOrWhiteSpace([string] $Value)) {
        throw "$Message must be set"
    }
}

function Assert-BooleanProperty {
    param([object] $Object, [string] $Name, [string] $Context)

    if (!($Object.PSObject.Properties.Name -contains $Name)) {
        throw "$Context missing '$Name'"
    }
    if ($Object.$Name -isnot [bool]) {
        throw "$Context '$Name' must be boolean"
    }
}

function Assert-Unique {
    param([string[]] $Values, [string] $Message)

    $seen = @{}
    foreach ($value in $Values) {
        if ($seen.ContainsKey($value)) {
            throw "$Message duplicate: $value"
        }
        $seen[$value] = $true
    }
}

function Get-YamlTopLevelSection {
    param([string] $Content, [string] $Name)

    $escapedName = [regex]::Escape($Name)
    if ($Content -notmatch "(?ms)^$escapedName\s*:\s*\r?\n(.*?)(?=^\S|\z)") {
        return ""
    }
    [string] $Matches[1]
}

function Test-ContainsLine {
    param([string] $Content, [string] $Line)

    $escapedLine = [regex]::Escape($Line)
    $Content -match "(?m)^\s*$escapedLine\s*$"
}

function Get-RuleNameFromLabel {
    param([string] $BazelTarget)

    if ($BazelTarget -notmatch '^//tools/bazel:([A-Za-z0-9_.-]+)$') {
        throw "verification task bazel_target must be a tools/bazel label: $BazelTarget"
    }
    [string] $Matches[1]
}

function Get-SuiteName {
    param([string] $Suite)

    switch ($Suite) {
        "fast" { "guardrails_fast"; break }
        "policy" { "guardrails_policy"; break }
        "policy_tests" { "guardrails_policy_tests"; break }
        default { throw "verification task bazel_suite is not registered: $Suite" }
    }
}

function Assert-LefthookProjection {
    param(
        [string] $Section,
        [object] $Task,
        [string] $HookName
    )

    $id = [string] $Task.id
    $command = $id
    if ($Task.lefthook.PSObject.Properties.Name -contains "command") {
        $command = [string] $Task.lefthook.command
        Assert-String -Value $command -Message "verification task $id lefthook.command"
    }
    $script = [string] $Task.script
    if ($Section -notmatch "(?m)^\s{4}$([regex]::Escape($command))\s*:\s*$") {
        throw "lefthook $HookName is missing task: $id"
    }
    if (!$Section.Contains($script)) {
        throw "lefthook $HookName task must run script: $id -> $script"
    }
}

function New-StringSet {
    param([string[]] $Values)

    $set = @{}
    foreach ($value in $Values) {
        $set[$value] = $true
    }
    $set
}

function Assert-BazelGuardrailTargetsAreRegistered {
    param([string] $Content, [hashtable] $RegisteredTargets)

    $pattern = '(?ms)transition_shell_test\(\s*name\s*=\s*"(?<name>guardrail_[^"]+)".*?script_args\s*=\s*\[\s*"(?<arg>[^"]+)"\s*\]'
    foreach ($match in [regex]::Matches($Content, $pattern)) {
        $label = "//tools/bazel:$($match.Groups["name"].Value)"
        if (!$RegisteredTargets.ContainsKey($label)) {
            throw "Bazel guardrail target is not registered: $label"
        }
    }
}

function Assert-RootGuardrailSuitesAreRegistered {
    param([string] $Content, [hashtable] $RegisteredTargets)

    $suitePattern = '(?ms)test_suite\(\s*name\s*=\s*"guardrails_(?:fast|policy|policy_tests)".*?tests\s*=\s*\[(?<body>.*?)\]'
    foreach ($suite in [regex]::Matches($Content, $suitePattern)) {
        $body = $suite.Groups["body"].Value
        foreach ($label in [regex]::Matches($body, '"(?<label>//tools/bazel:guardrail_[^"]+)"')) {
            $target = $label.Groups["label"].Value
            if (!$RegisteredTargets.ContainsKey($target)) {
                throw "root guardrail suite has unregistered target: $target"
            }
        }
    }
}

function Assert-RunGuardrailCasesAreRegistered {
    param([string] $Content, [hashtable] $RegisteredIds)

    foreach ($match in [regex]::Matches($Content, '(?m)^\s{2}(?<id>[a-z][a-z0-9-]*)\)\s*$')) {
        $id = $match.Groups["id"].Value
        if (!$RegisteredIds.ContainsKey($id)) {
            throw "run_guardrail_task.sh has unregistered task case: $id"
        }
    }
}

$registryPath = "docs/architecture/verification-task-registry.v1.json"
$registryFile = Resolve-RepoPath -RelativePath $registryPath
if (!(Test-Path -LiteralPath $registryFile -PathType Leaf)) {
    throw "verification task registry is missing: $registryPath"
}

$registry = Read-JsonFile -RelativePath $registryPath
if ([string] $registry.schema_version -ne "gongzzang.verification_task_registry.v1") {
    throw "verification task registry schema_version mismatch"
}
if ([string] $registry.repo_slug -ne "gongzzang") {
    throw "verification task registry repo_slug mismatch"
}

$tasks = @($registry.tasks)
if ($tasks.Count -eq 0) {
    throw "verification task registry must declare tasks"
}
Assert-Unique -Values @($tasks | ForEach-Object { [string] $_.id }) -Message "verification task id"
Assert-Unique -Values @($tasks | ForEach-Object { [string] $_.bazel_target }) -Message "verification task bazel_target"
$registeredIds = New-StringSet -Values @($tasks | ForEach-Object { [string] $_.id })
$registeredTargets = New-StringSet -Values @($tasks | ForEach-Object { [string] $_.bazel_target })

$toolsBazelBuild = Read-TextFile -RelativePath "tools/bazel/BUILD.bazel"
$rootBuild = Read-TextFile -RelativePath "BUILD.bazel"
$runGuardrailTask = Read-TextFile -RelativePath "tools/bazel/run_guardrail_task.sh"
$lefthook = Read-TextFile -RelativePath "lefthook.yml"
$ciWorkflow = Read-TextFile -RelativePath ".github/workflows/ci.yml"
$preCommit = Get-YamlTopLevelSection -Content $lefthook -Name "pre-commit"
$prePush = Get-YamlTopLevelSection -Content $lefthook -Name "pre-push"

Assert-BazelGuardrailTargetsAreRegistered -Content $toolsBazelBuild -RegisteredTargets $registeredTargets
Assert-RootGuardrailSuitesAreRegistered -Content $rootBuild -RegisteredTargets $registeredTargets
Assert-RunGuardrailCasesAreRegistered -Content $runGuardrailTask -RegisteredIds $registeredIds

$taskCount = 0
foreach ($task in $tasks) {
    $id = [string] $task.id
    $context = "verification task $id"
    Assert-String -Value $id -Message "task.id"
    if ($id -notmatch '^[a-z][a-z0-9-]*$') {
        throw "verification task id must be lowercase kebab-case: $id"
    }
    Assert-String -Value $task.owner -Message "$context owner"
    Assert-String -Value $task.reason -Message "$context reason"
    Assert-String -Value $task.bazel_target -Message "$context bazel_target"
    Assert-String -Value $task.bazel_suite -Message "$context bazel_suite"
    Assert-String -Value $task.script -Message "$context script"
    Assert-String -Value $task.shell -Message "$context shell"
    Assert-BooleanProperty -Object $task -Name "root_argument" -Context $context
    Assert-BooleanProperty -Object $task.lefthook -Name "pre_commit" -Context "$context lefthook"
    Assert-BooleanProperty -Object $task.lefthook -Name "pre_push" -Context "$context lefthook"
    Assert-BooleanProperty -Object $task.ci -Name "required" -Context "$context ci"

    $script = [string] $task.script
    if (!(Test-Path -LiteralPath (Resolve-RepoPath -RelativePath $script) -PathType Leaf)) {
        throw "verification task script is missing: $id -> $script"
    }

    $ruleName = Get-RuleNameFromLabel -BazelTarget ([string] $task.bazel_target)
    if ($toolsBazelBuild -notmatch "(?m)^\s*name\s*=\s*`"$([regex]::Escape($ruleName))`"\s*,\s*$") {
        throw "Bazel guardrail target is missing: $($task.bazel_target)"
    }
    if ($toolsBazelBuild -notmatch "(?m)^\s*script_args\s*=\s*\[\s*`"$([regex]::Escape($id))`"\s*\]\s*,\s*$") {
        throw "Bazel guardrail target script_args mismatch: $($task.bazel_target) -> $id"
    }

    $suiteName = Get-SuiteName -Suite ([string] $task.bazel_suite)
    $escapedBazelTarget = [regex]::Escape([string] $task.bazel_target)
    if ($rootBuild -notmatch "(?ms)name\s*=\s*`"$suiteName`".*?`"$escapedBazelTarget`"\s*,") {
        throw "root guardrail suite is missing $($task.bazel_target) in $suiteName"
    }

    if ($runGuardrailTask -notmatch "(?m)^\s*$([regex]::Escape($id))\)\s*$") {
        throw "run_guardrail_task.sh missing task case: $id"
    }
    if (!$runGuardrailTask.Contains($script)) {
        throw "run_guardrail_task.sh task must invoke script: $id -> $script"
    }
    if ([bool] $task.root_argument) {
        $expectedRootInvocation = "$script -Root ""`$repo_root"""
        if (!$runGuardrailTask.Contains($expectedRootInvocation)) {
            throw "run_guardrail_task.sh task must pass repo root: $id"
        }
    }

    if ([bool] $task.lefthook.pre_commit) {
        Assert-LefthookProjection -Section $preCommit -Task $task -HookName "pre-commit"
    }
    if ([bool] $task.lefthook.pre_push) {
        Assert-LefthookProjection -Section $prePush -Task $task -HookName "pre-push"
    }

    if ([bool] $task.ci.required) {
        Assert-String -Value $task.ci.run -Message "$context ci.run"
        $ciRun = [string] $task.ci.run
        if (!$ciWorkflow.Contains($ciRun)) {
            throw "CI workflow is missing task run: $id -> $ciRun"
        }
    }
    $taskCount += 1
}

Write-Host "verification-task-registry-ok tasks=$taskCount"
