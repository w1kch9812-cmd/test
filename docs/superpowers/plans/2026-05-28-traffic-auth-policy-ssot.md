# Traffic/Auth Policy SSOT Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a policy-as-code registry that is the single source for public route limits, service identity, cache, single-flight, and response budgets, then make proxy/API/CDN/mesh enforcement drift-checkable.

**Architecture:** Define traffic/auth policy once in `docs/architecture/traffic-auth-policy-registry.v1.json`. Enforcement remains layered at edge, Next proxy, Rust API, service-to-service auth, and data/cache layers, but every layer must either consume the registry or pass a CI drift check against it. This keeps rate limit and mTLS from becoming scattered constants while preserving defense in depth.

**Tech Stack:** JSON registry, PowerShell CI checks, Next.js proxy, TypeScript, Rust/Axum, Redis/Valkey-compatible cache, platform-core published HTTP/event contracts.

---

## Current State

The registry is now consumed by runtime code and checked by CI/pre-push:

- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `scripts/ci/check-traffic-auth-policy-registry.ps1`

Fresh local evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
# traffic-auth-policy-registry-ok routes=4 service_policies=2
```

This proves current Gongzzang public map route rate values, BFF route exposure,
auth route budgets, page gates, Rust API direct ingress rate policies, backend
role policies, provider-neutral edge/ingress projection, and listing marker
serving cache/budget constants match the registry. Runtime code consumes
generated TypeScript/Rust policy artifacts; the registry checker fails when
those generated artifacts, the edge projection, the AWS WAFv2/Pulumi manifest,
Pulumi consumer, association support, or middleware mounts drift.
`infrastructure/index.ts` now reads the generated WAFv2 manifest, declares the
Pulumi WebACL resource, and can attach the WebACL to a regional target when an
environment stack sets `wafRegionalResourceArn`. `@gongzzang/infrastructure`
owns the Pulumi npm dependencies, and CI runs the local Pulumi preview
guardrail against `infrastructure/security/aws-wafv2-edge-policy.generated.json`.
The preview guardrail fails on Pulumi warnings as well as non-zero exits.
The remaining production deployment gap is creating environment stacks with
real AWS credentials and setting the target ALB/API Gateway ARN, or wiring the
generated WebACL ARN into the CloudFront distribution module for global edge
scope. Production deploy admission now calls
`scripts/ci/check-production-edge-admission.ps1`, which blocks regional
production deploy admission unless `GONGZZANG_WAF_REGIONAL_RESOURCE_ARN`
contains a valid WAFv2 regional association target and the Pulumi preview emits
`regional_association=planned`.

## File Structure

- `docs/architecture/traffic-auth-policy-registry.v1.json`
  - Source registry for public map route policies and service-call policies.
- `scripts/ci/check-traffic-auth-policy-registry.ps1`
  - Drift check proving current proxy/API constants match the registry.
- `apps/web/proxy.ts`
  - Current Next proxy enforcement for anonymous public map route rate limits.
- `services/api/src/listing_marker_serving.rs`
  - Current Rust enforcement for Redis cache, single-flight, and marker response budgets.
- `apps/web/lib/policies/traffic-auth-policy.generated.ts`
  - Generated TypeScript policy module consumed by `apps/web/proxy.ts`.
- `services/api/src/listing_marker_policy.rs`
  - Generated Rust constants consumed by `listing_marker_serving.rs`.
- `services/api/src/traffic_auth_policy.rs`
  - Generated Rust backend rate and role policies consumed by API middleware.
- `services/api/src/backend_authorization.rs`
  - Rust API direct-ingress role guard for registry-declared privileged routes.
- `infrastructure/security/traffic-auth-edge-policy.generated.json`
  - Generated provider-neutral edge/ingress projection for CloudFront, AWS
    WAFv2, ALB, or service mesh IaC consumers.
- `infrastructure/security/aws-wafv2-edge-policy.generated.json`
  - Generated AWS WAFv2/Pulumi-facing rule manifest derived from the edge
    projection.
- `infrastructure/Pulumi.yaml`
  - Pulumi project descriptor for Gongzzang infrastructure.
- `infrastructure/Pulumi.local-preview.yaml`
  - Local preview stack config for CI-safe WebACL preview without real AWS
    credentials.
- `infrastructure/index.ts`
  - Pulumi WebACL consumer and optional regional WebACL association for the
    generated AWS WAFv2 manifest.
- `infrastructure/package.json`
  - Pulumi SDK and CLI dependencies for infrastructure-only previews.
- `scripts/ci/check-pulumi-local-preview.ps1`
  - CI guardrail that runs local Pulumi preview for the generated WebACL.
- `scripts/ci/generate-traffic-auth-policy.ps1`
  - Generator for TypeScript, Rust, edge, and AWS WAFv2 policy artifacts.
- `.github/workflows/ci.yml`
  - CI hook for registry drift checks.
- `lefthook.yml`
  - Pre-push hook for registry drift checks.

## Task 1: Registry And Drift Check

**Files:**

- Create: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Create: `scripts/ci/check-traffic-auth-policy-registry.ps1`
- Verify: `apps/web/proxy.ts`
- Verify: `services/api/src/listing_marker_serving.rs`

- [x] **Step 1: Add the policy registry**

The registry must include:

```json
{
  "schema_version": "gongzzang.traffic_auth_policy_registry.v1",
  "public_route_policies": [
    {
      "id": "gongzzang.public_map.listing_marker_tile",
      "rate_policy": {
        "key_prefix": "public-map:listing-marker-tile",
        "limit": 600,
        "window_seconds": 60,
        "rejection_status": 429
      },
      "cache_policy": {
        "server_cache": "redis",
        "ttl_seconds": 30
      },
      "single_flight_policy": {
        "required": true,
        "lock_seconds": 5,
        "wait_attempts": 10,
        "wait_milliseconds": 50
      },
      "response_budget": {
        "max_tile_bytes": 262144,
        "max_features": 10000
      }
    }
  ]
}
```

- [x] **Step 2: Add the drift check**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
```

Expected:

```text
traffic-auth-policy-registry-ok routes=4 service_policies=2
```

- [x] **Step 3: Confirm current registry matches code**

The script must compare:

- `apps/web/proxy.ts` public map route key prefixes, limits, and windows.
- `services/api/src/listing_marker_serving.rs` cache TTL, single-flight lock/wait values, tile byte budget, feature budget, and mask id budget.
- `docs/architecture/platform-core-boundary.v1.json` service auth env contract.

## Task 2: Generate TypeScript Proxy Policy

**Files:**

- Create: `scripts/ci/generate-traffic-auth-policy.ps1`
- Create: `apps/web/lib/policies/traffic-auth-policy.generated.ts`
- Modify: `apps/web/proxy.ts`
- Test: `apps/web/tests/unit/platform-core-proxy.test.ts`

- [x] **Step 1: Add a generator that emits TypeScript policy**

Create `scripts/ci/generate-traffic-auth-policy.ps1` with a generator that reads `docs/architecture/traffic-auth-policy-registry.v1.json` and writes `apps/web/lib/policies/traffic-auth-policy.generated.ts`.

The generated TypeScript shape must be:

```ts
export type GeneratedPublicMapRoutePolicy = {
  readonly kind: "exact" | "prefix";
  readonly pathSource: string;
  readonly rate: {
    readonly keyPrefix: string;
    readonly limit: number;
    readonly windowSec: number;
  };
};

export const GENERATED_PUBLIC_MAP_ROUTE_POLICIES: readonly GeneratedPublicMapRoutePolicy[] = [
  {
    kind: "prefix",
    pathSource: "API.proxy.listingMarkerTilesPrefix",
    rate: { keyPrefix: "public-map:listing-marker-tile", limit: 600, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerCounts",
    rate: { keyPrefix: "public-map:listing-marker-count", limit: 120, windowSec: 60 },
  },
  {
    kind: "exact",
    pathSource: "API.proxy.listingMarkerFilters",
    rate: { keyPrefix: "public-map:listing-marker-filter", limit: 60, windowSec: 60 },
  },
  {
    kind: "prefix",
    pathSource: "LISTING_MARKER_MASK_PREFIX",
    rate: { keyPrefix: "public-map:listing-marker-mask", limit: 120, windowSec: 60 },
  },
];
```

- [x] **Step 2: Run the generator**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\generate-traffic-auth-policy.ps1 -Root .
```

Expected:

```text
traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts
```

- [x] **Step 3: Replace hardcoded proxy rate constants with generated values**

Modify `apps/web/proxy.ts` so `PUBLIC_MAP_ROUTE_POLICIES` maps generated path sources to concrete route paths:

```ts
import { GENERATED_PUBLIC_MAP_ROUTE_POLICIES } from "@/lib/policies/traffic-auth-policy.generated";

function resolvePublicMapPath(pathSource: string): string {
  switch (pathSource) {
    case "API.proxy.listingMarkerTilesPrefix":
      return API.proxy.listingMarkerTilesPrefix;
    case "API.proxy.listingMarkerCounts":
      return API.proxy.listingMarkerCounts;
    case "API.proxy.listingMarkerFilters":
      return API.proxy.listingMarkerFilters;
    case "LISTING_MARKER_MASK_PREFIX":
      return LISTING_MARKER_MASK_PREFIX;
    default:
      throw new Error(`Unknown public map route policy path source: ${pathSource}`);
  }
}

const PUBLIC_MAP_ROUTE_POLICIES: readonly PublicMapRoutePolicy[] =
  GENERATED_PUBLIC_MAP_ROUTE_POLICIES.map((policy) => ({
    kind: policy.kind,
    path: resolvePublicMapPath(policy.pathSource),
    rate: policy.rate,
  }));
```

- [x] **Step 4: Verify proxy tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-proxy.test.ts
```

Expected:

```text
Test Files  1 passed
Tests  7 passed
```

## Task 3: Generate Rust Listing Marker Serving Policy

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy.ps1`
- Create: `services/api/src/listing_marker_policy.rs`
- Modify: `services/api/src/main.rs`
- Modify: `services/api/src/listing_marker_serving.rs`
- Test: `services/api/src/listing_marker_serving.rs`

- [x] **Step 1: Extend generator for Rust constants**

The generator must emit `services/api/src/listing_marker_policy.rs`:

```rust
//! Generated listing marker serving policy from docs/architecture/traffic-auth-policy-registry.v1.json.

pub const MAX_LISTING_MARKER_TILE_BYTES: usize = 262_144;
pub const MAX_LISTING_MARKER_TILE_FEATURES: i64 = 10_000;
pub const MAX_LISTING_MARKER_MASK_IDS: usize = 20_000;
pub const LISTING_MARKER_CACHE_TTL_SECONDS: u64 = 30;
pub const LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS: u64 = 5;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS: usize = 10;
pub const LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS: u64 = 50;
```

- [x] **Step 2: Register generated module**

Modify `services/api/src/main.rs`:

```rust
mod listing_marker_policy;
mod listing_marker_serving;
```

- [x] **Step 3: Consume generated constants**

Modify `services/api/src/listing_marker_serving.rs`:

```rust
use crate::listing_marker_policy::{
    LISTING_MARKER_CACHE_TTL_SECONDS, LISTING_MARKER_SINGLE_FLIGHT_LOCK_SECONDS,
    LISTING_MARKER_SINGLE_FLIGHT_WAIT_ATTEMPTS, LISTING_MARKER_SINGLE_FLIGHT_WAIT_MS,
    MAX_LISTING_MARKER_MASK_IDS, MAX_LISTING_MARKER_TILE_BYTES,
    MAX_LISTING_MARKER_TILE_FEATURES,
};
```

Then replace local constants with imported generated constants.

- [x] **Step 4: Verify Rust API binary compile**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --bins --all-features
```

Expected:

```text
Finished `dev` profile
```

## Task 4: CI And Pre-Push Enforcement

**Files:**

- Modify: `.github/workflows/ci.yml`
- Modify: `lefthook.yml`
- Test: `scripts/ci/check-traffic-auth-policy-registry.ps1`

- [x] **Step 1: Add CI workflow command**

Add this command to the CI guardrail section:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
```

- [x] **Step 2: Add pre-push command**

Add the same command to `lefthook.yml` pre-push checks.

- [x] **Step 3: Verify check command**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
```

Expected:

```text
traffic-auth-policy-registry-ok routes=4 service_policies=2
```

## Task 4.5: Generate Edge/Ingress Projection

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy.ps1`
- Modify: `scripts/ci/check-traffic-auth-policy-registry.ps1`
- Create: `infrastructure/security/traffic-auth-edge-policy.generated.json`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests.ps1`

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

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\generate-traffic-auth-policy.ps1 -Root .
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
```

Expected:

```text
traffic-auth-policy-registry-tests-ok
traffic-auth-policy-generated ts=apps/web/lib/policies/traffic-auth-policy.generated.ts rust=services/api/src/listing_marker_policy.rs,services/api/src/traffic_auth_policy.rs edge=infrastructure/security/traffic-auth-edge-policy.generated.json aws_wafv2=infrastructure/security/aws-wafv2-edge-policy.generated.json
traffic-auth-policy-registry-ok routes=4 service_policies=2
```

## Task 4.6: Generate AWS WAFv2/Pulumi Manifest

**Files:**

- Modify: `scripts/ci/generate-traffic-auth-policy.ps1`
- Modify: `scripts/ci/check-traffic-auth-policy-registry.ps1`
- Create: `infrastructure/security/aws-wafv2-edge-policy.generated.json`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests.ps1`

- [x] **Step 1: Fail when the WAFv2 manifest is missing**

The checker test suite includes a negative case proving that a repo with the
provider-neutral edge projection but without the AWS WAFv2/Pulumi-facing
manifest fails.

- [x] **Step 2: Generate the WAFv2 manifest**

The manifest contains WAF-representable IP rate rules for public/auth routes,
blocked query-shape rules for public marker tile requests, service identity
trace entries, and explicit `identity_aware_application_rules` for controls
that must stay in the application or service boundary because WAFv2 cannot key
them by session or service identity.

- [x] **Step 3: Verify WAFv2 manifest drift**

The checker validates source policy IDs, priorities, methods, path match mode,
five-minute WAF rate-limit conversion, forbidden query-shape blocks, and
identity-aware fallback markers against the registry and edge projection.

## Task 4.7: Add Pulumi WebACL Consumer

**Files:**

- Create: `infrastructure/Pulumi.yaml`
- Create: `infrastructure/index.ts`
- Modify: `scripts/ci/check-traffic-auth-policy-registry.ps1`
- Test: `scripts/ci/check-traffic-auth-policy-registry.tests.ps1`

- [x] **Step 1: Fail when Pulumi does not consume the manifest**

The checker test suite includes a negative case proving that the WAFv2 manifest
alone is not enough; the repo must also contain a Pulumi project and consumer
that reads `security/aws-wafv2-edge-policy.generated.json`.

- [x] **Step 2: Add the WebACL resource consumer**

`infrastructure/index.ts` reads the generated WAFv2 manifest, creates rate-based
rules, creates blocked query-shape rules, supports optional regional association
through `wafRegionalResourceArn`, and exports identity-aware and service-identity
rule IDs as evidence that those controls remain outside WAF when WAF cannot
represent the key strategy.

- [x] **Step 3: Verify consumer drift checks**

The registry checker validates the Pulumi project runtime, AWS provider import,
WebACL resource, optional WebACL association support, generated manifest path,
and required manifest members.

- [x] **Step 4: Run Pulumi preview**

`scripts/ci/check-pulumi-local-preview.ps1` logs into a local file backend under
`target/`, initializes the `local-preview` stack, and runs `pulumi preview`
without real AWS credentials. The guardrail fails on preview warnings as well as
non-zero exits. With no regional target ARN configured, the local preview plans
one Pulumi stack and one `aws:wafv2:WebAcl` resource, and exports
`awsWafv2RegionalAssociationId` as `not-configured`. Environment stacks that set
`wafRegionalResourceArn` will also plan an
`aws:wafv2:WebAclAssociation`. The production admission path passes
`GONGZZANG_WAF_REGIONAL_RESOURCE_ARN` only for the preview process, so
`Pulumi.local-preview.yaml` stays unmodified.

## Task 4.8: Add Pulumi Dependencies And CI Preview

**Files:**

- Modify: `pnpm-workspace.yaml`
- Create: `infrastructure/package.json`
- Create: `infrastructure/tsconfig.json`
- Create: `infrastructure/Pulumi.local-preview.yaml`
- Create: `scripts/ci/check-pulumi-local-preview.ps1`
- Modify: `.github/workflows/ci.yml`

- [x] **Step 1: Add infrastructure workspace package**

The `infrastructure` directory is now a pnpm workspace package named
`@gongzzang/infrastructure`, keeping Pulumi dependencies out of web/API runtime
packages.

- [x] **Step 2: Add Pulumi SDK and CLI dependencies**

The infrastructure package declares `@pulumi/pulumi`, `@pulumi/aws`, and the
`pulumi` CLI package. The traffic/auth checker verifies those package entries.

- [x] **Step 3: Typecheck infrastructure**

`pnpm typecheck` now includes `@gongzzang/infrastructure` through the workspace
coverage guard.

- [x] **Step 4: Add CI local preview**

`.github/workflows/ci.yml` runs `scripts/ci/check-pulumi-local-preview.ps1`,
which performs a warning-free local file-backend preview for the generated WAFv2
WebACL.

## Task 5: Platform Core Companion Registry

**Files:**

- Create in sibling repo: `../platform-core/docs/architecture/traffic-auth-policy-registry.v1.json`
- Create in sibling repo: `../platform-core/scripts/ci/check-traffic-auth-policy-registry.ps1`
- Modify in sibling repo: `../platform-core/services/api/src/traffic.rs`
- Modify in sibling repo: `../platform-core/services/api/src/routes/mod.rs`

- [x] **Step 1: Add platform-core registry**

The platform-core registry must declare:

- global HTTP timeout, body limit, and concurrency.
- public marker contract endpoint exposure.
- DB-backed marker tile route as `diagnostic_reference`, not launch runtime.
- required production edge/app route policy for public routes.
- service identity policy for Gongzzang callers.

- [x] **Step 2: Add drift check**

The drift check must compare registry values to:

- `services/api/src/traffic.rs`
- `services/api/src/routes/mod.rs`
- `docs/adr/0008-pnu-anchor-pbf-marker-tile-contract.md`

- [x] **Step 3: Verify platform-core check**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root C:\Users\admin\Desktop\platform-core
```

Expected:

```text
traffic-auth-policy-registry-ok
```

## Task 6: Completion Gate

**Files:**

- Verify: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Verify: `apps/web/proxy.ts`
- Verify: `services/api/src/listing_marker_serving.rs`
- Verify: sibling `platform-core` registry and checks

- [x] **Step 1: Verify Gongzzang registry drift**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry.ps1 -Root .
```

Expected:

```text
traffic-auth-policy-registry-ok routes=4 service_policies=2
```

- [x] **Step 2: Verify focused web policy tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-proxy.test.ts
```

Expected:

```text
Test Files  1 passed
Tests  7 passed
```

- [x] **Step 3: Verify Rust executable compile**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --bins --all-features
```

Expected:

```text
Finished `dev` profile
```

- [x] **Step 4: Verify all-targets once existing unrelated test drift is resolved**

Run:

```powershell
C:\Users\admin\.cargo\bin\cargo.exe check --workspace --all-targets --all-features
```

Expected:

```text
Finished `dev` profile
```

Fresh local evidence on 2026-05-29: the broad all-targets workspace check
completed successfully.
