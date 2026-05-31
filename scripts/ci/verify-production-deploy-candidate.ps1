[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $ArtifactPath,

    [string] $Repository = $env:GITHUB_REPOSITORY,

    [string] $RequiredWorkflow = ".github/workflows/ci.yml",

    [string] $RequiredRef = "refs/heads/main",

    [string] $PredicateType = ""
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Repository)) {
    throw "Repository is required. Pass -Repository owner/gongzzang or set GITHUB_REPOSITORY."
}

$resolvedArtifact = [System.IO.Path]::GetFullPath($ArtifactPath)
if (!(Test-Path -LiteralPath $resolvedArtifact -PathType Leaf)) {
    throw "Deploy candidate artifact does not exist: $ArtifactPath"
}

$gh = Get-Command gh -ErrorAction SilentlyContinue
if ($null -eq $gh) {
    throw "GitHub CLI 'gh' is required to verify artifact attestations."
}

$arguments = @(
    "attestation",
    "verify",
    $resolvedArtifact,
    "-R",
    $Repository,
    "--format",
    "json"
)

if (![string]::IsNullOrWhiteSpace($PredicateType)) {
    $arguments += @("--predicate-type", $PredicateType)
}

$previousErrorActionPreference = $ErrorActionPreference
$ErrorActionPreference = "Continue"
$output = & gh @arguments 2>&1
$exitCode = $LASTEXITCODE
$ErrorActionPreference = $previousErrorActionPreference

if ($exitCode -ne 0) {
    throw "Artifact attestation verification failed for '$ArtifactPath': $($output -join [Environment]::NewLine)"
}

$json = ($output -join [Environment]::NewLine) | ConvertFrom-Json
$jsonText = $json | ConvertTo-Json -Depth 100

if (!$jsonText.Contains($RequiredWorkflow)) {
    throw "Deploy candidate was not built by approved workflow '$RequiredWorkflow'."
}

if (!$jsonText.Contains($RequiredRef)) {
    throw "Deploy candidate was not built from approved ref '$RequiredRef'."
}

Write-Host "production-deploy-candidate-ok artifact=$ArtifactPath repository=$Repository"
