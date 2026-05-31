[CmdletBinding()]
param(
    [string] $Root = "",

    [string] $WafRegionalResourceArn = "",

    [switch] $ExpectRegionalAssociation
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

$targetRoot = [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot "target"))
$stateDir = [System.IO.Path]::GetFullPath((
        Join-Path `
            (Join-Path $targetRoot "pulumi-local-preview-state") `
            ([Guid]::NewGuid().ToString("N"))
    ))
if (!$stateDir.StartsWith($targetRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Pulumi local preview state path escaped target directory: $stateDir"
}
New-Item -ItemType Directory -Force -Path $stateDir | Out-Null

if ($ExpectRegionalAssociation -and [string]::IsNullOrWhiteSpace($WafRegionalResourceArn)) {
    throw "WafRegionalResourceArn must be set when ExpectRegionalAssociation is requested."
}

$backendPath = ($stateDir -replace "\\", "/")
$env:PULUMI_BACKEND_URL = "file://$backendPath"
if ([string]::IsNullOrWhiteSpace($env:PULUMI_CONFIG_PASSPHRASE)) {
    $env:PULUMI_CONFIG_PASSPHRASE = "local-preview-passphrase"
}
if ([string]::IsNullOrWhiteSpace($env:AWS_ACCESS_KEY_ID)) {
    $env:AWS_ACCESS_KEY_ID = "preview"
}
if ([string]::IsNullOrWhiteSpace($env:AWS_SECRET_ACCESS_KEY)) {
    $env:AWS_SECRET_ACCESS_KEY = "preview"
}
$env:AWS_EC2_METADATA_DISABLED = "true"
$previousWafRegionalResourceArn = $env:GONGZZANG_WAF_REGIONAL_RESOURCE_ARN
if ([string]::IsNullOrWhiteSpace($WafRegionalResourceArn)) {
    Remove-Item Env:\GONGZZANG_WAF_REGIONAL_RESOURCE_ARN -ErrorAction SilentlyContinue
} else {
    $env:GONGZZANG_WAF_REGIONAL_RESOURCE_ARN = $WafRegionalResourceArn
}

Push-Location $resolvedRoot
try {
    pnpm --filter "@gongzzang/infrastructure" exec pulumi login $env:PULUMI_BACKEND_URL
    if ($LASTEXITCODE -ne 0) {
        throw "pulumi login failed"
    }
    pnpm --filter "@gongzzang/infrastructure" exec pulumi stack init local-preview --non-interactive
    if ($LASTEXITCODE -ne 0) {
        throw "pulumi stack init failed"
    }
    $previewArguments = @(
        "--filter",
        "@gongzzang/infrastructure",
        "exec",
        "pulumi",
        "preview",
        "--stack",
        "local-preview",
        "--non-interactive"
    )
    $previewOutput = pnpm @previewArguments 2>&1
    $previewExitCode = $LASTEXITCODE
    $previewText = $previewOutput -join [Environment]::NewLine
    Write-Host $previewText
    if ($previewExitCode -ne 0) {
        throw "pulumi preview failed"
    }
    if ($previewText -match "\bwarning:") {
        throw "pulumi preview produced warnings"
    }
    if ($ExpectRegionalAssociation) {
        if (!$previewText.Contains("aws:wafv2:WebAclAssociation") -or !$previewText.Contains("gongzzang-edge-waf-regional-association")) {
            throw "pulumi preview did not plan regional WebAclAssociation"
        }
        Write-Host "pulumi-local-preview-ok stack=local-preview regional_association=planned"
    } else {
        Write-Host "pulumi-local-preview-ok stack=local-preview"
    }
} finally {
    Pop-Location
    if ([string]::IsNullOrWhiteSpace($previousWafRegionalResourceArn)) {
        Remove-Item Env:\GONGZZANG_WAF_REGIONAL_RESOURCE_ARN -ErrorAction SilentlyContinue
    } else {
        $env:GONGZZANG_WAF_REGIONAL_RESOURCE_ARN = $previousWafRegionalResourceArn
    }
}
