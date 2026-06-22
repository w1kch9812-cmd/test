# 2026-05-31 Change Bundle Split

> Historical record (2026-05-31). NOTE: the production deploy-admission /
> provenance machinery listed under "Bundle C" below was later removed
> pre-launch (ADR-0044) — `.github/workflows/production-deploy-admission.yml`,
> `scripts/ci/verify-production-deploy-candidate`, and
> `scripts/ci/check-production-edge-admission(.tests)` no longer exist. This file
> is kept as a timestamped record, not current guidance.

This workspace currently contains three logical change bundles. Keep them separate during review or
commit staging so marker-serving behavior, test-runtime hardening, and operations gates do not hide
each other.

## Bundle A - Listing Marker Data Plane

Purpose: implement the Gongzzang marker data-plane formula:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

Files:

- `migrations/30017_listing_marker_overlay_and_dirty_queue.sql`
- `crates/domain/core/listing/src/repository.rs`
- `crates/db/src/listing.rs`
- `crates/db/src/listing/repository.rs`
- `crates/db/src/listing/marker_projection.rs`
- `crates/db/src/listing/marker_tile.rs`
- `crates/db/src/listing/marker_delta.rs`
- `crates/db/src/listing/marker_tombstone.rs`
- `crates/db/tests/common.rs`
- `crates/db/tests/listing_marker_tile_integration.rs`
- `crates/db/tests/listing_marker_tile_integration/filter_index.rs`
- `services/api/src/listing_marker_serving.rs`
- `services/api/src/main.rs`
- `services/api/src/routes/listing_marker_common.rs`
- `services/api/src/routes/listing_marker_tiles.rs`
- `services/api/src/routes/listing_marker_deltas.rs`
- `services/api/src/routes/listing_marker_tombstones.rs`
- `services/api/src/routes/metrics.rs`
- `apps/web/components/listings/listing-map.tsx`
- `apps/web/lib/map/listing-map-runtime.ts`
- `apps/web/lib/map/listing-marker-filter.ts`
- `apps/web/lib/map/listing-marker-filter.test.ts`
- `apps/web/lib/map/marker-tile-contract.ts`
- `apps/web/lib/map/marker-tile-style.ts`
- `apps/web/tests/unit/map/marker-tile-contract.test.ts`
- `apps/web/tests/unit/map/marker-tile-style.test.ts`
- `apps/web/lib/routes.ts`
- `apps/web/proxy.ts`
- `docs/architecture/traffic-auth-policy-registry.v1.json`
- `docs/architecture/platform-integration/route-exposure-policy.v1.json`
- generated policy outputs that come from the registry.

Primary verification:

- `cargo test -p listing-domain`
- `cargo test -p db --features integration --test listing_marker_tile_integration`
- `cargo test -p api listing_marker`
- `pnpm --filter @gongzzang/web test`
- marker contract and traffic-auth registry guardrails.

## Bundle B - Playwright Runtime Isolation

Purpose: prevent E2E/probe runs from attaching to a different local app on `localhost:3000`.

Files:

- `apps/web/playwright-runtime.ts`
- `apps/web/playwright.config.ts`
- `apps/web/playwright.probes.config.ts`
- `apps/web/tests/unit/playwright-runtime.test.ts`
- `apps/web/tests/unit/playwright-config.test.ts`
- `apps/web/tests/unit/playwright-workflow.test.ts`
- `apps/web/tests/e2e/auth.ts`
- `apps/web/tests/e2e/panel-system.spec.ts`
- `apps/web/tests/probes/naver-sdk.probe.ts`
- `.github/workflows/frontend.yml`
- `docs/testing/playwright-runtime.md`
- `.gitignore` entry for `.wrangler/`.

Primary verification:

- `pnpm --filter @gongzzang/web test`
- `pnpm --filter @gongzzang/web typecheck`
- `CI=1 pnpm --filter @gongzzang/web exec playwright test`
- `pnpm lint`.

## Bundle C - Load-Test And Production Admission Gates

Purpose: keep load-test capacity evidence and production promotion checks explicit, artifact-backed,
and CI-guarded.

Files:

- `.github/workflows/ci.yml`
- `.github/workflows/load-test-capacity.yml`
- `.github/workflows/production-deploy-admission.yml`
- `docs/testing/load.md`
- `docs/research/2026-05-29-load-test-result.md`
- `docs/research/2026-05-29-local-sizing-test-results.md`
- `docs/runbooks/supply-chain-provenance-and-deploy-gate.md`
- `docs/superpowers/plans/2026-05-29-load-test-capacity-sizing.md`
- `scripts/load/run-k6`
- `scripts/load/normalize-k6-summary`
- `scripts/ci/check-load-test-assets`
- `scripts/ci/check-load-test-assets.tests`
- `scripts/ci/verify-load-test-capacity-evidence`
- `scripts/ci/verify-load-test-capacity-evidence.tests`
- `scripts/ci/check-production-edge-admission`
- `scripts/ci/check-production-edge-admission.tests`
- `scripts/ci/verify-production-deploy-candidate`
- `scripts/ci/check-pulumi-local-preview`
- `infrastructure/**`

Primary verification:

- `check-load-test-assets` and `.tests`
- `check-platform-integration-policy` and `.tests`
- production admission verification scripts.

Important: Bundle C does not prove launch capacity by itself. It proves the evidence pipeline and
the promotion gate. Launch capacity still requires accepted perf/staging evidence.

## Recommended Order

1. Merge Bundle B first if the team wants stable local/CI E2E behavior before reviewing marker code.
2. Merge Bundle A second. This is the product/runtime behavior change.
3. Merge Bundle C third, or keep it in the operations hardening track if product reviewers want a
   smaller marker-only PR.

If only one PR is allowed, use this split as review sections in the PR description.
