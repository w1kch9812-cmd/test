# Gongzzang Marker Data Plane SSS Readiness

Date: 2026-05-31

## Scope

This handoff covers the Gongzzang-owned listing marker data plane hardening:

- listing marker tombstone overlay;
- listing marker delta overlay;
- dirty tile queue and marker-serving metrics;
- truthful low-zoom aggregation;
- frontend base plus delta minus tombstone composition;
- route exposure and traffic-auth SSOT updates;
- Playwright runtime isolation so E2E cannot attach to another local app;
- load-test asset guardrails wired into CI.

The platform-core boundary remains unchanged: platform-core owns parcel geometry, PNU anchors,
reference spatial layers, manifests, and service identity. Gongzzang owns listing semantics,
listing visibility, listing filters, and listing marker serving.

## Change Groups

| Group | Main files |
|---|---|
| Schema | `migrations/30017_listing_marker_overlay_and_dirty_queue.sql` |
| Domain ports | `crates/domain/core/listing/src/repository.rs` |
| DB implementation | `crates/db/src/listing/marker_projection.rs`, `marker_tile.rs`, `marker_delta.rs`, `marker_tombstone.rs` |
| API routes | `services/api/src/routes/listing_marker_deltas.rs`, `listing_marker_tombstones.rs`, `metrics.rs` |
| Frontend map runtime | `apps/web/lib/map/*`, `apps/web/components/listings/listing-map.tsx` |
| Policy SSOT | `docs/architecture/traffic-auth-policy-registry.v1.json`, generated policy outputs |
| Guardrails | `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`, `check-traffic-auth-policy-registry.ps1`, load-test asset checks |
| Playwright isolation | `apps/web/playwright-runtime.ts`, Playwright configs, frontend workflow |
| Documentation | `docs/frontend/listings-search.md`, `docs/testing/load.md`, this handoff |

## Verified Evidence

Fresh local verification in the current workspace:

| Command | Result |
|---|---|
| `cargo fmt -- --check` | Passed using `%USERPROFILE%\.cargo\bin\cargo.exe` |
| `cargo test -p listing-domain` | 56 passed |
| `sqlx migrate run --source migrations` | Passed after loading `DATABASE_URL` from `.env` |
| `cargo test -p db --features integration --test listing_marker_tile_integration` | 18 passed |
| `cargo test -p api listing_marker` | 16 passed |
| `cargo test -p api` | 92 unit + 16 bin + 3 integration passed |
| `pnpm --filter @gongzzang/web test` | 45 files passed, 194 passed, 1 skipped |
| `pnpm --filter @gongzzang/web typecheck` | Passed |
| `pnpm lint` | Passed |
| `pnpm markdownlint-cli2 docs/superpowers/plans/2026-05-31-gongzzang-marker-data-plane-sss.md docs/superpowers/plans/2026-05-31-gongzzang-marker-data-plane-sss.part-*.md docs/testing/playwright-runtime.md` | 0 errors |
| `CI=1 pnpm --filter @gongzzang/web exec playwright test` | 15 passed, 4 skipped |
| `check-pnu-anchor-pbf-marker-contract.ps1` and `.tests.ps1` | Passed |
| `check-traffic-auth-policy-registry.ps1`, `.tests.ps1`, and `-IncludeProductionEdge` | Passed |
| `check-platform-core-boundary.ps1` | Passed |
| `check-platform-core-dependency-boundary.ps1` | Passed |
| `check-platform-integration-policy.ps1` and `.tests.ps1` | Passed |
| `check-load-test-assets.ps1` and `.tests.ps1` | Passed |
| `git diff --check` | Passed |

## Remaining Before Launch Claim

This work is merge-ready from code and guardrail evidence, but it is not launch capacity evidence.

Before a production launch-capacity claim:

- run the approved perf/staging k6 workflow for `api-read-mix`, `map-marker-mix`,
  `capacity-stress`, and `platform-core-events`;
- verify the downloaded artifact with `scripts/ci/verify-load-test-capacity-evidence.ps1`;
- confirm no stale private/deleted marker exposure, no silent tile drop, and no DB pool saturation
  under the accepted launch RPS.

## Review Notes

- The expected visible marker formula is:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

- The Playwright E2E default endpoint is now `http://127.0.0.1:3100`; probe runs default to
  `http://127.0.0.1:3101`.
- CI must not set a hardcoded local `ZITADEL_REDIRECT_URI` for Playwright. The runtime derives it
  from the managed base URL.
- `apps/web/next-env.d.ts` may be rewritten by local Next dev runs. It was restored after E2E.
