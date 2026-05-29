# Platform Core Parcel Lookup Cutover Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove Gongzzang's direct runtime dependency on Platform Core-owned parcel domain and V-World catalog clients by consuming a Platform Core parcel-by-PNU HTTP contract.

**Architecture:** Platform Core owns canonical Catalog parcel reads and exposes a PNU lookup endpoint. Gongzzang keeps only the B2C `ParcelInfoLookup` port and maps Platform Core parcel classification into listing denormalization inputs, with unavailable official price/zoning fields explicitly absent until Platform Core publishes those fields.

**Tech Stack:** Rust, axum, sqlx, reqwest, PowerShell CI boundary checks.

**Execution note, 2026-05-29:** Gongzzang-side cutover is implemented with the
runtime HTTP adapter in `services/api/src/platform_core_parcel_lookup.rs` and
the `parcel-lookup` crate now keeps only the Gongzzang-facing port. Local
verification passed for `cargo test -p parcel-lookup`,
`cargo test -p api platform_core_parcel_lookup`,
`SQLX_OFFLINE=true cargo check -p api`,
`scripts/ci/check-platform-core-catalog-api-contract.ps1`,
and `scripts/ci/check-platform-core-dependency-boundary.ps1`. The catalog
contract checker reported `source_checked=True` with the sibling
`platform-core` repo present.

**Cross-repo verification note, 2026-05-29:** The sibling `platform-core`
route-label test for Task 1 was rerun with
`cargo test -p platform-core-api canonical_route_label_bounds_dynamic_metric_cardinality`
and passed. Platform Core webhook receiver, webhook contract, and traffic/auth
policy registry guardrails were also rerun and passed from the sibling repo.

---

## Task 1: Platform Core Parcel-By-PNU API

**Files:**
- Modify: `../platform-core/services/api/src/routes/mod.rs`
- Modify: `../platform-core/services/api/src/routes/catalog.rs`
- Modify: `../platform-core/docs/openapi/catalog.v1.yaml`

- [ ] **Step 1: Write the failing route-label test**

Add an assertion for `/catalog/v1/parcels/by-pnu/1111010100100090000` expecting `/catalog/v1/parcels/by-pnu/:pnu`.

- [ ] **Step 2: Run the focused Platform Core test**

Run: `cargo test -p platform-core-api canonical_route_label_bounds_dynamic_metric_cardinality`
Expected: FAIL before the route label exists.

- [ ] **Step 3: Implement the minimal route**

Add `GET /catalog/v1/parcels/by-pnu/:pnu`, parse `Pnu`, call `CatalogRepository::find_parcel_by_pnu`, and return existing `ParcelResponse`.

- [ ] **Step 4: Update OpenAPI**

Document `/catalog/v1/parcels/by-pnu/{pnu}` returning `ParcelResponse`.

- [ ] **Step 5: Re-run focused Platform Core checks**

Run: `cargo test -p platform-core-api canonical_route_label_bounds_dynamic_metric_cardinality`

## Task 2: Gongzzang Parcel Lookup HTTP Consumer

**Files:**
- Modify: `crates/parcel-lookup/Cargo.toml`
- Modify: `crates/parcel-lookup/src/lib.rs`
- Modify: `crates/parcel-lookup/src/info.rs`
- Modify: `crates/parcel-lookup/src/lookup.rs`
- Create: `crates/parcel-lookup/src/platform_core_lookup.rs`
- Delete or stop compiling: `crates/parcel-lookup/src/vworld_lookup.rs`
- Modify: `services/api/src/startup.rs`
- Modify: `services/api/src/main.rs`

- [ ] **Step 1: Write failing parcel lookup tests**

Cover success mapping, `404 -> None`, mismatched response PNU as parse error, and missing `PLATFORM_CORE_API_BASE_URL` config.

- [ ] **Step 2: Run `cargo test -p parcel-lookup`**

Expected: FAIL before `platform_core_lookup` exists.

- [ ] **Step 3: Implement Platform Core lookup**

Use `reqwest` to call `/catalog/v1/parcels/by-pnu/{pnu}` and map `factory/support/public/river/other` parcel kinds into Gongzzang `LandUseType` without importing `parcel-domain` or `vworld-client`.

- [ ] **Step 4: Wire API startup**

`build_parcel_lookup` should require `PLATFORM_CORE_API_BASE_URL` in production and use `NoOpParcelInfoLookup` only as a development fallback.

- [ ] **Step 5: Re-run parcel lookup and API checks**

Run: `cargo test -p parcel-lookup` and `SQLX_OFFLINE=true cargo check -p api`.

## Task 3: Boundary Enforcement

**Files:**
- Modify: `docs/architecture/platform-core-boundary.v1.json`
- Modify: `scripts/ci/check-platform-core-dependency-boundary.ps1`
- Modify: `scripts/ci/check-platform-core-dependency-boundary.tests.ps1`

- [ ] **Step 1: Write failing boundary tests**

Add cases where `crates/parcel-lookup/Cargo.toml` depends on `parcel-domain` or `vworld-client` and expect checker failure.

- [ ] **Step 2: Run boundary tests**

Run: `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-dependency-boundary.tests.ps1`
Expected: FAIL until the checker forbids those dependencies.

- [ ] **Step 3: Remove transitional allowances**

Delete `parcel-lookup -> parcel-domain` and `parcel-lookup -> vworld-client` from the boundary SSOT and add checker rules forbidding them.

- [ ] **Step 4: Verify boundary**

Run the dependency boundary test script and then the real checker against the repository root.
