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

function Get-BazelPackagePath {
    param([string] $Label)

    if ($Label -notmatch '^//([^:]*):[A-Za-z0-9_.-]+$') {
        throw "invalid Bazel label: $Label"
    }
    [string] $Matches[1]
}

function Get-RunnerScriptRelativePath {
    param([string] $Target, [string] $RunnerScript)

    if ([System.IO.Path]::IsPathRooted($RunnerScript)) {
        throw "transition policy runner_script must be package-relative: $Target -> $RunnerScript"
    }
    $packagePath = Get-BazelPackagePath -Label $Target
    if ([string]::IsNullOrWhiteSpace($packagePath)) {
        return $RunnerScript
    }
    "$packagePath/$RunnerScript"
}

function Test-RunnerScriptHasTaskCase {
    param([string] $Content, [string] $RunnerTask)

    $escapedTask = [regex]::Escape($RunnerTask)
    $Content -match "(?m)^\s*$escapedTask\)\s*$"
}

function Test-RunnerScriptHasCommandGuard {
    param([string] $Content, [string] $Command)

    $escapedCommand = [regex]::Escape($Command)
    if ($Content -match "(?m)^\s*require_command\s+['""]?$escapedCommand['""]?\s*(#.*)?$") {
        return $true
    }
    if ($Command -eq "sqlx") {
        return (
            $Content -match '(?m)^\s*require_command\s+"\$\{SQLX_BIN:-sqlx\}"\s*(#.*)?$' -or
            $Content -match '(?m)^\s*require_command\s+"\$sqlx_bin"\s*(#.*)?$'
        )
    }
    $false
}

function Test-RunnerScriptHasServiceGuard {
    param([string] $Content, [string] $Service)

    if ($Service -eq "postgres") {
        return (
            $Content -match '(?m)^\s*wait_for_postgres\s*(#.*)?$' -and
            (Test-RunnerScriptHasCommandGuard -Content $Content -Command "pg_isready")
        )
    }
    $false
}

function Get-WorkflowJobBlocks {
    param([string] $Content)

    $blocks = New-Object System.Collections.Generic.List[object]
    $lines = $Content -split "`r?`n"
    $insideJobs = $false
    $currentName = $null
    $currentLines = New-Object System.Collections.Generic.List[string]

    foreach ($line in $lines) {
        if (!$insideJobs) {
            if ($line -match '^jobs:\s*$') {
                $insideJobs = $true
            }
            continue
        }

        if ($line -match '^\S') {
            break
        }

        if ($line -match '^  ([A-Za-z0-9_-]+):\s*$') {
            if (![string]::IsNullOrWhiteSpace($currentName)) {
                $blocks.Add([pscustomobject]@{
                    Name    = $currentName
                    Content = ($currentLines.ToArray() -join [Environment]::NewLine)
                })
            }
            $currentName = [string] $Matches[1]
            $currentLines.Clear()
            $currentLines.Add($line)
            continue
        }

        if (![string]::IsNullOrWhiteSpace($currentName)) {
            $currentLines.Add($line)
        }
    }

    if (![string]::IsNullOrWhiteSpace($currentName)) {
        $blocks.Add([pscustomobject]@{
            Name    = $currentName
            Content = ($currentLines.ToArray() -join [Environment]::NewLine)
        })
    }

    $blocks.ToArray()
}

function Get-CiTransitionReferenceRecords {
    $records = New-Object System.Collections.Generic.List[object]
    $workflowRoot = Resolve-RepoPath -RelativePath ".github/workflows"
    if (Test-Path -LiteralPath $workflowRoot -PathType Container) {
        Get-ChildItem -LiteralPath $workflowRoot -File |
            Where-Object { $_.Extension -in @(".yml", ".yaml") } |
            ForEach-Object {
                $relativePath = Get-RelativePath -Path $_.FullName
                $content = Read-TextFile -RelativePath $relativePath
                foreach ($jobBlock in @(Get-WorkflowJobBlocks -Content $content)) {
                    foreach ($match in [regex]::Matches([string] $jobBlock.Content, '//[A-Za-z0-9_./-]+:[A-Za-z0-9_.-]*_transition\b')) {
                        $records.Add([pscustomobject]@{
                            Target       = [string] $match.Value
                            RelativePath = $relativePath
                            JobName      = [string] $jobBlock.Name
                            JobContent   = [string] $jobBlock.Content
                            SourceKind   = "workflow"
                        })
                    }
                }
            }
    }
    $lefthookPath = Resolve-RepoPath -RelativePath "lefthook.yml"
    if (Test-Path -LiteralPath $lefthookPath -PathType Leaf) {
        $content = Read-TextFile -RelativePath "lefthook.yml"
        foreach ($match in [regex]::Matches($content, '//[A-Za-z0-9_./-]+:[A-Za-z0-9_.-]*_transition\b')) {
            $records.Add([pscustomobject]@{
                Target       = [string] $match.Value
                RelativePath = "lefthook.yml"
                JobName      = ""
                JobContent   = $content
                SourceKind   = "lefthook"
            })
        }
    }
    $records.ToArray()
}

function Get-CiTransitionReferences {
    $references = New-Object System.Collections.Generic.List[string]
    foreach ($record in @(Get-CiTransitionReferenceRecords)) {
        $references.Add([string] $record.Target)
    }
    $references.ToArray() | Sort-Object -Unique
}

function Test-WorkflowJobHasCommandProvisioning {
    param([string] $Content, [string] $Command)

    switch ($Command) {
        "cargo" {
            return $Content -match 'dtolnay/rust-toolchain@'
        }
        "cargo-deny" {
            return $Content -match '(?m)^\s*run:\s*cargo\s+install\s+cargo-deny\b'
        }
        "cargo-tarpaulin" {
            return $Content -match '(?m)^\s*run:\s*cargo\s+install\s+cargo-tarpaulin\b'
        }
        "curl" {
            return $Content -match '(?m)^\s*(sudo\s+)?apt-get\s+install\b[^\r\n]*\bcurl\b'
        }
        "pg_isready" {
            return $Content -match '(?m)^\s*(sudo\s+)?apt-get\s+install\b[^\r\n]*\bpostgresql-client\b'
        }
        "pnpm" {
            return (
                $Content -match 'pnpm/action-setup@' -and
                $Content -match '(?m)^\s*-\s*run:\s*pnpm\s+install\s+--frozen-lockfile\b'
            )
        }
        "psql" {
            return $Content -match '(?m)^\s*(sudo\s+)?apt-get\s+install\b[^\r\n]*\bpostgresql-client\b'
        }
        "python3" {
            return $Content -match '(?m)^\s*(sudo\s+)?apt-get\s+install\b[^\r\n]*\bpython3\b'
        }
        "sqlx" {
            return $Content -match '(?m)^\s*cargo\s+install\s+sqlx-cli\b'
        }
        default {
            return $false
        }
    }
}

function Test-WorkflowJobHasServiceProvisioning {
    param([string] $Content, [string] $Service)

    if ($Service -eq "postgres") {
        return (
            $Content -match '(?m)^\s+services:\s*$' -and
            $Content -match '(?m)^\s+postgres:\s*$' -and
            $Content -match 'image:\s*postgis/postgis:'
        )
    }
    $false
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

$exitTargetEntries = @()
if ($policy.PSObject.Properties.Name -contains "exit_targets") {
    $exitTargetEntries = @($policy.exit_targets)
}
if ($exitTargetEntries.Count -eq 0) {
    throw "transition ratchet policy must declare exit_targets"
}
Assert-Unique -Values @($exitTargetEntries | ForEach-Object { [string] $_.bazel_target }) -Message "transition ratchet exit target"

$approvalGateRegistryEntries = @()
if ($policy.PSObject.Properties.Name -contains "approval_gate_registry") {
    $approvalGateRegistryEntries = @($policy.approval_gate_registry)
}
if ($approvalGateRegistryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare approval_gate_registry"
}
Assert-Unique -Values @($approvalGateRegistryEntries | ForEach-Object { [string] $_.id }) -Message "transition ratchet approval gate"

$exitEvidenceRequirementEntries = @()
if ($policy.PSObject.Properties.Name -contains "exit_evidence_requirement_registry") {
    $exitEvidenceRequirementEntries = @($policy.exit_evidence_requirement_registry)
}
if ($exitEvidenceRequirementEntries.Count -eq 0) {
    throw "transition ratchet policy must declare exit_evidence_requirement_registry"
}
Assert-Unique `
    -Values @($exitEvidenceRequirementEntries | ForEach-Object { [string] $_.id }) `
    -Message "transition ratchet exit evidence requirement"

$transitionCategoryEntries = @()
if ($policy.PSObject.Properties.Name -contains "transition_category_registry") {
    $transitionCategoryEntries = @($policy.transition_category_registry)
}
if ($transitionCategoryEntries.Count -eq 0) {
    throw "transition ratchet policy must declare transition_category_registry"
}
Assert-Unique -Values @($transitionCategoryEntries | ForEach-Object { [string] $_.id }) -Message "transition ratchet category"

$policyByTarget = @{}
$exitTargetByLabel = @{}
$transitionCategoryById = @{}
$allowedApprovalGates = @{}
$externalCollectionApprovalGateSet = @{}
foreach ($entry in $approvalGateRegistryEntries) {
    $context = "approval gate registry"
    $approvalGate = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($approvalGate -notmatch '^[a-z][a-z0-9_]*$') {
        throw "approval gate registry id must be lowercase snake_case: $approvalGate"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "approval gate registry $approvalGate")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "approval gate registry $approvalGate")
    [void] (Get-RequiredString -Object $entry -Name "decision_reference" -Context "approval gate registry $approvalGate")
    $requiresHumanApproval = Get-RequiredBoolean `
        -Object $entry `
        -Name "requires_human_approval" `
        -Context "approval gate registry $approvalGate"
    if (!$requiresHumanApproval) {
        throw "approval gate registry $approvalGate requires_human_approval must be true"
    }
    $externalCollectionApprovalRequired = Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "approval gate registry $approvalGate"
    $allowedApprovalGates[$approvalGate] = $true
    if ($externalCollectionApprovalRequired) {
        $externalCollectionApprovalGateSet[$approvalGate] = $true
    }
}
if ($externalCollectionApprovalGateSet.Count -eq 0) {
    throw "approval_gate_registry must declare an external collection approval gate"
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
$allowedExitStates = @{
    blocked         = $true
    ready_to_retire = $true
}
$allowedExitEvidenceRequirements = @{}
$allowedExitEvidenceKinds = @{
    native_bazel_evidence   = $true
    pinned_external_evidence = $true
    provisioning_decision   = $true
}
foreach ($entry in $exitEvidenceRequirementEntries) {
    $context = "exit evidence requirement registry"
    $evidenceRequirement = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($evidenceRequirement -notmatch '^[a-z][a-z0-9_]*$') {
        throw "exit evidence requirement registry id must be lowercase snake_case: $evidenceRequirement"
    }
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "owner" `
        -Context "exit evidence requirement registry $evidenceRequirement")
    [void] (Get-RequiredString `
        -Object $entry `
        -Name "reason" `
        -Context "exit evidence requirement registry $evidenceRequirement")
    $evidenceKind = Get-RequiredString `
        -Object $entry `
        -Name "evidence_kind" `
        -Context "exit evidence requirement registry $evidenceRequirement"
    if (!$allowedExitEvidenceKinds.ContainsKey($evidenceKind)) {
        throw "unknown exit evidence requirement kind for ${evidenceRequirement}: $evidenceKind"
    }
    $allowedExitEvidenceRequirements[$evidenceRequirement] = $true
}
foreach ($entry in $transitionCategoryEntries) {
    $context = "transition category registry"
    $category = Get-RequiredString -Object $entry -Name "id" -Context $context
    if ($category -notmatch '^[a-z][a-z0-9-]*$') {
        throw "transition category registry id must be lowercase kebab-case: $category"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition category registry $category")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition category registry $category")
    $categoryEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_exit_evidence_requirements" `
        -Context "transition category registry $category")
    Assert-Unique -Values $categoryEvidenceRequirements -Message "transition category registry $category evidence requirement"
    foreach ($categoryEvidenceRequirement in $categoryEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($categoryEvidenceRequirement)) {
            throw "transition category exit evidence requirement is not registered for ${category}: $categoryEvidenceRequirement"
        }
    }
    $categoryApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "required_approval_gates" `
        -Context "transition category registry $category")
    Assert-Unique -Values $categoryApprovalGates -Message "transition category registry $category approval gate"
    foreach ($categoryApprovalGate in $categoryApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($categoryApprovalGate)) {
            throw "transition category approval gate is not registered for ${category}: $categoryApprovalGate"
        }
    }
    [void] (Get-RequiredBoolean `
        -Object $entry `
        -Name "external_collection_approval_required" `
        -Context "transition category registry $category")
    $transitionCategoryById[$category] = $entry
}
$allowedExitTargetStates = @{
    planned   = $true
    available = $true
}
$runnerTaskRequirements = @{
    "cargo-deny"                     = [pscustomobject]@{ Commands = @("cargo", "cargo-deny"); Services = @() }
    "coverage-tarpaulin"             = [pscustomobject]@{ Commands = @("cargo", "cargo-tarpaulin"); Services = @() }
    "deleted"                        = [pscustomobject]@{ Commands = @(); Services = @() }
    "frontend-e2e"                   = [pscustomobject]@{ Commands = @("pnpm"); Services = @() }
    "migration-v001-full"            = [pscustomobject]@{ Commands = @("pg_isready", "psql", "sqlx"); Services = @("postgres") }
    "migration-v002-audit-immutable" = [pscustomobject]@{ Commands = @("pg_isready", "psql", "sqlx"); Services = @("postgres") }
    "node-audit"                     = [pscustomobject]@{ Commands = @("pnpm"); Services = @() }
    "rust-check"                     = [pscustomobject]@{ Commands = @("cargo"); Services = @() }
    "rustfmt-check"                  = [pscustomobject]@{ Commands = @("cargo"); Services = @() }
    "walking-skeleton-e2e"           = [pscustomobject]@{
        Commands = @("cargo", "curl", "pg_isready", "psql", "python3", "sqlx")
        Services = @("postgres")
    }
}
foreach ($entry in $exitTargetEntries) {
    $context = "exit target registry"
    $exitTarget = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "exit target registry target must be a Bazel label: $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "exit target registry target must not be a transition: $exitTarget"
    }
    $exitTargetState = Get-RequiredString -Object $entry -Name "state" -Context "exit target registry $exitTarget"
    if (!$allowedExitTargetStates.ContainsKey($exitTargetState)) {
        throw "unknown exit target registry state for ${exitTarget}: $exitTargetState"
    }
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "exit target registry $exitTarget")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "exit target registry $exitTarget")
    $exitTargetEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "exit_evidence_requirements" `
        -Context "exit target registry $exitTarget")
    Assert-Unique -Values $exitTargetEvidenceRequirements -Message "exit target registry $exitTarget evidence requirement"
    foreach ($exitTargetEvidenceRequirement in $exitTargetEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($exitTargetEvidenceRequirement)) {
            throw "exit target exit evidence requirement is not registered for ${exitTarget}: $exitTargetEvidenceRequirement"
        }
    }
    $exitTargetBlockingApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "blocking_approval_gates" `
        -Context "exit target registry $exitTarget")
    Assert-Unique -Values $exitTargetBlockingApprovalGates -Message "exit target registry $exitTarget blocking approval gate"
    foreach ($exitTargetBlockingApprovalGate in $exitTargetBlockingApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($exitTargetBlockingApprovalGate)) {
            throw "exit target approval gate is not registered for ${exitTarget}: $exitTargetBlockingApprovalGate"
        }
    }
    $exitTargetByLabel[$exitTarget] = $entry
}
foreach ($entry in $policyEntries) {
    $context = "transition policy"
    $target = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "transition policy target must be a Bazel _transition label: $target"
    }
    $category = Get-RequiredString -Object $entry -Name "category" -Context "transition policy $target"
    if (!$transitionCategoryById.ContainsKey($category)) {
        throw "transition category is not registered: $target -> $category"
    }
    $registeredCategory = $transitionCategoryById[$category]
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition policy $target")
    $exitTarget = Get-RequiredString -Object $entry -Name "exit_target" -Context "transition policy $target"
    if ($exitTarget -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+$') {
        throw "transition policy exit_target must be a Bazel label: $target -> $exitTarget"
    }
    if ($exitTarget -match '_transition$') {
        throw "transition policy exit_target must not be another transition: $target -> $exitTarget"
    }
    if (!$exitTargetByLabel.ContainsKey($exitTarget)) {
        throw "transition exit_target is not registered: $target -> $exitTarget"
    }
    $registeredExitTarget = $exitTargetByLabel[$exitTarget]
    $exitState = Get-RequiredString -Object $entry -Name "exit_state" -Context "transition policy $target"
    if (!$allowedExitStates.ContainsKey($exitState)) {
        throw "unknown transition exit_state for ${target}: $exitState"
    }
    $exitEvidenceRequirements = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "exit_evidence_requirements" `
        -Context "transition policy $target")
    Assert-Unique -Values $exitEvidenceRequirements -Message "transition policy $target exit evidence requirement"
    foreach ($exitEvidenceRequirement in $exitEvidenceRequirements) {
        if (!$allowedExitEvidenceRequirements.ContainsKey($exitEvidenceRequirement)) {
            throw "transition exit evidence requirement is not registered for ${target}: $exitEvidenceRequirement"
        }
    }
    Assert-ContainsAll `
        -Actual $exitEvidenceRequirements `
        -Expected @($registeredCategory.required_exit_evidence_requirements | ForEach-Object { [string] $_ }) `
        -Message "transition category required_exit_evidence_requirements for $target"
    Assert-ContainsAll `
        -Actual @($registeredExitTarget.exit_evidence_requirements | ForEach-Object { [string] $_ }) `
        -Expected $exitEvidenceRequirements `
        -Message "exit target registry exit_evidence_requirements for $exitTarget"
    $blockingApprovalGates = @(Get-RequiredStringArray `
        -Object $entry `
        -Name "blocking_approval_gates" `
        -Context "transition policy $target")
    Assert-Unique -Values $blockingApprovalGates -Message "transition policy $target blocking approval gate"
    foreach ($blockingApprovalGate in $blockingApprovalGates) {
        if (!$allowedApprovalGates.ContainsKey($blockingApprovalGate)) {
            throw "transition blocking approval gate is not registered for ${target}: $blockingApprovalGate"
        }
    }
    Assert-ContainsAll `
        -Actual @($registeredExitTarget.blocking_approval_gates | ForEach-Object { [string] $_ }) `
        -Expected $blockingApprovalGates `
        -Message "exit target registry blocking_approval_gates for $exitTarget"
    $registeredExitTargetState = [string] $registeredExitTarget.state
    if ($exitState -eq "blocked" -and $registeredExitTargetState -ne "planned") {
        throw "blocked transition must point at a planned exit target: $target -> $exitTarget"
    }
    if ($exitState -eq "ready_to_retire" -and $registeredExitTargetState -ne "available") {
        throw "ready_to_retire transition must point at an available exit target: $target -> $exitTarget"
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
            throw "transition approval gate is not registered for ${target}: $approvalGate"
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
    Assert-ContainsAll `
        -Actual $approvalGates `
        -Expected @($registeredCategory.required_approval_gates | ForEach-Object { [string] $_ }) `
        -Message "transition category required_approval_gates for $target"
    $categoryExternalCollectionApprovalRequired = [bool] $registeredCategory.external_collection_approval_required
    if ($categoryExternalCollectionApprovalRequired -and !$externalCollection) {
        throw "transition category requires external collection approval: $target -> $category"
    }
    $hasExternalCollectionApprovalGate = $false
    foreach ($approvalGate in $approvalGates) {
        if ($externalCollectionApprovalGateSet.ContainsKey($approvalGate)) {
            $hasExternalCollectionApprovalGate = $true
        }
    }
    if ($externalCollection -and !$hasExternalCollectionApprovalGate) {
        throw "external collection transition must declare external advisory approval gate: $target"
    }
    foreach ($approvalGate in $approvalGates) {
        if ($externalCollectionApprovalGateSet.ContainsKey($approvalGate) -and !$externalCollection) {
            throw "external advisory approval gate must require external collection approval: $target"
        }
    }
    foreach ($approvalGate in $approvalGates) {
        if (!(Test-ContainsValue -Values $blockingApprovalGates -Expected $approvalGate)) {
            throw "transition policy blocking_approval_gates for $target missing '$approvalGate'"
        }
    }
    foreach ($blockingApprovalGate in $blockingApprovalGates) {
        if (!(Test-ContainsValue -Values $approvalGates -Expected $blockingApprovalGate)) {
            throw "transition policy blocking_approval_gates for $target must be declared in approval_gates: $blockingApprovalGate"
        }
    }
    if ($exitState -eq "ready_to_retire" -and $approvalGates.Count -gt 0) {
        throw "ready_to_retire transition must not have unresolved approval_gates: $target"
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

Write-Host "bazel-transition-ratchet-ok targets=$($actualTargets.Count) ci_refs=$($ciReferences.Count)"
