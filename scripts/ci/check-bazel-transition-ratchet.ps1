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

function Get-BuildTransitionTargets {
    $targets = New-Object System.Collections.Generic.List[string]
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
        foreach ($line in Get-Content -LiteralPath $file.FullName -Encoding UTF8) {
            if ($line -match '^\s*transition_shell_test\s*\(') {
                $insideTransitionShellTest = $true
                continue
            }
            if ($insideTransitionShellTest -and $line -match '^\s*name\s*=\s*"([^"]+_transition)"') {
                $name = $Matches[1]
                if ([string]::IsNullOrWhiteSpace($packagePath)) {
                    $targets.Add("//:$name")
                } else {
                    $targets.Add("//${packagePath}:$name")
                }
            }
            if ($insideTransitionShellTest -and $line -match '^\s*\),?\s*$') {
                $insideTransitionShellTest = $false
            }
        }
    }
    $targets.ToArray()
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
foreach ($entry in $policyEntries) {
    $context = "transition policy"
    $target = Get-RequiredString -Object $entry -Name "bazel_target" -Context $context
    if ($target -notmatch '^//[A-Za-z0-9_./-]*:[A-Za-z0-9_.-]+_transition$') {
        throw "transition policy target must be a Bazel _transition label: $target"
    }
    [void] (Get-RequiredString -Object $entry -Name "category" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "owner" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "reason" -Context "transition policy $target")
    [void] (Get-RequiredString -Object $entry -Name "exit_target" -Context "transition policy $target")
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
    if ([string] $entry.category -eq "external-advisory-sca" -and !$externalCollection) {
        throw "external advisory collection transition must require approval: $target"
    }
    $policyByTarget[$target] = $entry
}

$actualTargets = @(Get-BuildTransitionTargets)
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

$ciReferences = @(Get-CiTransitionReferences)
foreach ($target in $ciReferences) {
    if ($retiredTransitionTargetSet.ContainsKey($target)) {
        throw "CI references retired transition target: $target"
    }
    if (!$policyByTarget.ContainsKey($target)) {
        throw "CI references transition target without policy: $target"
    }
}

Write-Host "bazel-transition-ratchet-ok targets=$($actualTargets.Count) ci_refs=$($ciReferences.Count)"
