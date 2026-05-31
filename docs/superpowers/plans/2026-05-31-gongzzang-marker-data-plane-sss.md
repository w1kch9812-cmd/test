# Gongzzang Marker Data Plane SSS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Gongzzang listing marker serving structurally SSS-grade by adding tombstone, delta, aggregation, dirty-tile, and platform-core-aligned registry controls.

**Architecture:** platform-core remains the map control-plane for PNU anchors, vector manifests, reference layers, service identity, and events. Gongzzang remains the listing marker data-plane for listing semantics, public/private visibility, filter hashes, marker projection, overlays, and dirty-tile rebuild decisions. The runtime composition becomes `base tile + delta overlay - tombstone overlay - unauthorized records`.

**Tech Stack:** Rust, Axum, SQLx, Postgres/PostGIS, Redis, Next.js route proxy, existing Gongzzang policy registries.

---

## Implementation Status - 2026-05-31

Current status: implemented and locally verified for code, contracts, CI guardrails, and Playwright
runtime isolation. This plan remains the original execution recipe; the checklist below is not the
authoritative completion log.

Authoritative evidence gathered in this workspace:

| Area | Evidence |
|---|---|
| Schema and DB behavior | `cargo test -p db --features integration --test listing_marker_tile_integration` passed after loading `DATABASE_URL` from `.env`. |
| Domain contracts | `cargo test -p listing-domain` passed. |
| API routes and metrics | `cargo test -p api listing_marker` and `cargo test -p api` passed. |
| Frontend composition | `pnpm --filter @gongzzang/web test`, `pnpm --filter @gongzzang/web typecheck`, and Playwright E2E passed. |
| SSOT route policy | `check-traffic-auth-policy-registry.ps1`, `check-traffic-auth-policy-registry.tests.ps1`, and `check-traffic-auth-policy-registry.ps1 -IncludeProductionEdge` passed. |
| PNU marker guardrail | `check-pnu-anchor-pbf-marker-contract.ps1` and `.tests.ps1` passed. |
| Platform boundary | `check-platform-core-boundary.ps1`, `check-platform-core-dependency-boundary.ps1`, `check-platform-integration-policy.ps1`, and `.tests.ps1` passed. |
| Load-test asset gate | `check-load-test-assets.ps1` and `.tests.ps1` passed, and CI now runs both checks. |
| Repository hygiene | `cargo fmt -- --check`, `pnpm lint`, and `git diff --check` passed. |

Important remaining distinction:

- A real perf/staging k6 run is still required before claiming launch capacity. The current load-test
  evidence is a harness and guardrail proof, not a production capacity proof.

---

## File Structure

Create:

- `migrations/30017_listing_marker_overlay_and_dirty_queue.sql` - tombstone, delta, and dirty-tile tables.
- `crates/db/src/listing/marker_delta.rs` - DB query for recent listing marker delta overlays.
- `crates/db/src/listing/marker_tombstone.rs` - DB query for listing marker tombstones.
- `services/api/src/routes/listing_marker_deltas.rs` - HTTP endpoint for delta overlay.
- `services/api/src/routes/listing_marker_tombstones.rs` - HTTP endpoint for tombstone overlay.

Modify:

- `crates/domain/core/listing/src/repository.rs` - add overlay query/response value objects and repository ports.
- `crates/db/src/listing.rs` - expose new DB modules.
- `crates/db/src/listing/repository.rs` - implement new repository methods.
- `crates/db/src/listing/marker_projection.rs` - write tombstone/delta/dirty records when projection changes.
- `crates/db/src/listing/marker_tile.rs` - support truthful low-zoom aggregation.
- `crates/db/src/listing/marker_mask.rs` - ensure mask responses exclude active tombstoned marker ids.
- `services/api/src/listing_marker_serving.rs` - cache and validate delta/tombstone responses.
- `services/api/src/routes/mod.rs` - register new routes.
- `services/api/src/routes/listing_marker_common.rs` - reuse filter resolution for new overlay routes.
- `docs/architecture/traffic-auth-policy-registry.v1.json` - add route policies for delta/tombstone/dirty operations.
- `docs/architecture/platform-integration/route-exposure-policy.v1.json` - add public exposure entries.
- `apps/web/lib/map/marker-tile-style.ts` - register stable style ids for base and delta layers.
- `apps/web/lib/map/vector-tile-manifest.ts` - resolve allowed marker overlay origins and URLs.
- `apps/web/lib/map/listing-map-runtime.ts` - apply base + delta - tombstone composition.
- `apps/web/components/listings/listing-map.tsx` - pass overlay state into the map runtime.
- `crates/db/tests/listing_marker_tile_integration.rs` and `crates/db/tests/listing_marker_tile_integration/filter_index.rs` - add integration coverage.
- `services/api/src/routes/listing_marker_tiles.rs` - mirror route parsing tests for overlay routes.
- `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1` - block regressions.

Do not modify:

- platform-core Catalog tables from Gongzzang.
- listing canonical coordinates. Listing rows still must not own lat/lng/geom_point.
- platform-core DB directly from Gongzzang runtime.

---

## Plan Parts

Detailed task bodies are split by responsibility so this plan remains a navigable SSOT instead of a single oversized file.

- [Part 01 - Overlay Schema And Domain Contracts](./2026-05-31-gongzzang-marker-data-plane-sss.part-01-overlay-schema-domain.md)
- [Part 02 - Projection, Tombstone, And Delta](./2026-05-31-gongzzang-marker-data-plane-sss.part-02-projection-tombstone-delta.md)
- [Part 03 - Aggregation And Dirty Queue](./2026-05-31-gongzzang-marker-data-plane-sss.part-03-aggregation-dirty-queue.md)
- [Part 04 - Guardrails, Frontend, And Release Gate](./2026-05-31-gongzzang-marker-data-plane-sss.part-04-guardrails-frontend-release.md)
