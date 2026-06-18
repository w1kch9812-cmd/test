$actualTargetMetadata = Get-BuildTransitionTargetMetadata
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
