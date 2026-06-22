# Next Actions

Last updated: 2026-06-19

This file is the active short-term queue. Historical SP9, Bronze, and public
data collection notes were removed from this active queue because Platform Core
now owns Catalog ingestion, raw lineage, and public/reference spatial data
lifecycle.

Current marker gate: Gongzzang-owned listing PBF marker tiles are
local-verification-backed. This is not a whole-product launch completion claim;
rerun the handoff/audit verification before changing or claiming this slice.
platform-core owns PNU anchors; Gongzzang owns listing semantics and
Gongzzang-owned listing PBF tiles.

## Current SSOT

- ADR 0034: [Catalog ownership handover](../adr/0034-catalog-ownership-handover-to-platform-core.md)
- Boundary manifest: [platform-core-boundary.v1.json](../architecture/platform-core-boundary.v1.json)
- Catalog API pin: [platform-core-catalog-api-contract.v1.pin.json](../architecture/platform-core-catalog-api-contract.v1.pin.json)
- Webhook receiver pin: [platform-core-webhook-receiver-contract.v1.pin.json](../architecture/platform-core-webhook-receiver-contract.v1.pin.json)
- DB approval handoff: [2026-05-28 platform-core physical extraction DB schema approval](./handoff/2026-05-28-platform-core-physical-extraction-db-schema-approval.md)
- Anchor inbox DB approval request:
  [2026-05-29 platform-core anchor inbox DB schema approval request](./handoff/2026-05-29-platform-core-anchor-inbox-db-schema-approval-request.md)
- Listing marker gate: [2026-05-22 listing PBF review gate](./handoff/2026-05-22-listing-pbf-review-gate.md)
- Platform Integration Policy:
  [platform-integration index](../architecture/platform-integration/index.v1.json)
- Traffic/auth and public API exposure registry:
  [traffic-auth-policy-registry.v1.json](../architecture/traffic-auth-policy-registry.v1.json)
- Platform Core integration operations:
  [operations runbook](../runbooks/platform-core-integration-operations.md)
- Concurrent session role split:
  [2026-06-19 concurrent session role split](./handoff/2026-06-19-concurrent-session-role-split.md)

## Platform Core UI Decision

Platform Core UI is deferred. Gongzzang must not create customer-facing Platform
Core UI or move product listing workflows into Platform Core. If an internal
Platform Core console is introduced later, it should be a read-only-first
control-plane console for Platform Core operational evidence: Catalog snapshots,
tile manifests, anchor rebuilds, outbox/webhook delivery, DLQ, SLO, provenance,
and deploy candidate verification.

The console must use Platform Core public/admin APIs only. It must not bypass
the API boundary with direct database access, and any future write action must
require RBAC, audit log, approval trail, and runbook coverage.

## Concurrent Session Rule

When more than one agent session is active, split work by ownership before editing files.
The current split is recorded in
[2026-06-19 concurrent session role split](./handoff/2026-06-19-concurrent-session-role-split.md).

The short version:

- Build/verification stays on the native toolchains: `cargo` for Rust, `pnpm` + `Turborepo`
  for the frontend (ADR-0002; ADR-0044 reversed the abandoned Bazel transition, so there is no
  Bazel worker / transition ratchet role).
- Product architecture, ownership-boundary audit, and next-action clarity may proceed in a
  separate session.
- Public-data collection, DB migrations, R2 deletion, production infrastructure, Kafka, and
  Kubernetes remain approval-gated.

## Do Next

This queue is for the current Platform Core consumer integration scope. Do not
start production edge attachment, Pulumi environment stacks, production deploy
admission, or perf/staging launch-capacity evidence from this section. Those
belong to the deferred production-promotion queue below.

1. Keep Platform Core boundary gates green after every Platform Core integration
   change. The boundary, dependency boundary, catalog API consumer contract, and
   event-receiver contract are enforced by `scripts/lefthook/catalog-m1-boundary.sh`
   together with the contracts in `docs/architecture/platform-core-boundary.v1.json`
   and `docs/architecture/platform-core-catalog-api-contract.v1.pin.json`.

2. Keep browser-visible API work classified in the traffic/auth registry before
   implementation. Anonymous public routes may expose only minimized derived
   data; raw listing details, private listings, business-verified listing
   details, contact data, raw Platform Core Catalog data, and bulk exports must
   stay behind authenticated/privileged or service-to-service surfaces. The
   `/api/proxy/[...path]` BFF target allowlist is generated from the same
   registry, so new frontend backend calls must add an explicit
   `api_proxy_route_policies` entry before code uses the path. Privileged BFF
   routes must also declare `required_roles` there; route handlers consume the
   generated policy instead of hardcoding role lists. Auth endpoint rate limits
   must be declared under `auth_route_policies`. Authenticated and privileged
   BFF route-handler rate budgets must be declared once under
   `route_rate_profiles` and referenced by `api_proxy_route_policies` through
   `rate_profile`; the generated route handler policy enforces those budgets
   before upstream calls. Public marker route budgets live on
   `public_route_policies.rate_policy`. The Rust API direct ingress limiter is
   generated from the same registry into
   `services/api/src/traffic_auth_policy.rs` and enforced by
   `services/api/src/backend_rate_limit.rs` with Redis-backed counters. Backend
   privileged route role gates are generated into the same Rust policy module
   as `BACKEND_ROLE_POLICIES` and enforced by
   `services/api/src/backend_authorization.rs`, so direct Rust API calls cannot
   rely only on BFF-side role checks. Page-level role gates must be declared
   under `page_route_policies`;
   `apps/web/proxy.ts` consumes only the generated policy outputs for those
   controls. Listing mutation page roles must match the corresponding
   privileged BFF/API route roles, so the UI cannot admit roles that the backend
   mutation route will reject. Rust API routes must have a matching
   `backend_route_policies` entry so the checker can verify public, protected,
   and internal route exposure against `services/api/src/main.rs` and route
   modules. Health/readiness routes remain unmetered; internal service routes
   must stay non-browser surfaces or receive an explicit policy before exposure.

   Platform Core service-to-service auth now prefers
   `PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE` for short-lived credentials.
   The static `PLATFORM_CORE_SERVICE_TOKEN` path remains a temporary fallback
   only with scope, issue time, expiry time, rotation owner, default-deny
   allowed-call headers, and rotation runbook coverage. Platform Core's
   matching inbound policy lives in
   `../platform-core/docs/architecture/traffic-auth-policy-registry.v1.json`;
   its API now protects Gongzzang's parcel-by-PNU service reads with
   `PLATFORM_CORE_GONGZZANG_WORKLOAD_IDENTITY_TOKEN_FILE` preferred and
   `PLATFORM_CORE_GONGZZANG_SERVICE_TOKEN` as fallback, plus the same
   policy/source/target and allowed-call headers. Public manifest/contract
   routes remain separate public surfaces and are not treated as private
   service reads.

   Anchor snapshot import now has a durable retry surface: the Rust importer can
   be run with `PLATFORM_CORE_EVENT_ID`, reclaims `processing` inbox rows after
   a worker exit, loads the event payload from `platform_core_event_inbox` when
   local artifact paths are absent, fetches the Platform Core manifest URL,
   verifies the manifest checksum, resolves object keys relative to that
   manifest URL, and holds a PostgreSQL advisory lock derived from the event id
   so two importers cannot process the same event concurrently. When no single
   event id or local artifact path is set, the importer now runs pending inbox
   batch mode with `PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT` defaulting to 10
   and capped at 100. The operational recovery steps live in
   `docs/runbooks/platform-core-integration-operations.md`, and the required
   fault evidence is tracked in
   `docs/architecture/platform-integration/operations-policy.v1.json` as
   `anchor_import_processing_reclaim`.

3. Legacy Platform Core schema cleanup has its own approval handoff and uses
   `migrations/30015_drop_platform_core_legacy_schema.sql`. The durable anchor
   event inbox/import migration was separately approved on 2026-05-29 and is now
   implemented as `migrations/30016_platform_core_event_inbox_anchor_import.sql`
   for `platform_core_event_inbox` and
   `parcel_marker_anchor.algorithm_version` widening. Any follow-up schema
   change needs a new DB approval record before creating another migration.

4. Keep Gongzzang-owned listing marker work inside the listing boundary:
   listing semantics, listing marker serving projections, filters, masks, and
   PBF tile responses stay here. PNU anchors, parcel geometry, public/reference
   spatial layers, and Catalog source ingestion stay in Platform Core.

5. When Platform Core changes a published Catalog or webhook contract, update
   only the pin files and local consumer adapters, then rerun the contract
   checkers. Gongzzang must not consume Platform Core databases or private
   internal modules.

6. For any future Gongzzang-owned external API adapter, write an ADR first and
   update the boundary manifest. Catalog source, raw lineage, and public spatial
   readers are not valid Gongzzang adapter candidates.

7. Keep Platform Core UI out of Gongzzang scope. Treat future console work as a
   Platform Core or Dawneer workbench concern, not as a Gongzzang product
   surface.

## Deferred Until Production Promotion

These items are important, but they are not part of the current Platform Core
consumer integration work. Do not run or expand them while only validating the
Gongzzang consumer boundary.

1. Production edge attachment remains a deployment-stage task. The
   traffic/auth registry can generate provider-neutral edge policy and
   AWS WAFv2/Pulumi-facing manifests, but real production work starts only when
   the release owner is ready to attach CloudFront, AWS WAFv2, ALB, or service
   mesh policy. That stage requires `GONGZZANG_WAF_REGIONAL_RESOURCE_ARN` or an
   explicit CloudFront attachment path.

2. Any future production deploy workflow must call
   `.github/workflows/production-deploy-admission.yml` before promotion. The
   admission workflow verifies the attested CI run artifact, provenance, SBOM,
   approved workflow, approved ref, subject digest, healthy perf/staging
   load-test capacity evidence, and production edge admission.
   The deploy-candidate verifier and supply-chain promotion runbook are
   production-promotion artifacts; they are not required for the current
   consumer integration gate.

3. Load-test harness work is evidence-pipeline ready, not launch-capacity
   complete. Do not claim production launch sizing from local/ci smoke or
   host-process sizing results. A real perf/staging operator run against an
   approved non-production target remains required before promotion.

## Do Not Recreate

- Local Catalog source clients for parcel, building, industrial complex, or
  manufacturer master data.
- Local Catalog raw capture or API drift observability flows.
- Local public/reference vector tile ETL, scraper, or R2 reader implementations.
- Listing-owned canonical marker coordinates such as latitude, longitude, or
  geometry point columns.

Git history preserves the old queue. Do not copy those historical steps back
into this file unless a new ADR supersedes ADR 0034.
