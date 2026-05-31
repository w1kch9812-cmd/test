Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$ScriptPath = Join-Path $PSScriptRoot "check-production-edge-admission.ps1"
$RepoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\.."))
$TempRoot = Join-Path `
    (Join-Path $RepoRoot "target\check-production-edge-admission-tests") `
    ([Guid]::NewGuid().ToString("N"))
$PowerShellExe = if ($PSVersionTable.PSEdition -eq "Core") { "pwsh" } else { "powershell.exe" }

function Write-File {
    param([string] $Root, [string] $RelativePath, [string] $Content)
    $path = Join-Path $Root $RelativePath
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
    Set-Content -LiteralPath $path -Encoding UTF8 -Value $Content
}

function Invoke-Admission {
    param(
        [string] $Root,
        [string] $Mode = "REGIONAL",
        [string] $RegionalArn = "",
        [string] $CloudFrontDistributionId = "",
        [switch] $RequirePulumiAssociationPreview,
        [string] $PulumiPreviewScript = ""
    )
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $arguments = @(
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        $ScriptPath,
        "-Root",
        $Root,
        "-EdgeAttachmentMode",
        $Mode
    )
    if (![string]::IsNullOrWhiteSpace($RegionalArn)) {
        $arguments += @("-WafRegionalResourceArn", $RegionalArn)
    }
    if (![string]::IsNullOrWhiteSpace($CloudFrontDistributionId)) {
        $arguments += @("-CloudFrontDistributionId", $CloudFrontDistributionId)
    }
    if ($RequirePulumiAssociationPreview) {
        $arguments += "-RequirePulumiAssociationPreview"
    }
    if (![string]::IsNullOrWhiteSpace($PulumiPreviewScript)) {
        $arguments += @("-PulumiPreviewScript", $PulumiPreviewScript)
    }
    $output = & $PowerShellExe @arguments 2>&1
    $ErrorActionPreference = $previousErrorActionPreference
    [pscustomobject]@{
        ExitCode = $LASTEXITCODE
        Output   = ($output -join [Environment]::NewLine)
    }
}

function Assert-Equals {
    param([object] $Actual, [object] $Expected, [string] $Message)
    if ($Actual -ne $Expected) {
        throw "$Message expected='$Expected' actual='$Actual'"
    }
}

function Assert-Contains {
    param([string] $Text, [string] $Expected)
    $compactText = $Text -replace "\s+", ""
    $compactExpected = $Expected -replace "\s+", ""
    if (!$Text.Contains($Expected) -and !$compactText.Contains($compactExpected)) {
        throw "Expected output to contain '$Expected'. Actual output: $Text"
    }
}

function Write-MinimalRepo {
    param(
        [string] $Root,
        [switch] $OmitWafManifest,
        [switch] $OmitAssociationSupport
    )

    Write-File -Root $Root -RelativePath "docs\architecture\traffic-auth-policy-registry.v1.json" -Content @'
{
  "schema_version": "gongzzang.traffic_auth_policy_registry.v1",
  "public_route_policies": [{"id":"gongzzang.public_map.listing_marker_tile"}]
}
'@
    if (!$OmitWafManifest) {
        Write-File -Root $Root -RelativePath "infrastructure\security\aws-wafv2-edge-policy.generated.json" -Content @'
{
  "schema_version": "gongzzang.aws_wafv2_edge_policy_manifest.v1",
  "managed_by": "pulumi",
  "rate_based_rules": []
}
'@
    }
    Write-File -Root $Root -RelativePath "infrastructure\Pulumi.yaml" -Content @'
name: gongzzang-infrastructure
runtime: nodejs
'@
    $associationSupport = if ($OmitAssociationSupport) {
        'const wafRegionalResourceArn = config.get("wafRegionalResourceArn");'
    } else {
        @'
const wafRegionalResourceArn = config.get("wafRegionalResourceArn");
new aws.wafv2.WebAclAssociation("gongzzang-edge-waf-regional-association", {
  resourceArn: wafRegionalResourceArn,
  webAclArn: webAcl.arn,
});
'@
    }
    Write-File -Root $Root -RelativePath "infrastructure\index.ts" -Content @"
import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";
const config = new pulumi.Config();
const webAcl = new aws.wafv2.WebAcl("gongzzang-edge-waf", {});
$associationSupport
"@
    Write-File -Root $Root -RelativePath "scripts\ci\mock-pulumi-association-preview.ps1" -Content @'
param(
    [string] $Root,
    [string] $WafRegionalResourceArn,
    [switch] $ExpectRegionalAssociation
)
if ([string]::IsNullOrWhiteSpace($Root)) { throw "Root missing" }
if ([string]::IsNullOrWhiteSpace($WafRegionalResourceArn)) { throw "WafRegionalResourceArn missing" }
if (!$ExpectRegionalAssociation) { throw "ExpectRegionalAssociation missing" }
Write-Host "pulumi-local-preview-ok stack=local-preview regional_association=planned"
'@
    Write-File -Root $Root -RelativePath "scripts\ci\mock-pulumi-no-association-preview.ps1" -Content @'
param(
    [string] $Root,
    [string] $WafRegionalResourceArn,
    [switch] $ExpectRegionalAssociation
)
Write-Host "pulumi-local-preview-ok stack=local-preview"
'@
}

New-Item -ItemType Directory -Force -Path $TempRoot | Out-Null

try {
    $successRoot = Join-Path $TempRoot "success"
    Write-MinimalRepo -Root $successRoot
    $validArn = "arn:aws:elasticloadbalancing:ap-northeast-2:123456789012:loadbalancer/app/gongzzang-prod/50dc6c495c0c9188"
    $success = Invoke-Admission -Root $successRoot -RegionalArn $validArn
    Assert-Equals $success.ExitCode 0 "successful admission exit code mismatch output=$($success.Output)"
    Assert-Contains $success.Output "production-edge-admission-ok mode=REGIONAL"

    $previewSuccessRoot = Join-Path $TempRoot "preview-success"
    Write-MinimalRepo -Root $previewSuccessRoot
    $previewSuccess = Invoke-Admission `
        -Root $previewSuccessRoot `
        -RegionalArn $validArn `
        -RequirePulumiAssociationPreview `
        -PulumiPreviewScript "scripts/ci/mock-pulumi-association-preview.ps1"
    Assert-Equals $previewSuccess.ExitCode 0 "preview admission exit code mismatch output=$($previewSuccess.Output)"
    Assert-Contains $previewSuccess.Output "production-edge-association-preview-ok"

    $previewMissingAssociationRoot = Join-Path $TempRoot "preview-missing-association"
    Write-MinimalRepo -Root $previewMissingAssociationRoot
    $previewMissingAssociation = Invoke-Admission `
        -Root $previewMissingAssociationRoot `
        -RegionalArn $validArn `
        -RequirePulumiAssociationPreview `
        -PulumiPreviewScript "scripts/ci/mock-pulumi-no-association-preview.ps1"
    Assert-Equals $previewMissingAssociation.ExitCode 1 "missing association preview exit code mismatch"
    Assert-Contains $previewMissingAssociation.Output "regional_association=planned"

    $missingArnRoot = Join-Path $TempRoot "missing-arn"
    Write-MinimalRepo -Root $missingArnRoot
    $missingArn = Invoke-Admission -Root $missingArnRoot
    Assert-Equals $missingArn.ExitCode 1 "missing ARN exit code mismatch"
    Assert-Contains $missingArn.Output "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN"

    $invalidArnRoot = Join-Path $TempRoot "invalid-arn"
    Write-MinimalRepo -Root $invalidArnRoot
    $invalidArn = Invoke-Admission -Root $invalidArnRoot -RegionalArn "not-an-arn"
    Assert-Equals $invalidArn.ExitCode 1 "invalid ARN exit code mismatch"
    Assert-Contains $invalidArn.Output "must be an AWS ARN"

    $missingManifestRoot = Join-Path $TempRoot "missing-manifest"
    Write-MinimalRepo -Root $missingManifestRoot -OmitWafManifest
    $missingManifest = Invoke-Admission -Root $missingManifestRoot -RegionalArn $validArn
    Assert-Equals $missingManifest.ExitCode 1 "missing WAF manifest exit code mismatch"
    Assert-Contains $missingManifest.Output "aws-wafv2-edge-policy.generated.json"

    $missingAssociationRoot = Join-Path $TempRoot "missing-association"
    Write-MinimalRepo -Root $missingAssociationRoot -OmitAssociationSupport
    $missingAssociation = Invoke-Admission -Root $missingAssociationRoot -RegionalArn $validArn
    Assert-Equals $missingAssociation.ExitCode 1 "missing association support exit code mismatch"
    Assert-Contains $missingAssociation.Output "WebAclAssociation"

    $cloudFrontRoot = Join-Path $TempRoot "cloudfront"
    Write-MinimalRepo -Root $cloudFrontRoot
    $cloudFront = Invoke-Admission -Root $cloudFrontRoot -Mode "CLOUDFRONT" -CloudFrontDistributionId "E1234567890ABC"
    Assert-Equals $cloudFront.ExitCode 1 "CloudFront admission exit code mismatch"
    Assert-Contains $cloudFront.Output "CloudFront production attachment is not wired"

    Write-Host "production-edge-admission-tests-ok"
} finally {
    if (Test-Path -LiteralPath $TempRoot) {
        Remove-Item -LiteralPath $TempRoot -Recurse -Force
    }
}
