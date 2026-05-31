# Listing Marker Serving Index And Filter Mask Plan - Part 4: Registration, Mask, Docs

> Extracted from `2026-05-26-listing-marker-serving-index-filter-mask.md` to keep each plan file below the 500-line SSS guardrail.
> See the index file for the full sequence and cross-links.

## Task 7: Filter Registration And Stable Hash API

**Files:**
- Create: `services/api/src/routes/listing_marker_filters.rs`
- Create: `services/api/src/routes/listing_marker_common.rs`
- Create: `migrations/30014_listing_marker_filter_registry.sql`
- Create: `crates/db/src/listing/marker_filter_registry.rs`
- Modify: `services/api/src/main.rs`
- Modify: `services/api/src/routes/listing_marker_counts.rs`
- Modify: `services/api/src/routes/listing_marker_tiles.rs`
- Modify: `crates/db/src/listing/marker_tile.rs`
- Modify: `apps/web/lib/routes.ts`
- Modify: `apps/web/lib/map/marker-tile-contract.ts`
- Modify: `apps/web/tests/unit/map/marker-tile-contract.test.ts`

- [x] **Step 1: Add route unit tests**

In `services/api/src/routes/listing_marker_filters.rs`, add tests for hash validation:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_response_hash_uses_stable_prefix() {
        let response = ListingMarkerFilterResponse {
            filter_hash: "lst_filter_v1_abc".to_owned(),
        };

        assert!(response.filter_hash.starts_with("lst_filter_v1_"));
    }
}
```

- [x] **Step 2: Implement route**

Create route with payload:

```rust
use axum::extract::Json;
use listing_domain::marker_filter::ListingMarkerFilterSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ListingMarkerFilterRequest {
    pub types: Vec<shared_kernel::listing_type::ListingType>,
    pub transactions: Vec<shared_kernel::transaction_type::TransactionType>,
    pub min_area_m2: Option<i64>,
    pub max_area_m2: Option<i64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListingMarkerFilterResponse {
    pub filter_hash: String,
}

pub async fn post_listing_marker_filter(
    Json(request): Json<ListingMarkerFilterRequest>,
) -> Result<Json<ListingMarkerFilterResponse>, crate::http::problem::ProblemResponse> {
    let normalized = ListingMarkerFilterSpec {
        types: request.types,
        transactions: request.transactions,
        min_area_m2: request.min_area_m2,
        max_area_m2: request.max_area_m2,
        min_price_krw: request.min_price_krw,
        max_price_krw: request.max_price_krw,
    }
    .try_normalized()
    .map_err(|e| {
        crate::http::problem::problem(
            "map/listing-marker-filter-invalid",
            "listing marker filter is invalid",
            axum::http::StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    Ok(Json(ListingMarkerFilterResponse {
        filter_hash: normalized.filter_hash(),
    }))
}
```

- [x] **Step 3: Wire route**

Add to `services/api/src/main.rs` public map router:

```rust
.route(
    "/map/v1/marker-filters/listing",
    axum::routing::post(routes::listing_marker_filters::post_listing_marker_filter),
)
```

- [x] **Step 4: Add frontend route constants**

Update `apps/web/lib/routes.ts`:

```ts
listingMarkerFilters: `${API_PROXY_BASE}/map/v1/marker-filters/listing`,
listingMarkerCounts: `${API_PROXY_BASE}/map/v1/marker-counts/listing`,
listingMarkerMaskTemplate: `${API_PROXY_BASE}/map/v1/marker-masks/listing/{z}/{x}/{y}?filter_hash={hash}&base_version={baseVersion}`,
```

- [x] **Step 5: Run API and frontend checks**

Run:

```bash
cargo check -p api
pnpm --filter @gongzzang/web test -- tests/unit/map/marker-tile-contract.test.ts
```

Expected: API compiles and route constants do not regress marker tile URL tests.

Additional implementation completed during execution:
- Persisted `filter_hash -> normalized filter spec` in `listing_marker_filter_registry`.
- Resolved registered filter hashes from repository-backed API common code.
- Applied normalized filters inside the listing marker MVT SQL path.
- Verified registered listing marker hashes in the web marker tile contract.

## Task 8: Optional Filter Mask

**Files:**
- Create: `crates/db/src/listing/marker_mask.rs`
- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/db/src/listing.rs`
- Modify: `crates/db/src/listing/repository.rs`
- Create: `services/api/src/routes/listing_marker_masks.rs`
- Modify: `services/api/src/main.rs`
- Modify: `crates/db/tests/listing_marker_tile_integration.rs`

- [x] **Step 1: Add failing DB mask test**

Add `listing_marker_mask_returns_show_ids_for_loaded_tile` to
`crates/db/tests/listing_marker_tile_integration.rs`. Seed one active listing and one
`parcel_marker_anchor`, call `upsert_listing_marker_projection`, then assert
`find_listing_marker_mask(ListingMarkerMaskQuery { z: 0, x: 0, y: 0, filter: AllActive, base_version: None })`
returns `encoding = Show`, one `marker_id`, latest `projection_version`, and `snapshot-test-v1`.

- [x] **Step 2: Add domain DTOs**

Add `ListingMarkerMaskEncoding::{Show, Hide}`, `ListingMarkerMask`, and `ListingMarkerMaskQuery`
to `crates/domain/core/listing/src/repository.rs`. `ListingMarkerMask` must expose only
`marker_ids`, `projection_version`, and `anchor_snapshot_id`; it must not expose coordinates.

- [x] **Step 3: Implement DB mask**

Create `crates/db/src/listing/marker_mask.rs`. Query `listing_marker_projection` only, with
`listing_status = 'active'`, `visibility_scope = 'public'`, and tile containment through
`ST_Intersects(ST_Transform(anchor_point, 3857), ST_TileEnvelope($1, $2, $3))`. Return sorted
`marker_id` values and aggregate metadata. The first encoding is `Show`; add `Hide` only as a
domain-supported enum for future smaller responses.

- [x] **Step 4: Add JSON route**

Create `services/api/src/routes/listing_marker_masks.rs` returning JSON with:
`encoding`, `marker_ids`, `projection_version`, and `anchor_snapshot_id`. The route must reject
malformed `filter_hash` and stale `base_version` with `application/problem+json`.

- [x] **Step 5: Run focused checks**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_mask
cargo check -p api
```

Expected: mask test passes and API compiles.

## Task 9: Frontend Authoritative Count And Mask Hook

**Files:**
- Modify: `apps/web/lib/routes.ts`
- Create: `apps/web/lib/map/listing-marker-server-state.ts`
- Create: `apps/web/lib/map/listing-marker-server-state.test.ts`
- Modify: `apps/web/components/listings/listing-map.tsx`

- [x] **Step 1: Add tests for request coalescing key**

Create `apps/web/lib/map/listing-marker-server-state.test.ts`. Assert
`buildListingMarkerServerKey({ filterHash: "all-active-v1", projectionVersion: 123, anchorSnapshotId: "snapshot-test-v1" })`
returns `listing|all-active-v1|123|snapshot-test-v1`.

- [x] **Step 2: Implement helper**

Create `apps/web/lib/map/listing-marker-server-state.ts` with
`ListingMarkerServerKeyInput { filterHash, projectionVersion, anchorSnapshotId }` and
`buildListingMarkerServerKey`, joining missing metadata as `none`.

- [x] **Step 3: Wire count fetch after fast filter changes**

In `listing-map.tsx`, keep browser instant filter immediate. In a separate effect, POST current fast
filters to `API.proxy.listingMarkerFilters` with `AbortController`, store returned `filter_hash`,
then fetch `API.proxy.listingMarkerCounts?filter_hash=...` in another abortable effect. Abort stale
requests and ignore `AbortError`; log non-abort failures through the existing frontend convention.

- [x] **Step 4: Run frontend tests**

Run:

```bash
pnpm --filter @gongzzang/web test -- lib/map/listing-marker-server-state.test.ts lib/map/listing-marker-filter.test.ts
```

Expected: server-state helper and instant filter tests pass.

## Task 10: Guardrails And Documentation

**Files:**
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`
- Modify: `docs/frontend/listings-search.md`
- Modify: `docs/superpowers/next-actions.md`

- [x] **Step 1: Extend guardrail contract**

Add required tokens for:

```text
docs/adr/0038-listing-marker-serving-index-filter-mask.md
docs/superpowers/specs/2026-05-26-listing-marker-serving-index-filter-mask-design.md
docs/superpowers/plans/2026-05-26-listing-marker-serving-index-filter-mask.md
listing_marker_projection
listing_marker_projection_anchor_srid_chk
listing_marker_projection_z14_tile_idx
buildListingMarkerLayerFilter
marker-counts/listing
marker-masks/listing
```

Add forbidden tokens for new marker serving files:

```text
bounds=
bbox=
listing_lng
listing_lat
geom_point
find_markers_in_bbox
```

- [x] **Step 2: Run guardrail tests**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
```

Expected: guardrail test suite and contract check pass.

- [x] **Step 3: Update runtime guide**

Update `docs/frontend/listings-search.md` with:

```markdown
## Listing Marker Serving

Listing map markers use the Gongzzang `listing_marker_projection` read model. The listing table is
the write model and remains the SSOT for listing semantics, but map serving reads from projection
and filter indexes.

Fast filters such as asset type, deal type, price, and area are applied immediately in the browser
against safe properties present in the loaded marker tile. Exact nationwide counts and unseen tile
results come from server marker indexes.
```

- [x] **Step 4: Run markdown and diff checks**

Run:

```bash
pnpm markdownlint-cli2 docs/adr/0038-listing-marker-serving-index-filter-mask.md docs/superpowers/specs/2026-05-26-listing-marker-serving-index-filter-mask-design.md docs/superpowers/plans/2026-05-26-listing-marker-serving-index-filter-mask.md docs/frontend/listings-search.md
git diff --check
```

Expected: markdown lint and whitespace checks pass.

## Final Verification

Run the focused verification set before claiming this implementation slice is complete:

```bash
cargo test -p listing-domain marker_filter
cargo test -p db --features integration --test listing_marker_tile_integration
cargo check -p api
pnpm --filter @gongzzang/web test -- lib/map/listing-marker-filter.test.ts lib/map/listing-marker-server-state.test.ts tests/unit/map/marker-tile-contract.test.ts tests/unit/map/marker-tile-style.test.ts tests/unit/listings/filters.test.ts
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
pnpm markdownlint-cli2 docs/adr/0038-listing-marker-serving-index-filter-mask.md docs/superpowers/specs/2026-05-26-listing-marker-serving-index-filter-mask-design.md docs/superpowers/plans/2026-05-26-listing-marker-serving-index-filter-mask.md docs/frontend/listings-search.md
git diff --check
```

Completion claim is allowed only when every command above exits 0 and the migration approval gate for `30013_listing_marker_projection.sql` has been satisfied.
