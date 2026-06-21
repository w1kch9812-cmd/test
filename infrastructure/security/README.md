# Security Infrastructure

Generated traffic/auth edge artifacts consumed by the Pulumi infrastructure
stack live here.

- `traffic-auth-edge-policy.generated.json` is the provider-neutral edge ingress
  projection generated from `docs/architecture/traffic-auth-policy-registry.v1.json`.
- `aws-wafv2-edge-policy.generated.json` is the AWS WAFv2/Pulumi-facing rule
  manifest derived from that projection.

Do not edit generated files by hand. Change the registry, then regenerate with
`cargo run -p api --bin generate-traffic-auth-policy`.

Current status: WAFv2 rule intent is generated, drift-checked, and consumed by
`../index.ts`. A local Pulumi preview must pass without warnings before
promotion. Regional production stacks can attach the WebACL by setting
`wafRegionalResourceArn` to the target ALB/API Gateway ARN. CloudFront
attachment still belongs in the CloudFront distribution module because global
WebACLs are associated through the distribution configuration. Production deploy
admission requires `GONGZZANG_WAF_REGIONAL_RESOURCE_ARN` plus the
`regional_association=planned` Pulumi preview evidence for regional ingress.
