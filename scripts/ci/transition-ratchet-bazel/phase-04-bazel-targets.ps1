$actualTargetMetadata = Get-BuildTransitionTargetMetadata
$actualBuildTargetSet = Get-BuildTargetSet
$actualTargets = @($actualTargetMetadata.Keys | Sort-Object)
Assert-Unique -Values $actualTargets -Message "Bazel transition target"
$actualTargetSet = @{}
foreach ($target in $actualTargets) {
    $actualTargetSet[$target] = $true
    if ($retiredTransitionTargetSet.ContainsKey($target)) {
        throw "retired transition target still exists: $target"
    }
    if (!$policyByTarget.ContainsKey($target)) {
        throw "missing transition policy for $target"
    }
}
foreach ($target in $policyByTarget.Keys) {
    if (!$actualTargetSet.ContainsKey($target)) {
        throw "stale transition policy for $target"
    }
}
foreach ($exitTarget in $exitTargetByLabel.Keys) {
    $registeredExitTarget = $exitTargetByLabel[$exitTarget]
    if ([string] $registeredExitTarget.state -eq "available" -and !$actualBuildTargetSet.ContainsKey($exitTarget)) {
        throw "available exit target does not exist in Bazel BUILD files: $exitTarget"
    }
    foreach ($evidenceStatus in @($registeredExitTarget.evidence_status)) {
        if ([string] $evidenceStatus.state -ne "available") {
            continue
        }
        $evidenceTarget = [string] $evidenceStatus.bazel_target
        if (!$actualBuildTargetSet.ContainsKey($evidenceTarget)) {
            throw "available exit evidence target does not exist in Bazel BUILD files: $exitTarget -> $evidenceTarget"
        }
    }
}
foreach ($target in $policyByTarget.Keys) {
    $entry = $policyByTarget[$target]
    $metadata = $actualTargetMetadata[$target]
    $runnerScript = [string] $entry.runner_script
    $runnerTask = [string] $entry.runner_task
    if (!(Test-ContainsValue -Values @($metadata.Srcs) -Expected $runnerScript)) {
        throw "transition policy runner_script does not match BUILD srcs: $target -> $runnerScript"
    }
    if (@($metadata.ScriptArgs).Count -ne 1 -or @($metadata.ScriptArgs)[0] -ne $runnerTask) {
        throw "transition policy runner_task does not match BUILD script_args: $target -> $runnerTask"
    }
}
