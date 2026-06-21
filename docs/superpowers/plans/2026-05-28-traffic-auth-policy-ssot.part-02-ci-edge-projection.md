# Traffic/Auth Policy SSOT Plan - Part 02: CI And Edge Projection

Parent index: [Traffic/Auth Policy SSOT Implementation Plan](./2026-05-28-traffic-auth-policy-ssot.md).


## Task 4: CI And Pre-Push Enforcement

**Files:**

- Modify: `.github/workflows/ci.yml`
- Modify: `lefthook.yml`
- Test: `scripts/ci/check-traffic-auth-policy-registry`

- [x] **Step 1: Add CI workflow command**

Add this command to the CI guardrail section (registry parity is now verified by
regenerating with the Rust generator and confirming no diff, per ADR-0044):

```bash
cargo run -p api --bin generate-traffic-auth-policy
git diff --exit-code apps/web/lib/policies services/api/src/traffic_auth_policy.rs services/api/src/listing_marker_policy.rs infrastructure/security/traffic-auth-edge-policy.generated.json
```

- [x] **Step 2: Add pre-push command**

Add the same command to `lefthook.yml` pre-push checks.

- [x] **Step 3: Verify check command**

Run:

```bash
cargo run -p api --bin generate-traffic-auth-policy
git diff --exit-code apps/web/lib/policies services/api/src/traffic_auth_policy.rs services/api/src/listing_marker_policy.rs
```

Expected: no diff (generated policy artifacts already match the registry).

## Task 4.5: Generate Edge/Ingress Projection

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy`
- Modify: `scripts/ci/check-traffic-auth-policy-registry`
- Create: `infrastructure/security/traffic-auth-edge-policy.generated.json`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests`

- [x] **Step 1: Emit provider-neutral edge policy**

The generator writes `traffic-auth-edge-policy.generated.json` from the
registry. The projection includes public route rules, auth route rules, BFF API
proxy route rules, and service-to-service rules for future CloudFront, AWS
WAFv2, ALB, or service mesh IaC consumers.

- [x] **Step 2: Fail CI on edge projection drift**

The checker validates the generated edge projection against registry route IDs,
paths, methods, exposure classes, rate projections, required roles, forbidden
public request shapes, and service-auth environment names. The checker test
suite includes a negative case for a missing generated edge policy file.

- [x] **Step 3: Verify edge projection checks**

Run:

```bash
cargo run -p api --bin generate-traffic-auth-policy
git diff --exit-code apps/web/lib/policies services/api/src/traffic_auth_policy.rs services/api/src/listing_marker_policy.rs infrastructure/security/traffic-auth-edge-policy.generated.json
```

Expected:

```text
traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts rust=services/api/src/listing_marker_policy.rs,services/api/src/traffic_auth_policy.rs edge=infrastructure/security/traffic-auth-edge-policy.generated.json
```

(`git diff --exit-code` reports no diff, proving the generated artifacts match the registry.)

## Task 4.6: Generate AWS WAFv2/Pulumi Manifest

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy`
- Modify: `scripts/ci/check-traffic-auth-policy-registry`
- Create: `infrastructure/security/aws-wafv2-edge-policy.generated.json`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests`

> Deferred until production promotion. Do not execute as part of the current
> Platform Core consumer integration gate.

- [ ] **Step 1: Fail when the WAFv2 manifest is missing**

The checker test suite should include a negative case proving that a repo with the
provider-neutral edge projection but without the AWS WAFv2/Pulumi-facing
manifest fails.

- [ ] **Step 2: Generate the WAFv2 manifest**

The manifest should contain WAF-representable IP rate rules for public/auth routes,
blocked query-shape rules for public marker tile requests, service identity
trace entries, and explicit `identity_aware_application_rules` for controls
that must stay in the application or service boundary because WAFv2 cannot key
them by session or service identity.

- [ ] **Step 3: Verify WAFv2 manifest drift**

The checker should validate source policy IDs, priorities, methods, path match mode,
five-minute WAF rate-limit conversion, forbidden query-shape blocks, and
identity-aware fallback markers against the registry and edge projection.

## Task 4.7: Add Pulumi WebACL Consumer

**Files:**

- Create: `infrastructure/Pulumi.yaml`
- Create: `infrastructure/index.ts`
- Modify: `scripts/ci/check-traffic-auth-policy-registry`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests`

> Deferred until production promotion. Do not execute as part of the current
> Platform Core consumer integration gate.

- [ ] **Step 1: Fail when Pulumi does not consume the manifest**

The checker test suite should include a negative case proving that the WAFv2 manifest
alone is not enough; the repo must also contain a Pulumi project and consumer
that reads `security/aws-wafv2-edge-policy.generated.json`.

- [ ] **Step 2: Add the WebACL resource consumer**

`infrastructure/index.ts` should read the generated WAFv2 manifest, create
rate-based rules, create blocked query-shape rules, support optional regional
association through `wafRegionalResourceArn`, and export identity-aware and
service-identity rule IDs as evidence that those controls remain outside WAF
when WAF cannot represent the key strategy.

- [ ] **Step 3: Verify consumer drift checks**

The registry checker should validate the Pulumi project runtime, AWS provider import,
WebACL resource, optional WebACL association support, generated manifest path,
and required manifest members.

- [ ] **Step 4: Run Pulumi preview**

When this deferred workstream opens, `scripts/ci/check-pulumi-local-preview`
should log into a local file backend under `target/`, initialize a local preview
stack, and run `pulumi preview` without real AWS credentials. The guardrail
should fail on preview warnings as well as non-zero exits. Production admission
must pass `GONGZZANG_WAF_REGIONAL_RESOURCE_ARN` only for the preview process, so
`Pulumi.local-preview.yaml` remains unmodified.

## Task 4.8: Add Pulumi Dependencies And CI Preview

**Files:**

- Modify: `pnpm-workspace.yaml`
- Create: `infrastructure/package.json`
- Create: `infrastructure/tsconfig.json`
- Create: `infrastructure/Pulumi.local-preview.yaml`
- Create: `scripts/ci/check-pulumi-local-preview`
- Modify: `.github/workflows/ci.yml`

> Deferred until production promotion. Do not execute as part of the current
> Platform Core consumer integration gate.

- [ ] **Step 1: Add infrastructure workspace package**

The `infrastructure` directory should become a pnpm workspace package named
`@gongzzang/infrastructure`, keeping Pulumi dependencies out of web/API runtime
packages.

- [ ] **Step 2: Add Pulumi SDK and CLI dependencies**

The infrastructure package should declare `@pulumi/pulumi`, `@pulumi/aws`, and
the `pulumi` CLI package. The traffic/auth checker should verify those package
entries.

- [ ] **Step 3: Typecheck infrastructure**

`pnpm typecheck` should include `@gongzzang/infrastructure` through the
workspace coverage guard.

- [ ] **Step 4: Add CI local preview**

Future CI should run `scripts/ci/check-pulumi-local-preview`, which performs
a warning-free local file-backend preview for the generated WAFv2 WebACL.
