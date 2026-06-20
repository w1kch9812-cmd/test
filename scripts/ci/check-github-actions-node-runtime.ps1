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

$requiredNode24ActionPins = @{
    "actions/attest" = "281a49d4cbb0a72c9575a50d18f6deb515a11deb"
    "actions/checkout" = "de0fac2e4500dabe0009e67214ff5f5447ce83dd"
    "actions/download-artifact" = "3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c"
    "actions/setup-node" = "48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e"
    "actions/upload-artifact" = "043fb46d1a93c77aae656e7c1c64a875d1fc6a0a"
    "gitleaks/gitleaks-action" = "e0c47f4f8be36e29cdc102c57e68cb5cbf0e8d1e"
    "pnpm/action-setup" = "0e279bb959325dab635dd2c09392533439d90093"
    "Swatinem/rust-cache" = "c19371144df3bb44fab255c43d04cbc2ab54d1c4"
}
$usesPattern = "(?m)^\s*(?:-\s*)?uses:\s*([A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+)@([0-9a-f]{40})\b"
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
    foreach ($match in [regex]::Matches($content, $usesPattern)) {
        $action = $match.Groups[1].Value
        $sha = $match.Groups[2].Value
        if ($requiredNode24ActionPins.ContainsKey($action)) {
            $expectedSha = $requiredNode24ActionPins[$action]
            if ($sha -ne $expectedSha) {
                throw "workflow $relativePath uses $action@$sha; must use Node 24 native action pin $expectedSha"
            }
        }
    }
}

$ciWorkflowPath = Resolve-RepoPath -RelativePath ".github/workflows/ci.yml"
$ciWorkflow = Read-TextFile -Path $ciWorkflowPath
if (!$ciWorkflow.Contains("./scripts/ci/check-github-actions-node-runtime.ps1")) {
    throw "CI workflow must run check-github-actions-node-runtime.ps1"
}

Write-Host "github-actions-node-runtime-ok workflows=$($workflowFiles.Count)"
