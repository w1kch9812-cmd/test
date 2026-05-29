# Gongzzang-Owned Listing PBF Marker Tiles Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Serve Gongzzang-owned active listing marker tiles as MVT/PBF while using platform-core PNU anchors as the only marker position source.

**Architecture:** Gongzzang keeps listing semantics and exposes `/map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf`. Marker coordinates come from a local `platform-core` anchor projection keyed by PNU. Successful tiles represent every eligible listing; no bbox launch API and no listing-owned coordinates are introduced.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL/PostGIS `ST_AsMVT`, Next.js mapbox-gl vector source.

---

## Task 1: Repository Tile Contract

**Files:**
- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/db/src/listing.rs`
- Test: `crates/db/tests/listing_marker_tile_integration.rs`

- [x] **Step 1: Write failing integration tests**

Add tests proving two active listings on the same PNU are both represented in the PBF tile and draft listings are excluded.

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p db --features integration --test listing_marker_tile_integration`

- [x] **Step 3: Implement minimal repository API**

Add typed filter and tile query structs, validate `all-active-v1`, and generate MVT bytes with PostGIS.

- [x] **Step 4: Run test to verify it passes**

Run: `cargo test -p db --features integration --test listing_marker_tile_integration`

## Task 2: Anchor Projection Migration

**Files:**
- Create: `migrations/30012_parcel_marker_anchor_projection.sql`
- Modify: `crates/db/tests/common.rs`
- Modify: `tests/migrations/test_v001_full.sh`

- [x] **Step 1: Add migration smoke assertions**

Assert the anchor projection table, SRID constraint, and index exist.

- [x] **Step 2: Run migration smoke and observe failure before migration**

Run: `bash tests/migrations/test_v001_full.sh`

- [x] **Step 3: Add the projection table**

Create `parcel_marker_anchor` as a platform-core projection with `anchor_point geometry(Point, 4326)` and lineage columns.

- [x] **Step 4: Re-run migration smoke**

Run: `bash tests/migrations/test_v001_full.sh`

## Task 3: API Route

**Files:**
- Create: `services/api/src/routes/listing_marker_tiles.rs`
- Modify: `services/api/src/main.rs`

- [x] **Step 1: Add route validation tests or focused compile checks**

Validate tile coordinate/hash failures map to problem+json and success returns PBF content type.

- [x] **Step 2: Wire public listing tile route**

Expose `GET /map/v1/marker-tiles/listing/{z}/{x}/{y}.pbf?filter_hash=all-active-v1` outside authenticated listing card APIs.

- [x] **Step 3: Run API compile check**

Run: `cargo check -p api`

## Task 4: Frontend Listing Source

**Files:**
- Modify: `apps/web/lib/map/marker-tile-contract.ts`
- Modify: `apps/web/lib/map/marker-tile-style.ts`
- Modify: `apps/web/components/listings/listing-map.tsx`
- Modify: `apps/web/tests/unit/map/marker-tile-contract.test.ts`
- Modify: `apps/web/tests/unit/map/marker-tile-style.test.ts`

- [x] **Step 1: Add failing unit tests**

Prove the frontend builds a Gongzzang listing vector source without `bbox`, `bounds`, `lat`, or `lng`.

- [x] **Step 2: Implement listing source/layer registration**

Use the Gongzzang API base/proxy path for `listing` PBF tiles and keep platform-core parcel polygons separate.

- [x] **Step 3: Run frontend unit tests**

Run: `pnpm --filter web test:unit -- map/marker-tile-contract.test.ts map/marker-tile-style.test.ts`

## Task 5: Guardrails And Verification

**Files:**
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`
- Modify: `docs/superpowers/handoff/2026-05-22-active-goal-completion-audit.md`

- [x] **Step 1: Extend guardrails for the new route/table/source**

Require listing PBF route, anchor projection wording, and no bbox/listing-coordinate regression.

- [x] **Step 2: Run focused verification**

Run Rust tests/checks, frontend unit tests, markdown lint, guardrails, and diff checks before any completion claim.
