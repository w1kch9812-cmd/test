function Write-TrafficAuthPulumiFixtures {
    if (!$OmitPulumiWafConsumer) {
        $pulumiCliDependency = if ($OmitPulumiCliPackage) { "" } else { ', "pulumi": "3.244.0"' }
        Write-File -Root $Root -RelativePath "infrastructure\package.json" -Content @"
{
  "name": "@gongzzang/infrastructure",
  "private": true,
  "dependencies": {
    "@pulumi/aws": "7.31.0",
    "@pulumi/pulumi": "3.244.0"$pulumiCliDependency
  }
}
"@
        Write-File -Root $Root -RelativePath "infrastructure\Pulumi.yaml" -Content @'
name: gongzzang-infrastructure
runtime: nodejs
description: Gongzzang Pulumi infrastructure.
'@
        if (!$OmitPulumiLocalPreviewStack) {
            $pulumiLocalPreviewContent = @'
encryptionsalt: local-preview-test-salt
config:
  aws:region: ap-northeast-2
  aws:skipCredentialsValidation: "true"
  aws:skipRequestingAccountId: "true"
  aws:skipMetadataApiCheck: "true"
'@
            if ($PollutePulumiLocalPreviewStack) {
                $pulumiLocalPreviewContent += "`n  gongzzang-infrastructure:wafRegionalResourceArn: arn:aws:elasticloadbalancing:ap-northeast-2:123456789012:loadbalancer/app/gongzzang-ci/50dc6c495c0c9188"
            }
            Write-File -Root $Root -RelativePath "infrastructure\Pulumi.local-preview.yaml" -Content $pulumiLocalPreviewContent
        }
        $pulumiWafAssociationCode = if ($OmitPulumiWafAssociation) {
            ""
        } else {
            @'

new aws.wafv2.WebAclAssociation("gongzzang-edge-waf-regional-association", {
  resourceArn: wafRegionalResourceArn,
  webAclArn: "awsWafv2WebAclArn",
});
'@
        }
        Write-File -Root $Root -RelativePath "infrastructure\index.ts" -Content @"
import * as aws from "@pulumi/aws";

const manifestPath = "security/aws-wafv2-edge-policy.generated.json";
const wafRegionalResourceArn = "wafRegionalResourceArn";
const previewWafRegionalResourceArn = "GONGZZANG_WAF_REGIONAL_RESOURCE_ARN";
const rateBasedRules = "rate_based_rules";
const blockedQueryShapeRules = "blocked_query_shape_rules";
const identityAwareApplicationRules = "identity_aware_application_rules";
const serviceIdentityRules = "service_identity_rules";

new aws.wafv2.WebAcl("gongzzang-edge-waf", {
  scope: "REGIONAL",
  defaultAction: { allow: {} },
  visibilityConfig: {
    cloudwatchMetricsEnabled: true,
    metricName: "gongzzang-edge-waf",
    sampledRequestsEnabled: true,
  },
  rules: [
    manifestPath,
    rateBasedRules,
    blockedQueryShapeRules,
    identityAwareApplicationRules,
    serviceIdentityRules,
  ].map((name, priority) => ({
    name,
    priority,
    action: { count: {} },
    statement: { rateBasedStatement: { aggregateKeyType: "IP", limit: 100 } },
    visibilityConfig: {
      cloudwatchMetricsEnabled: true,
      metricName: name,
      sampledRequestsEnabled: true,
    },
  })),
});
$pulumiWafAssociationCode
"@
    }
}
