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
        throw "Required file is missing: $RelativePath"
    }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Read-JsonFile {
    param([string] $RelativePath)
    (Read-TextFile -RelativePath $RelativePath) | ConvertFrom-Json
}

function Assert-NotEmptyString {
    param([object] $Value, [string] $Message)
    if ([string]::IsNullOrWhiteSpace([string] $Value)) {
        throw "$Message must be set"
    }
}

function Test-ScopedPath {
    param([string] $RelativePath, [string] $Scope)
    $pattern = [System.Management.Automation.WildcardPattern]::new(
        ($Scope -replace "\\", "/"),
        [System.Management.Automation.WildcardOptions]::IgnoreCase
    )
    $pattern.IsMatch(($RelativePath -replace "\\", "/"))
}

function Get-InspectedFiles {
    $files = @()
    $workflowRoot = Resolve-RepoPath -RelativePath ".github/workflows"
    if (Test-Path -LiteralPath $workflowRoot -PathType Container) {
        $files += Get-ChildItem -LiteralPath $workflowRoot -File |
            Where-Object { $_.Extension -in @(".yml", ".yaml") } |
            ForEach-Object {
                [pscustomobject]@{
                    FullName = $_.FullName
                    RelativePath = ".github/workflows/$($_.Name)"
                }
            }
    }

    $lefthookPath = Resolve-RepoPath -RelativePath "lefthook.yml"
    if (Test-Path -LiteralPath $lefthookPath -PathType Leaf) {
        $files += [pscustomobject]@{
            FullName = $lefthookPath
            RelativePath = "lefthook.yml"
        }
    }

    $files | Sort-Object RelativePath
}

function Get-CommandLines {
    param([object] $File)

    $lines = Get-Content -LiteralPath $File.FullName -Encoding UTF8
    $commands = @()
    $inRunBlock = $false
    $runIndent = 0

    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = [string] $lines[$index]
        $trimmed = $line.Trim()
        $indent = $line.Length - $line.TrimStart().Length

        if ($inRunBlock) {
            if ($trimmed -eq "") {
                continue
            }
            if ($indent -gt $runIndent) {
                $commands += [pscustomobject]@{
                    File = $File.RelativePath
                    LineNumber = $index + 1
                    Command = $trimmed
                }
                continue
            }
            $inRunBlock = $false
        }

        if ($line -match "^\s*(?:-\s*)?run:\s*(.*)$") {
            $value = $Matches[1].Trim()
            if ($value -in @("|", ">")) {
                $inRunBlock = $true
                $runIndent = $indent
                continue
            }
            if ($value -ne "") {
                $commands += [pscustomobject]@{
                    File = $File.RelativePath
                    LineNumber = $index + 1
                    Command = $value
                }
            }
        }
    }

    $commands
}

function Test-AllowedCommand {
    param([object] $Command, [object[]] $AllowedCommands)

    foreach ($allowed in $AllowedCommands) {
        Assert-NotEmptyString -Value $allowed.id -Message "allowed_direct_commands.id"
        Assert-NotEmptyString -Value $allowed.pattern -Message "allowed_direct_commands.pattern"
        Assert-NotEmptyString -Value $allowed.scope -Message "allowed_direct_commands.scope"
        Assert-NotEmptyString -Value $allowed.owner -Message "allowed_direct_commands.owner"
        Assert-NotEmptyString -Value $allowed.reason -Message "allowed_direct_commands.reason"
        Assert-NotEmptyString -Value $allowed.exit_target -Message "allowed_direct_commands.exit_target"
        Assert-NotEmptyString -Value $allowed.sunset -Message "allowed_direct_commands.sunset"

        $sunset = [DateTimeOffset]::Parse(
            [string] $allowed.sunset,
            [System.Globalization.CultureInfo]::InvariantCulture
        )
        if ($sunset.UtcDateTime.Date -lt [DateTimeOffset]::UtcNow.Date) {
            throw "allowed_direct_commands '$($allowed.id)' sunset '$($allowed.sunset)' is in the past"
        }

        if ((Test-ScopedPath -RelativePath $Command.File -Scope ([string] $allowed.scope)) -and
            $Command.Command -match ([string] $allowed.pattern)) {
            return $true
        }
    }

    return $false
}

$policyPath = "docs/architecture/verification-control-plane.v1.json"
$policy = Read-JsonFile -RelativePath $policyPath
if ($policy.schema_version -ne 1) {
    throw "verification control plane schema_version must be 1"
}

$forbiddenCommands = @($policy.forbidden_direct_verification_commands)
$allowedCommands = @($policy.allowed_direct_commands)
if ($forbiddenCommands.Count -eq 0) {
    throw "verification control plane must define forbidden_direct_verification_commands"
}

$files = @(Get-InspectedFiles)
if ($files.Count -eq 0) {
    throw "verification control plane found no workflow or lefthook files to inspect"
}

$violations = @()
$allowlisted = 0
foreach ($file in $files) {
    foreach ($command in Get-CommandLines -File $file) {
        if (Test-AllowedCommand -Command $command -AllowedCommands $allowedCommands) {
            $allowlisted += 1
            continue
        }

        foreach ($forbidden in $forbiddenCommands) {
            Assert-NotEmptyString -Value $forbidden.id -Message "forbidden_direct_verification_commands.id"
            Assert-NotEmptyString -Value $forbidden.pattern -Message "forbidden_direct_verification_commands.pattern"
            if ($command.Command -match ([string] $forbidden.pattern)) {
                $violations += [pscustomobject]@{
                    Id = [string] $forbidden.id
                    File = $command.File
                    LineNumber = $command.LineNumber
                    Command = $command.Command
                }
            }
        }
    }
}

foreach ($violation in $violations) {
    Write-Error (
        "verification-control-plane: forbidden direct verification command " +
        "$($violation.Id) in $($violation.File):$($violation.LineNumber): $($violation.Command)"
    )
}

if ($violations.Count -gt 0) {
    exit 1
}

Write-Host "verification-control-plane-ok files=$($files.Count) allowlisted=$allowlisted"
