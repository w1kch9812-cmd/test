import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import * as aws from "@pulumi/aws";
import * as pulumi from "@pulumi/pulumi";

type HttpMethod = "DELETE" | "GET" | "PATCH" | "POST" | "PUT";
type PathMatch = "EXACT" | "STARTS_WITH";

type AwsWafRateBasedRule = {
  readonly source_policy_id: string;
  readonly priority: number;
  readonly aggregate_key_type: "IP";
  readonly limit_per_5m: number;
  readonly match: {
    readonly path?: string;
    readonly path_source?: string;
    readonly path_match: PathMatch;
    readonly methods: readonly HttpMethod[];
  };
};

type AwsWafBlockedQueryShapeRule = {
  readonly source_policy_id: string;
  readonly priority: number;
  readonly action: "BLOCK";
  readonly match: {
    readonly path: string;
    readonly path_match: PathMatch;
    readonly query_parameters: readonly string[];
  };
};

type AwsWafEdgePolicyManifest = {
  readonly schema_version: "gongzzang.aws_wafv2_edge_policy_manifest.v1";
  readonly managed_by: "pulumi";
  readonly scope_options: readonly ("CLOUDFRONT" | "REGIONAL")[];
  readonly rate_based_rules: readonly AwsWafRateBasedRule[];
  readonly blocked_query_shape_rules: readonly AwsWafBlockedQueryShapeRule[];
  readonly identity_aware_application_rules: readonly {
    readonly source_policy_id: string;
    readonly reason: "key_strategy_not_representable_in_wafv2";
  }[];
  readonly service_identity_rules: readonly {
    readonly source_policy_id: string;
    readonly target_auth_method: string;
  }[];
};

const infrastructureRoot = path.dirname(fileURLToPath(import.meta.url));
const manifestPath = path.join(
  infrastructureRoot,
  "security",
  "aws-wafv2-edge-policy.generated.json",
);
const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8")) as AwsWafEdgePolicyManifest;

if (manifest.schema_version !== "gongzzang.aws_wafv2_edge_policy_manifest.v1") {
  throw new Error(`Unsupported WAFv2 edge manifest schema: ${manifest.schema_version}`);
}
if (manifest.managed_by !== "pulumi") {
  throw new Error(`WAFv2 edge manifest must be managed by Pulumi: ${manifest.managed_by}`);
}

const config = new pulumi.Config();
const wafScope = config.get("wafScope") ?? "REGIONAL";
if (!manifest.scope_options.includes(wafScope as "CLOUDFRONT" | "REGIONAL")) {
  throw new Error(`Unsupported WAFv2 scope: ${wafScope}`);
}
const wafRegionalResourceArn =
  config.get("wafRegionalResourceArn") ?? process.env.GONGZZANG_WAF_REGIONAL_RESOURCE_ARN;
if (wafRegionalResourceArn && wafScope !== "REGIONAL") {
  throw new Error("wafRegionalResourceArn can only be used with REGIONAL WAFv2 scope.");
}

function metricName(value: string): string {
  return value.replace(/[^A-Za-z0-9_-]/g, "-").slice(0, 128);
}

function pathMatchStatement(
  pathValue: string,
  pathMatch: PathMatch,
): aws.types.input.wafv2.WebAclRuleStatement {
  return {
    byteMatchStatement: {
      fieldToMatch: { uriPath: {} },
      positionalConstraint: pathMatch === "EXACT" ? "EXACTLY" : "STARTS_WITH",
      searchString: pathValue,
      textTransformations: [{ priority: 0, type: "NONE" }],
    },
  };
}

function methodMatchStatement(method: HttpMethod): aws.types.input.wafv2.WebAclRuleStatement {
  return {
    byteMatchStatement: {
      fieldToMatch: { method: {} },
      positionalConstraint: "EXACTLY",
      searchString: method,
      textTransformations: [{ priority: 0, type: "NONE" }],
    },
  };
}

function queryArgumentExistsStatement(
  queryParameter: string,
): aws.types.input.wafv2.WebAclRuleStatement {
  return {
    sizeConstraintStatement: {
      comparisonOperator: "GT",
      fieldToMatch: { singleQueryArgument: { name: queryParameter } },
      size: 0,
      textTransformations: [{ priority: 0, type: "NONE" }],
    },
  };
}

function andStatement(
  statements: aws.types.input.wafv2.WebAclRuleStatement[],
): aws.types.input.wafv2.WebAclRuleStatement {
  if (statements.length === 0) {
    throw new Error("WAFv2 andStatement requires at least one statement.");
  }
  if (statements.length === 1) {
    return statements[0] as aws.types.input.wafv2.WebAclRuleStatement;
  }
  return { andStatement: { statements } };
}

function orStatement(
  statements: aws.types.input.wafv2.WebAclRuleStatement[],
): aws.types.input.wafv2.WebAclRuleStatement {
  if (statements.length === 0) {
    throw new Error("WAFv2 orStatement requires at least one statement.");
  }
  if (statements.length === 1) {
    return statements[0] as aws.types.input.wafv2.WebAclRuleStatement;
  }
  return { orStatement: { statements } };
}

function rateBasedRule(rule: AwsWafRateBasedRule): aws.types.input.wafv2.WebAclRule {
  const pathValue = rule.match.path;
  if (!pathValue) {
    throw new Error(`WAFv2 rate rule path missing: ${rule.source_policy_id}`);
  }
  return {
    name: metricName(rule.source_policy_id),
    priority: rule.priority,
    action: { block: {} },
    statement: {
      rateBasedStatement: {
        aggregateKeyType: rule.aggregate_key_type,
        limit: rule.limit_per_5m,
        scopeDownStatement: andStatement([
          pathMatchStatement(pathValue, rule.match.path_match),
          orStatement(rule.match.methods.map((method) => methodMatchStatement(method))),
        ]),
      },
    },
    visibilityConfig: {
      cloudwatchMetricsEnabled: true,
      metricName: metricName(`rate-${rule.source_policy_id}`),
      sampledRequestsEnabled: true,
    },
  };
}

function blockedQueryShapeRule(
  rule: AwsWafBlockedQueryShapeRule,
): aws.types.input.wafv2.WebAclRule {
  return {
    name: metricName(`${rule.source_policy_id}-blocked-query-shape`),
    priority: rule.priority,
    action: { block: {} },
    statement: andStatement([
      pathMatchStatement(rule.match.path, rule.match.path_match),
      orStatement(
        rule.match.query_parameters.map((queryParameter) =>
          queryArgumentExistsStatement(queryParameter),
        ),
      ),
    ]),
    visibilityConfig: {
      cloudwatchMetricsEnabled: true,
      metricName: metricName(`shape-${rule.source_policy_id}`),
      sampledRequestsEnabled: true,
    },
  };
}

const webAcl = new aws.wafv2.WebAcl("gongzzang-edge-waf", {
  defaultAction: { allow: {} },
  description: "Generated from infrastructure/security/aws-wafv2-edge-policy.generated.json.",
  rules: [
    ...manifest.rate_based_rules.map((rule) => rateBasedRule(rule)),
    ...manifest.blocked_query_shape_rules.map((rule) => blockedQueryShapeRule(rule)),
  ],
  scope: wafScope,
  visibilityConfig: {
    cloudwatchMetricsEnabled: true,
    metricName: "gongzzang-edge-waf",
    sampledRequestsEnabled: true,
  },
});

const regionalAssociation = wafRegionalResourceArn
  ? new aws.wafv2.WebAclAssociation("gongzzang-edge-waf-regional-association", {
      resourceArn: wafRegionalResourceArn,
      webAclArn: webAcl.arn,
    })
  : undefined;

export const awsWafv2IdentityAwareApplicationRuleIds =
  manifest.identity_aware_application_rules.map((rule) => rule.source_policy_id);
export const awsWafv2ServiceIdentityRuleIds = manifest.service_identity_rules.map(
  (rule) => rule.source_policy_id,
);
export const awsWafv2RegionalAssociationId = regionalAssociation
  ? regionalAssociation.id
  : "not-configured";
export const awsWafv2WebAclArn = webAcl.arn;
export const awsWafv2WebAclName = webAcl.name;
