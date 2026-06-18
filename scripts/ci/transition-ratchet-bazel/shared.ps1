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
