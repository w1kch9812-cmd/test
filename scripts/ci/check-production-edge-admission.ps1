[CmdletBinding()]
param(
    [string] $Root = "",

    [ValidateSet("REGIONAL", "CLOUDFRONT")]
    [string] $EdgeAttachmentMode = $env:GONGZZANG_EDGE_ATTACHMENT_MODE,

    [string] $WafRegionalResourceArn = $env:GONGZZANG_WAF_REGIONAL_RESOURCE_ARN,

    [string] $CloudFrontDistributionId = $env:GONGZZANG_CLOUDFRONT_DISTRIBUTION_ID,

    [switch] $RequirePulumiAssociationPreview,

    [string] $PulumiPreviewScript = "scripts/ci/check-pulumi-local-preview.ps1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($Root)) {
    $Root = Join-Path $PSScriptRoot "..\.."
}
if ([string]::IsNullOrWhiteSpace($EdgeAttachmentMode)) {
    $EdgeAttachmentMode = "REGIONAL"
}

$resolvedRoot = [System.IO.Path]::GetFullPath($Root)
if (!(Test-Path -LiteralPath $resolvedRoot -PathType Container)) {
    throw "Root does not exist: $resolvedRoot"
}

function Resolve-RepoPath {
    param([string] $RelativePath)
    return [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot ($RelativePath -replace "/", "\")))
}

function Read-TextFile {
    param([string] $RelativePath)
    $path = Resolve-RepoPath -RelativePath $RelativePath
    if (!(Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required file is missing: $RelativePath"
    }
    return Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

function Assert-Contains {
    param([string] $Content, [string] $Needle, [string] $Message)
    if (!$Content.Contains($Needle)) {
        throw "$Message missing '$Needle'"
    }
}

function Assert-ValidRegionalResourceArn {
    param([string] $Arn)
    if ([string]::IsNullOrWhiteSpace($Arn)) {
        throw "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN must be set for REGIONAL production edge admission."
    }
    if ($Arn -notmatch "^arn:(aws|aws-us-gov|aws-cn):[a-z0-9-]+:[a-z0-9-]+:[0-9]{12}:.+") {
        throw "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN must be an AWS ARN."
    }
    if ($Arn.Contains("*")) {
        throw "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN must not contain wildcards."
    }
    $supportedTargetPatterns = @(
        ":elasticloadbalancing:",
        ":apigateway:",
        ":appsync:",
        ":cognito-idp:",
        ":apprunner:",
        ":verified-access-instance/"
    )
    foreach ($pattern in $supportedTargetPatterns) {
        if ($Arn.Contains($pattern)) {
            return
        }
    }
    throw "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN must target a WAFv2 regional association resource."
}

function Invoke-PulumiAssociationPreview {
    param([string] $PreviewScript, [string] $RegionalArn)
    $previewScriptPath = if ([System.IO.Path]::IsPathRooted($PreviewScript)) {
        [System.IO.Path]::GetFullPath($PreviewScript)
    } else {
        Resolve-RepoPath -RelativePath $PreviewScript
    }
    if (!(Test-Path -LiteralPath $previewScriptPath -PathType Leaf)) {
        throw "Pulumi association preview script missing: $PreviewScript"
    }
    $powerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }
    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        $previewScriptPath,
        "-Root",
        $resolvedRoot,
        "-WafRegionalResourceArn",
        $RegionalArn,
        "-ExpectRegionalAssociation"
    )
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $previewOutput = & $powerShellExe @arguments 2>&1
    $previewExitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    $previewText = $previewOutput -join [Environment]::NewLine
    Write-Host $previewText
    if ($previewExitCode -ne 0) {
        throw "Pulumi association preview failed"
    }
    if (!$previewText.Contains("regional_association=planned")) {
        throw "Pulumi association preview missing 'regional_association=planned'"
    }
    Write-Host "production-edge-association-preview-ok"
}

$trafficRegistry = Read-TextFile -RelativePath "docs/architecture/traffic-auth-policy-registry.v1.json"
$wafManifest = Read-TextFile -RelativePath "infrastructure/security/aws-wafv2-edge-policy.generated.json"
$pulumiProject = Read-TextFile -RelativePath "infrastructure/Pulumi.yaml"
$pulumiProgram = Read-TextFile -RelativePath "infrastructure/index.ts"

Assert-Contains `
    -Content $trafficRegistry `
    -Needle "gongzzang.traffic_auth_policy_registry.v1" `
    -Message "traffic/auth registry"
Assert-Contains `
    -Content $wafManifest `
    -Needle "gongzzang.aws_wafv2_edge_policy_manifest.v1" `
    -Message "AWS WAFv2 generated manifest"
Assert-Contains -Content $pulumiProject -Needle "runtime: nodejs" -Message "Pulumi project"
Assert-Contains -Content $pulumiProgram -Needle "wafRegionalResourceArn" -Message "Pulumi regional WAF config"
Assert-Contains -Content $pulumiProgram -Needle "aws.wafv2.WebAclAssociation" -Message "Pulumi regional WAF association"

switch ($EdgeAttachmentMode) {
    "REGIONAL" {
        Assert-ValidRegionalResourceArn -Arn $WafRegionalResourceArn
        if ($RequirePulumiAssociationPreview) {
            Invoke-PulumiAssociationPreview `
                -PreviewScript $PulumiPreviewScript `
                -RegionalArn $WafRegionalResourceArn
        }
        Write-Host "production-edge-admission-ok mode=REGIONAL target=$WafRegionalResourceArn"
    }
    "CLOUDFRONT" {
        if ([string]::IsNullOrWhiteSpace($CloudFrontDistributionId)) {
            throw "GONGZZANG_CLOUDFRONT_DISTRIBUTION_ID must be set for CLOUDFRONT production edge admission."
        }
        throw "CloudFront production attachment is not wired in Pulumi yet. Wire the generated WebACL ARN into the CloudFront distribution module before using CLOUDFRONT mode."
    }
    default {
        throw "Unsupported production edge attachment mode: $EdgeAttachmentMode"
    }
}
