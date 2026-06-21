# Platform Integration Policy SSOT Design

## Purpose

Gongzzang already has separate guardrails for Platform Core boundary ownership,
traffic/rate policy, service-auth environment contracts, webhook signatures, and
supply-chain checks. The next enterprise-grade step is to make those controls
discoverable and drift-checkable as one integration control plane without
collapsing every policy into a large single file.

## Design

Create `docs/architecture/platform-integration/` as a folder-shaped SSOT. The
folder owns an index file and small policy files:

- `index.v1.json` lists the governed policy components and required guardrails.
- `route-exposure-policy.v1.json` declares which Platform Core-facing surfaces
  are public, service-only, webhook-only, or diagnostic.
- `service-auth-policy.v1.json` declares outbound service identity requirements.
- `webhook-policy.v1.json` declares inbound signed-event requirements.
- `supply-chain-policy.v1.json` declares npm/Rust/secret scanning enforcement.

Existing files remain authoritative for their narrow domains:

- `docs/architecture/traffic-auth-policy-registry.v1.json` remains the traffic,
  public-map rate, cache, single-flight, and response-budget SSOT.
- `docs/architecture/platform-core-boundary.v1.json` remains the Platform Core
  ownership and integration-boundary SSOT.

The new checker verifies the index and policy files exist, agree with the
existing registries, and are wired into CI/pre-push. This prevents the common
failure mode where rate limits, mTLS/service tokens, webhook signatures, and
supply-chain gates are implemented in different places with no single inventory.

## Enforcement

`scripts/ci/check-platform-integration-policy` validates:

- policy component presence and schema versions;
- traffic-auth registry and Platform Core boundary registry are referenced;
- outbound `PLATFORM_CORE_SERVICE_TOKEN` and inbound
  `PLATFORM_CORE_WEBHOOK_SECRET` are declared and enforced by code;
- webhook signature uses timestamped HMAC and required Platform Core headers;
- package overrides, `pnpm audit`, cargo-deny, and gitleaks are wired;
- the checker itself is present in CI and pre-push.

## Non-Goals

This design does not introduce service mesh, mTLS certificates, artifact
signing, or SBOM publication in this step. It creates the control-plane shape
and drift checks so those can be added as generated enforcement later.
