# Traffic/Auth Policy SSOT Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a policy-as-code registry that is the single source for public route limits, service identity, cache, single-flight, and response budgets, then make proxy/API/CDN/mesh enforcement drift-checkable.

**Architecture:** Define traffic/auth policy once in `docs/architecture/traffic-auth-policy-registry.v1.json`. Enforcement remains layered at edge, Next proxy, Rust API, service-to-service auth, and data/cache layers, but every layer must either consume the registry or pass a CI drift check against it. This keeps rate limit and mTLS from becoming scattered constants while preserving defense in depth.

**Tech Stack:** JSON registry, PowerShell CI checks, Next.js proxy, TypeScript, Rust/Axum, Redis/Valkey-compatible cache, platform-core published HTTP/event contracts.

---

## Current State

The registry is now consumed by runtime code and checked by CI/pre-push:

- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `scripts/ci/check-traffic-auth-policy-registry`

Fresh local evidence:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-traffic-auth-policy-registry -Root .
# traffic-auth-policy-registry-ok routes=6 service_policies=2
```

This proves current Gongzzang public map route rate values, BFF route exposure,
auth route budgets, page gates, Rust API direct ingress rate policies, backend
role policies, provider-neutral edge/ingress projection, and listing marker
serving cache/budget constants match the registry. Runtime code consumes
generated TypeScript/Rust policy artifacts; the registry checker fails when
those generated artifacts, the provider-neutral edge projection, or middleware
mounts drift. AWS WAFv2/Pulumi manifests, Pulumi WebACL consumers, and
production deploy admission are deferred production-promotion work. They are not
part of the current Gongzzang Platform Core consumer integration gate and must
not be expanded while only validating the consumer boundary.

## Generated API Control Plane

Phase 1 is now active for Gongzzang browser-visible API proxy calls. The
registry is no longer only a guardrail that checks manually written frontend API
paths; it also generates `apps/web/lib/api/api-proxy-client.generated.ts` from
`api_proxy_route_policies`. Product code must call generated operations such as
`apiProxyClient.listingsCollectionRead.getJson(...)` or
`apiProxyClient.notificationMarkRead.patch(...)` instead of writing raw
`api.get("...")` or `api.post("...")` route strings directly.

The CI checker rejects direct `api.get/post/put/patch/delete(...)` usage outside
the generated client and transport boundary. This means newly added
browser-visible BFF calls must first be registered in
`traffic-auth-policy-registry.v1.json`, then generated, then consumed through the
approved client.

Current phase-1 shape:

- Registry remains the single source for exposure class, method, path, rate
  profile, auth requirement, and operational budget.
- Generator emits a typed API proxy client for all
  `api_proxy_route_policies`.
- Application code imports generated operations instead of hardcoding API path
  strings.
- CI rejects direct raw `api.get/post/put/patch/delete("...")` calls outside the
  generated client/transport layer.
- Edge/proxy/backend policy projection continues to be generated from the same
  registry, so rate limit, service identity, and exposure policy do not split
  across files.

Still deferred: production-promotion infrastructure such as AWS WAF/Pulumi
admission, mTLS certificate rollout, and GitHub environment secret wiring. Those
are operational deployment tasks, not blockers for the current API client
control-plane phase.

In plain terms: engineers no longer hand-write browser API roads and wait for an
inspector to catch missing registration. For supported API proxy calls, they now
build from the approved city map.

## File Structure

- `docs/architecture/traffic-auth-policy-registry.v1.json`
  - Source registry for public map route policies and service-call policies.
- `scripts/ci/check-traffic-auth-policy-registry`
  - Drift check proving current proxy/API constants match the registry.
- `apps/web/proxy.ts`
  - Current Next proxy enforcement for anonymous public map route rate limits.
- `services/api/src/listing_marker_serving.rs`
  - Current Rust enforcement for Redis cache, single-flight, and marker response budgets.
- `apps/web/lib/policies/traffic-auth-policy.generated.ts`
  - Generated TypeScript policy module consumed by `apps/web/proxy.ts`.
- `apps/web/lib/api/api-proxy-client.generated.ts`
  - Generated browser API proxy client consumed by frontend API modules.
- `services/api/src/listing_marker_policy.rs`
  - Generated Rust constants consumed by `listing_marker_serving.rs`.
- `services/api/src/traffic_auth_policy.rs`
  - Generated Rust backend rate and role policies consumed by API middleware.
- `services/api/src/backend_authorization.rs`
  - Rust API direct-ingress role guard for registry-declared privileged routes.
- `infrastructure/security/traffic-auth-edge-policy.generated.json`
  - Generated provider-neutral edge/ingress projection for CloudFront, AWS
    WAFv2, ALB, or service mesh IaC consumers.
- `scripts/ci/generate-traffic-auth-policy`
  - Generator for TypeScript, Rust, and provider-neutral edge policy artifacts.
- `.github/workflows/ci.yml`
  - CI hook for registry drift checks.
- `lefthook.yml`
  - Pre-push hook for registry drift checks.

## Plan Parts

Detailed task bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Registry And Runtime Generated Policies](./2026-05-28-traffic-auth-policy-ssot.part-01-registry-runtime-policies.md)
- [Part 02 - CI And Edge Projection](./2026-05-28-traffic-auth-policy-ssot.part-02-ci-edge-projection.md)
- [Part 03 - Platform Core Companion And Completion Gate](./2026-05-28-traffic-auth-policy-ssot.part-03-platform-core-completion-gate.md)
