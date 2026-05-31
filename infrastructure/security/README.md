# Security Infrastructure

Generated traffic/auth edge artifacts consumed by the Pulumi infrastructure
stack live here.

- `traffic-auth-edge-policy.generated.json` is the provider-neutral edge ingress
  projection generated from `docs/architecture/traffic-auth-policy-registry.v1.json`.
- `aws-wafv2-edge-policy.generated.json` is the AWS WAFv2/Pulumi-facing rule
  manifest derived from that projection.

Do not edit generated files by hand. Change the registry, run
`scripts/ci/generate-traffic-auth-policy.ps1`, then run the registry checker.

Current status: WAFv2 rule intent is generated, drift-checked, and consumed by
`../index.ts`. The local preview guardrail runs through
`../../scripts/ci/check-pulumi-local-preview.ps1` and fails on Pulumi preview
warnings. Regional production stacks can attach the WebACL by setting
`wafRegionalResourceArn` to the target ALB/API Gateway ARN. CloudFront
attachment still belongs in the CloudFront distribution module because global
WebACLs are associated through the distribution configuration. Production deploy
admission also runs `../../scripts/ci/check-production-edge-admission.ps1` and
requires `GONGZZANG_WAF_REGIONAL_RESOURCE_ARN` plus the
`regional_association=planned` Pulumi preview evidence for regional ingress.
