$runnerScriptContentByPath = @{}
foreach ($target in $policyByTarget.Keys) {
    $entry = $policyByTarget[$target]
    $runnerScript = [string] $entry.runner_script
    $runnerTask = [string] $entry.runner_task
    $runnerScriptPath = Get-RunnerScriptRelativePath -Target $target -RunnerScript $runnerScript
    if (!$runnerScriptContentByPath.ContainsKey($runnerScriptPath)) {
        $runnerScriptContentByPath[$runnerScriptPath] = Read-TextFile -RelativePath $runnerScriptPath
    }
    $runnerScriptContent = [string] $runnerScriptContentByPath[$runnerScriptPath]
    if (!(Test-RunnerScriptHasTaskCase -Content $runnerScriptContent -RunnerTask $runnerTask)) {
        throw "runner script missing task case: $target -> $runnerScriptPath task=$runnerTask"
    }
    foreach ($requiredCommand in @($entry.required_commands | ForEach-Object { [string] $_ })) {
        if (!(Test-RunnerScriptHasCommandGuard -Content $runnerScriptContent -Command $requiredCommand)) {
            throw "runner script missing required command guard: $target -> $runnerScriptPath command=$requiredCommand"
        }
    }
    foreach ($requiredService in @($entry.required_services | ForEach-Object { [string] $_ })) {
        if (!(Test-RunnerScriptHasServiceGuard -Content $runnerScriptContent -Service $requiredService)) {
            throw "runner script missing required service guard: $target -> $runnerScriptPath service=$requiredService"
        }
    }
}

$ciReferenceRecords = @(Get-CiTransitionReferenceRecords)
$ciReferences = @($ciReferenceRecords | ForEach-Object { [string] $_.Target } | Sort-Object -Unique)
$ciReferenceSet = @{}
foreach ($target in $ciReferences) {
    $ciReferenceSet[$target] = $true
    if ($retiredTransitionTargetSet.ContainsKey($target)) {
        throw "CI references retired transition target: $target"
    }
    if (!$policyByTarget.ContainsKey($target)) {
        throw "CI references transition target without policy: $target"
    }
}
foreach ($target in $policyByTarget.Keys) {
    if (!$ciReferenceSet.ContainsKey($target)) {
        throw "active transition target is not referenced by CI or hooks: $target"
    }
}

foreach ($record in $ciReferenceRecords) {
    if ([string] $record.SourceKind -ne "workflow") {
        continue
    }
    $target = [string] $record.Target
    if (!$policyByTarget.ContainsKey($target)) {
        continue
    }
    $entry = $policyByTarget[$target]
    $jobContent = [string] $record.JobContent
    foreach ($requiredCommand in @($entry.required_commands | ForEach-Object { [string] $_ })) {
        if (!(Test-WorkflowJobHasCommandProvisioning -Content $jobContent -Command $requiredCommand)) {
            throw "workflow job missing required command provisioning: $target -> $($record.RelativePath) job=$($record.JobName) command=$requiredCommand"
        }
    }
    foreach ($requiredService in @($entry.required_services | ForEach-Object { [string] $_ })) {
        if (!(Test-WorkflowJobHasServiceProvisioning -Content $jobContent -Service $requiredService)) {
            throw "workflow job missing required service provisioning: $target -> $($record.RelativePath) job=$($record.JobName) service=$requiredService"
        }
    }
}
