param(
    [string] $Root = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-RepoPath {
    param([string] $RelativePath)

    [System.IO.Path]::GetFullPath((Join-Path $Root $RelativePath))
}

function Read-TextFile {
    param([string] $Path)

    if (!(Test-Path -LiteralPath $Path -PathType Leaf)) {
        throw "missing file: $Path"
    }
    Get-Content -LiteralPath $Path -Raw -Encoding UTF8
}

$workflowRoot = Resolve-RepoPath -RelativePath ".github/workflows"
if (!(Test-Path -LiteralPath $workflowRoot -PathType Container)) {
    throw "missing workflow directory: .github/workflows"
}

$workflowFiles = @(
    Get-ChildItem -LiteralPath $workflowRoot -File |
        Where-Object { $_.Extension -in @(".yml", ".yaml") } |
        Sort-Object Name
)
if ($workflowFiles.Count -eq 0) {
    throw "missing GitHub Actions workflows under .github/workflows"
}

$node24Pattern = "(?m)^\s*FORCE_JAVASCRIPT_ACTIONS_TO_NODE24\s*:\s*['""]?true['""]?\s*(?:#.*)?$"
foreach ($workflowFile in $workflowFiles) {
    $content = Read-TextFile -Path $workflowFile.FullName
    $relativePath = ".github/workflows/$($workflowFile.Name)"

    if ($content.Contains("ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION")) {
        throw "workflow $relativePath must not set ACTIONS_ALLOW_USE_UNSECURE_NODE_VERSION"
    }
    if ($content -notmatch $node24Pattern) {
        throw "workflow $relativePath missing FORCE_JAVASCRIPT_ACTIONS_TO_NODE24 true opt-in"
    }
}

$ciWorkflowPath = Resolve-RepoPath -RelativePath ".github/workflows/ci.yml"
$ciWorkflow = Read-TextFile -Path $ciWorkflowPath
if (!$ciWorkflow.Contains("./scripts/ci/check-github-actions-node-runtime.ps1")) {
    throw "CI workflow must run check-github-actions-node-runtime.ps1"
}
if (!$ciWorkflow.Contains("./scripts/ci/check-github-actions-node-runtime.tests.ps1")) {
    throw "CI workflow must run check-github-actions-node-runtime.tests.ps1"
}

Write-Host "github-actions-node-runtime-ok workflows=$($workflowFiles.Count)"
