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

function Get-RelativePath {
    param([string] $Path)

    $root = $resolvedRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    )
    $fullPath = [System.IO.Path]::GetFullPath($Path)
    if (!$fullPath.StartsWith($root, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "path is outside repo root: $Path"
    }
    $fullPath.Substring($root.Length).TrimStart(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) -replace "\\", "/"
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

function Get-LineCount {
    param([string] $Path)

    (Get-Content -LiteralPath $Path | Measure-Object -Line).Lines
}

function Assert-String {
    param([object] $Value, [string] $Message)

    if ([string]::IsNullOrWhiteSpace([string] $Value)) {
        throw "$Message must be set"
    }
}

function Assert-PositiveInteger {
    param([object] $Value, [string] $Message)

    if ($Value -isnot [int] -and $Value -isnot [long]) {
        throw "$Message must be an integer"
    }
    if ([int] $Value -le 0) {
        throw "$Message must be positive"
    }
}

function Test-IgnoredPath {
    param([string] $RelativePath)

    foreach ($segment in @(
        ".git/",
        "bazel-",
        "node_modules/",
        "target/",
        ".next/",
        ".turbo/",
        ".wrangler/",
        "_archive/",
        "reference/"
    )) {
        if ($RelativePath.Contains($segment)) {
            return $true
        }
    }
    $false
}

function Get-SourceFiles {
    param([string] $RelativePath)

    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (Test-Path -LiteralPath $path -PathType Leaf) {
        return @([pscustomobject]@{
            FullName = $path
            RelativePath = $RelativePath -replace "\\", "/"
        })
    }
    if (!(Test-Path -LiteralPath $path -PathType Container)) {
        throw "source path is missing: $RelativePath"
    }
    @(
        Get-ChildItem -LiteralPath $path -File -Recurse |
            Sort-Object FullName |
            ForEach-Object {
                [pscustomobject]@{
                    FullName = $_.FullName
                    RelativePath = Get-RelativePath -Path $_.FullName
                }
            }
    )
}

function Test-LargeGeneratedJsonCandidate {
    param([string] $RelativePath, [int] $LineCount)

    if ($LineCount -le 500) {
        return $false
    }
    if ($RelativePath.EndsWith(".generated.json")) {
        return $true
    }
    $RelativePath -eq "docs/architecture/traffic-auth-policy-registry.v1.json"
}

$registryPath = "docs/architecture/generated-artifacts.v1.json"
$registryFile = Resolve-RepoPath -RelativePath $registryPath
if (!(Test-Path -LiteralPath $registryFile -PathType Leaf)) {
    throw "generated artifact registry is missing: $registryPath"
}

$registry = Read-JsonFile -RelativePath $registryPath
if ([string] $registry.schema_version -ne "gongzzang.generated_artifacts.v1") {
    throw "generated artifact registry schema_version mismatch"
}
if ([string] $registry.repo_slug -ne "gongzzang") {
    throw "generated artifact registry repo_slug mismatch"
}

$artifacts = @($registry.artifacts)
if ($artifacts.Count -eq 0) {
    throw "generated artifact registry must declare artifacts"
}

$registeredPaths = @{}
$artifactCount = 0
$sourceCount = 0
foreach ($artifact in $artifacts) {
    $path = [string] $artifact.path
    Assert-String -Value $path -Message "artifact.path"
    if ($registeredPaths.ContainsKey($path)) {
        throw "generated artifact registry duplicate path: $path"
    }
    $registeredPaths[$path] = $true

    Assert-String -Value $artifact.kind -Message "artifact.kind"
    Assert-String -Value $artifact.owner -Message "artifact.owner"
    Assert-String -Value $artifact.reason -Message "artifact.reason"
    Assert-String -Value $artifact.generator -Message "artifact.generator"
    Assert-String -Value $artifact.verifier -Message "artifact.verifier"
    Assert-PositiveInteger -Value $artifact.max_artifact_lines -Message "artifact.max_artifact_lines"
    Assert-PositiveInteger -Value $artifact.max_source_lines -Message "artifact.max_source_lines"

    $artifactFile = Resolve-RepoPath -RelativePath $path
    if (!(Test-Path -LiteralPath $artifactFile -PathType Leaf)) {
        throw "generated artifact is missing: $path"
    }
    $generatorFile = Resolve-RepoPath -RelativePath ([string] $artifact.generator)
    if (!(Test-Path -LiteralPath $generatorFile -PathType Leaf)) {
        throw "generator is missing for ${path}: $($artifact.generator)"
    }
    $verifierFile = Resolve-RepoPath -RelativePath ([string] $artifact.verifier)
    if (!(Test-Path -LiteralPath $verifierFile -PathType Leaf)) {
        throw "verifier is missing for ${path}: $($artifact.verifier)"
    }

    $artifactLines = Get-LineCount -Path $artifactFile
    if ($artifactLines -gt [int] $artifact.max_artifact_lines) {
        throw "generated artifact exceeds max_artifact_lines: $path lines=$artifactLines max=$($artifact.max_artifact_lines)"
    }

    $sourcePaths = @($artifact.source_paths)
    if ($sourcePaths.Count -eq 0) {
        throw "artifact.source_paths must not be empty: $path"
    }
    foreach ($sourcePath in $sourcePaths) {
        foreach ($sourceFile in Get-SourceFiles -RelativePath ([string] $sourcePath)) {
            $sourceLines = Get-LineCount -Path $sourceFile.FullName
            if ($sourceLines -gt [int] $artifact.max_source_lines) {
                throw "source file exceeds max_source_lines: $($sourceFile.RelativePath) lines=$sourceLines max=$($artifact.max_source_lines)"
            }
            $sourceCount += 1
        }
    }
    $artifactCount += 1
}

$jsonFiles = @(
    Get-ChildItem -LiteralPath $resolvedRoot -File -Recurse -Filter "*.json" |
        Sort-Object FullName
)
foreach ($file in $jsonFiles) {
    $relativePath = Get-RelativePath -Path $file.FullName
    if (Test-IgnoredPath -RelativePath $relativePath) {
        continue
    }
    $lineCount = Get-LineCount -Path $file.FullName
    if ((Test-LargeGeneratedJsonCandidate -RelativePath $relativePath -LineCount $lineCount) -and
        !$registeredPaths.ContainsKey($relativePath)) {
        throw "large generated JSON artifact must be registered: $relativePath lines=$lineCount"
    }
}

Write-Host "generated-artifact-registry-ok artifacts=$artifactCount sources=$sourceCount"
