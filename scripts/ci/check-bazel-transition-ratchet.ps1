param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param([string] $RelativePath)

    [System.IO.Path]::GetFullPath((Join-Path $Root $RelativePath))
}

function Get-RelativePath {
    param([string] $Path)

    $rootPath = [System.IO.Path]::GetFullPath($Root).TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if (!$fullPath.StartsWith($rootPath, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "path is outside repo root: $Path"
    }
    $relative = $fullPath.Substring($rootPath.Length).TrimStart([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar)
    $relative -replace "\\", "/"
}

function Read-TextFile {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "missing file: $RelativePath"
    }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)

    Read-TextFile -RelativePath $RelativePath | ConvertFrom-Json
}

function Get-RequiredString {
    param([object] $Object, [string] $Name, [string] $Context)

    if (!($Object.PSObject.Properties.Name -contains $Name)) {
        throw "$Context missing '$Name'"
    }
    $value = [string] $Object.$Name
    if ([string]::IsNullOrWhiteSpace($value)) {
        throw "$Context '$Name' must not be empty"
    }
    $value
}

function Get-RequiredBoolean {
    param([object] $Object, [string] $Name, [string] $Context)

    if (!($Object.PSObject.Properties.Name -contains $Name)) {
        throw "$Context missing '$Name'"
    }
    if ($Object.$Name -isnot [bool]) {
        throw "$Context '$Name' must be boolean"
    }
    [bool] $Object.$Name
}

function Get-RequiredStringArray {
    param([object] $Object, [string] $Name, [string] $Context)

    if (!($Object.PSObject.Properties.Name -contains $Name)) {
        throw "$Context missing '$Name'"
    }
    $rawValue = $Object.PSObject.Properties[$Name].Value
    if ($null -eq $rawValue) {
        throw "$Context '$Name' must be a string array"
    }
    if ($rawValue -isnot [System.Array]) {
        throw "$Context '$Name' must be a string array"
    }

    $values = @()
    foreach ($value in @($rawValue)) {
        $stringValue = [string] $value
        if ([string]::IsNullOrWhiteSpace($stringValue)) {
            throw "$Context '$Name' entries must not be empty"
        }
        $values += $stringValue
    }
    $values
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)

    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Unique {
    param([string[]] $Values, [string] $Message)

    $seen = @{}
    foreach ($value in $Values) {
        if ($seen.ContainsKey($value)) {
            throw "$Message duplicate '$value'"
        }
        $seen[$value] = $true
    }
}

function Assert-ContainsAll {
    param([string[]] $Actual, [string[]] $Expected, [string] $Message)

    foreach ($expectedValue in $Expected) {
        if (!(Test-ContainsValue -Values $Actual -Expected $expectedValue)) {
            throw "$Message missing '$expectedValue'"
        }
    }
}

function Test-ContainsValue {
    param([string[]] $Values, [string] $Expected)

    foreach ($value in $Values) {
        if ($value -eq $Expected) {
            return $true
        }
    }
    $false
}

function Get-ListLiteralValues {
    param([string] $Line, [string] $Name)

    $escapedName = [regex]::Escape($Name)
    if ($Line -notmatch "^\s*$escapedName\s*=\s*\[(.*)\]\s*,?\s*$") {
        return @()
    }
    $values = @()
    foreach ($match in [regex]::Matches($Matches[1], '"([^"]+)"')) {
        $values += [string] $match.Groups[1].Value
    }
    $values
}

function Test-IsIgnoredPath {
    param([string] $RelativePath)

    foreach ($segment in @(
        ".git/",
        "bazel-",
        "node_modules/",
        "target/",
        ".next/",
        ".turbo/"
    )) {
        if ($RelativePath.Contains($segment)) {
            return $true
        }
    }
    $false
}

function Get-BuildTransitionTargetMetadata {
    $targets = @{}
    $buildFiles = @(
        Get-ChildItem -LiteralPath (Resolve-RepoPath -RelativePath ".") -Recurse -File -Filter "BUILD.bazel" |
            Sort-Object FullName
    )
    foreach ($file in $buildFiles) {
        $relativePath = Get-RelativePath -Path $file.FullName
        if (Test-IsIgnoredPath -RelativePath $relativePath) {
            continue
        }
        $packagePath = Split-Path -Parent $relativePath
        if ($packagePath -eq "." -or [string]::IsNullOrWhiteSpace($packagePath)) {
            $packagePath = ""
        } else {
            $packagePath = $packagePath -replace "\\", "/"
        }

        $insideTransitionShellTest = $false
        $blockLines = New-Object System.Collections.Generic.List[string]
        foreach ($line in Get-Content -LiteralPath $file.FullName -Encoding UTF8) {
            if ($line -match '^\s*transition_shell_test\s*\(') {
                $insideTransitionShellTest = $true
                $blockLines.Clear()
                continue
            }
            if ($insideTransitionShellTest) {
                $blockLines.Add($line)
            }
            if ($insideTransitionShellTest -and $line -match '^\s*\),?\s*$') {
                $name = $null
                $srcs = @()
                $scriptArgs = @()
                foreach ($blockLine in $blockLines) {
                    if ($blockLine -match '^\s*name\s*=\s*"([^"]+_transition)"') {
                        $name = $Matches[1]
                    }
                    $srcs += @(Get-ListLiteralValues -Line $blockLine -Name "srcs")
                    $scriptArgs += @(Get-ListLiteralValues -Line $blockLine -Name "script_args")
                }
                if (![string]::IsNullOrWhiteSpace($name)) {
                    $label = if ([string]::IsNullOrWhiteSpace($packagePath)) {
                        "//:$name"
                    } else {
                        "//${packagePath}:$name"
                    }
                    $targets[$label] = [pscustomobject]@{
                        Srcs       = @($srcs)
                        ScriptArgs = @($scriptArgs)
                    }
                }
                $insideTransitionShellTest = $false
            }
        }
    }
    $targets
}

function Get-BuildTransitionTargets {
    @(Get-BuildTransitionTargetMetadata).Keys | Sort-Object
}

function Get-CiTransitionReferences {
    $references = New-Object System.Collections.Generic.List[string]
    $paths = New-Object System.Collections.Generic.List[string]
    $workflowRoot = Resolve-RepoPath -RelativePath ".github/workflows"
    if (Test-Path -LiteralPath $workflowRoot -PathType Container) {
        Get-ChildItem -LiteralPath $workflowRoot -File |
            Where-Object { $_.Extension -in @(".yml", ".yaml") } |
            ForEach-Object { $paths.Add((Get-RelativePath -Path $_.FullName)) }
    }
    $lefthookPath = Resolve-RepoPath -RelativePath "lefthook.yml"
    if (Test-Path -LiteralPath $lefthookPath -PathType Leaf) {
        $paths.Add("lefthook.yml")
    }

    $transitionLabelPattern = '//[A-Za-z0-9_./-]+:[A-Za-z0-9_.-]*_transition\b'
    foreach ($relativePath in $paths) {
        $content = Read-TextFile -RelativePath $relativePath
        foreach ($match in [regex]::Matches($content, $transitionLabelPattern)) {
            $references.Add($match.Value)
        }
    }
    $references.ToArray() | Sort-Object -Unique
}

$policy = Read-JsonFile -RelativePath "docs/architecture/verification-transition-ratchet.v1.json"
Assert-Equals -Actual $policy.schema_version -Expected "gongzzang.verification_transition_ratchet.v1" -Message "transition ratchet schema_version mismatch"
Assert-Equals -Actual $policy.repo_slug -Expected "gongzzang" -Message "transition ratchet repo_slug mismatch"
Assert-Equals `
    -Actual $policy.default_decision `
    -Expected "deny_new_transition_without_policy" `
    -Message "transition ratchet default decision mismatch"

$policyEntries = @($policy.transition_targets)
if ($policyEntries.Count -eq 0) {
    throw "transition ratchet policy must declare at least one transition target"
}
Assert-Unique -Values @($policyEntries | ForEach-Object { [string] $_.bazel_target }) -Message "transition ratchet policy target"

$retiredTransitionTargets = @()
if ($policy.PSObject.Properties.Name -contains "retired_transition_targets") {
    $retiredTransitionTargets = @($policy.retired_transition_targets | ForEach-Object { [string] $_ })
}
Assert-Unique -Values $retiredTransitionTargets -Message "retired transition target"
$retiredTransitionTargetSet = @{}
foreach ($target in $retiredTransitionTargets) {
    if ([string]::IsNullOrWhiteSpace($target)) {
        throw "retired transition target must not be empty"
    }
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "retired transition target must be a Bazel _transition label: $target"
    }
    $retiredTransitionTargetSet[$target] = $true
}

$policyByTarget = @{}
$allowedApprovalGates = @{
    external_advisory_collection  = $true
    browser_runtime_provisioning  = $true
    toolchain_provisioning        = $true
    database_service_provisioning = $true
    service_orchestration_provisioning = $true
}
$allowedRequiredCommands = @{
    cargo           = $true
    "cargo-deny"    = $true
    "cargo-tarpaulin" = $true
    curl            = $true
    pg_isready      = $true
    pnpm            = $true
    psql            = $true
    python3         = $true
    sqlx            = $true
}
$allowedRequiredServices = @{
    postgres = $true
}
$runnerTaskRequirements = @{
    "cargo-deny"                     = [pscustomobject]@{ Commands = @("cargo", "cargo-deny"); Services = @() }
    "coverage-tarpaulin"             = [pscustomobject]@{ Commands = @("cargo", "cargo-tarpaulin"); Services = @() }
    "deleted"                        = [pscustomobject]@{ Commands = @(); Services = @() }
    "frontend-e2e"                   = [pscustomobject]@{ Commands = @("pnpm"); Services = @() }
    "migration-v001-full"            = [pscustomobject]@{ Commands = @("sqlx", "psql"); Services = @("postgres") }
    "migration-v002-audit-immutable" = [pscustomobject]@{ Commands = @("sqlx", "psql"); Services = @("postgres") }
    "node-audit"                     = [pscustomobject]@{ Commands = @("pnpm"); Services = @() }
    "rust-check"                     = [pscustomobject]@{ Commands = @("cargo"); Services = @() }
    "rustfmt-check"                  = [pscustomobject]@{ Commands = @("cargo"); Services = @() }
    "walking-skeleton-e2e"           = [pscustomobject]@{
        Commands = @("cargo", "curl", "pg_isready", "psql", "python3", "sqlx")
        Services = @("postgres")
    }
}
foreach ($entry in $policyEntries) {
    $context = "transition policy"
    $target = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "transition policy target must be a Bazel _transition label: $target"
    }
    $category = Get-RequiredString -Object $entry -Name "category" -Context "transition policy $target"
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition policy $target")
    $exitTarget = Get-RequiredString -Object $entry -Name "exit_target" -Context "transition policy $target"
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "transition policy exit_target must be a Bazel label: $target -> $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "transition policy exit_target must not be another transition: $target -> $exitTarget"
    }
    $runnerScript = Get-RequiredString -Object $entry -Name "runner_script" -Context "transition policy $target"
    $runnerTask = Get-RequiredString -Object $entry -Name "runner_task" -Context "transition policy $target"
    if (!$runnerTaskRequirements.ContainsKey($runnerTask)) {
        throw "unknown transition runner_task for ${target}: $runnerTask"
    }
    $requiredCommands = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_commands" `
        -Context "transition policy $target")
    Assert-Unique -Values $requiredCommands -Message "transition policy $target required command"
    foreach ($requiredCommand in $requiredCommands) {
        if (!$allowedRequiredCommands.ContainsKey($requiredCommand)) {
            throw "unknown transition required command for ${target}: $requiredCommand"
        }
    }
    $requiredServices = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_services" `
        -Context "transition policy $target")
    Assert-Unique -Values $requiredServices -Message "transition policy $target required service"
    foreach ($requiredService in $requiredServices) {
        if (!$allowedRequiredServices.ContainsKey($requiredService)) {
            throw "unknown transition required service for ${target}: $requiredService"
        }
    }
    $runnerRequirements = $runnerTaskRequirements[$runnerTask]
    Assert-ContainsAll `
        -Actual $requiredCommands `
        -Expected @($runnerRequirements.Commands) `
        -Message "transition policy required_commands for $target"
    Assert-ContainsAll `
        -Actual $requiredServices `
        -Expected @($runnerRequirements.Services) `
        -Message "transition policy required_services for $target"
    $approvalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "approval_gates" `
        -Context "transition policy $target")
    Assert-Unique -Values $approvalGates -Message "transition policy $target approval gate"
    foreach ($approvalGate in $approvalGates) {
        if (!$allowedApprovalGates.ContainsKey($approvalGate)) {
            throw "unknown transition approval gate for ${target}: $approvalGate"
        }
    }
    $sunset = Get-RequiredString -Object $entry -Name "sunset" -Context "transition policy $target"
    $sunsetDate = [DateTime]::ParseExact(
        $sunset,
        "yyyy-MM-dd",
        [System.Globalization.CultureInfo]::InvariantCulture
    )
    if ($sunsetDate.Date -lt [DateTime]::UtcNow.Date) {
        throw "expired transition sunset for ${target}: $sunset"
    }
    $externalCollection = Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "transition policy $target"
    if ($category -eq "external-advisory-sca" -and !$externalCollection) {
        throw "external advisory collection transition must require approval: $target"
    }
    if (
        $category -eq "external-advisory-sca" -and
        !(Test-ContainsValue -Values $approvalGates -Expected "external_advisory_collection")
    ) {
        throw "external advisory transition must declare approval gate: $target"
    }
    if ($externalCollection -and !(Test-ContainsValue -Values $approvalGates -Expected "external_advisory_collection")) {
        throw "external collection transition must declare external advisory approval gate: $target"
    }
    if (
        (Test-ContainsValue -Values $approvalGates -Expected "external_advisory_collection") -and
        !$externalCollection
    ) {
        throw "external advisory approval gate must require external collection approval: $target"
    }
    if (
        $target -eq "//tools/bazel:frontend_e2e_transition" -and
        !(Test-ContainsValue -Values $approvalGates -Expected "browser_runtime_provisioning")
    ) {
        throw "frontend e2e transition must declare browser runtime provisioning gate: $target"
    }
    if (
        $category -eq "coverage-verification" -and
        !(Test-ContainsValue -Values $approvalGates -Expected "toolchain_provisioning")
    ) {
        throw "coverage transition must declare toolchain provisioning gate: $target"
    }
    if ($category -eq "database-verification") {
        if (!(Test-ContainsValue -Values $approvalGates -Expected "toolchain_provisioning")) {
            throw "database transition must declare toolchain provisioning gate: $target"
        }
        if (!(Test-ContainsValue -Values $approvalGates -Expected "database_service_provisioning")) {
            throw "database transition must declare database service provisioning gate: $target"
        }
    }
    if ($category -eq "service-e2e-verification") {
        if (!(Test-ContainsValue -Values $approvalGates -Expected "toolchain_provisioning")) {
            throw "service e2e transition must declare toolchain provisioning gate: $target"
        }
        if (!(Test-ContainsValue -Values $approvalGates -Expected "database_service_provisioning")) {
            throw "service e2e transition must declare database service provisioning gate: $target"
        }
        if (!(Test-ContainsValue -Values $approvalGates -Expected "service_orchestration_provisioning")) {
            throw "service e2e transition must declare service orchestration gate: $target"
        }
    }
    $policyByTarget[$target] = $entry
}

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

$ciReferences = @(Get-CiTransitionReferences)
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

Write-Host "bazel-transition-ratchet-ok targets=$($actualTargets.Count) ci_refs=$($ciReferences.Count)"
