# Listing Marker Serving Index And Filter Mask Plan - Part 3: Counts And Instant Filters

> Extracted from `2026-05-26-listing-marker-serving-index-filter-mask.md` to keep each plan file below the 500-line SSS guardrail.
> See the index file for the full sequence and cross-links.

## Task 5: Count API Backed By Server Index

**Files:**
- Create: `crates/db/src/listing/marker_count.rs`
- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/db/src/listing.rs`
- Modify: `crates/db/src/listing/repository.rs`
- Create: `services/api/src/routes/listing_marker_counts.rs`
- Modify: `services/api/src/main.rs`
- Modify: `crates/db/tests/listing_marker_tile_integration.rs`

- [x] **Step 1: Add failing DB test for exact count**

Add:

```rust
#[tokio::test]
async fn listing_marker_count_applies_price_area_and_type_filters() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-count", "marker-count@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100110000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Count listing");
    activate_listing(&repo, &mut listing, &owner).await;
    repo.upsert_listing_marker_projection(&listing.id).await.unwrap();

    let filter_spec = ListingMarkerFilterSpec {
        types: vec![ListingType::Factory],
        transactions: vec![TransactionType::Sale],
        min_area_m2: Some(300),
        max_area_m2: Some(400),
        min_price_krw: Some(100_000_000),
        max_price_krw: Some(900_000_000),
    };
    let filter = match filter_spec.try_normalized() {
        Ok(value) => value,
        Err(err) => panic!("valid count filter rejected: {err}"),
    };

    let count = repo.count_listing_markers(filter).await.unwrap();
    assert_eq!(count.total_count, 1);
}
```

- [x] **Step 2: Run test and confirm failure**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_count
```

Expected: failure because `count_listing_markers` is not implemented.

- [x] **Step 3: Add repository DTO and trait method**

In `repository.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerCount {
    pub total_count: i64,
    pub projection_version: Option<i64>,
    pub anchor_snapshot_id: Option<String>,
}

async fn count_listing_markers(
    &self,
    filter: NormalizedListingMarkerFilterSpec,
) -> Result<ListingMarkerCount, RepoError>;
```

- [x] **Step 4: Implement DB count query**

Create `crates/db/src/listing/marker_count.rs`:

```rust
use listing_domain::marker_filter::NormalizedListingMarkerFilterSpec;
use listing_domain::repository::{ListingMarkerCount, RepoError};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn count_listing_markers(
    pool: &PgPool,
    filter: NormalizedListingMarkerFilterSpec,
) -> Result<ListingMarkerCount, RepoError> {
    let row = sqlx::query(
        r"
        select
            count(*)::int8 as total_count,
            max(projection_version)::int8 as projection_version,
            max(anchor_snapshot_id) as anchor_snapshot_id
        from listing_marker_projection
        where listing_status = 'active'
          and visibility_scope = 'public'
          and (cardinality($1::text[]) = 0 or listing_type = any($1::text[]))
          and (cardinality($2::text[]) = 0 or transaction_type = any($2::text[]))
          and ($3::int8 is null or area_m2 >= $3)
          and ($4::int8 is null or area_m2 <= $4)
          and ($5::int8 is null or price_krw >= $5)
          and ($6::int8 is null or price_krw <= $6)
        ",
    )
    .bind(filter.types.iter().map(|v| v.as_str()).collect::<Vec<_>>())
    .bind(filter.transactions.iter().map(|v| v.as_str()).collect::<Vec<_>>())
    .bind(filter.min_area_m2)
    .bind(filter.max_area_m2)
    .bind(filter.min_price_krw)
    .bind(filter.max_price_krw)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerCount {
        total_count: row.try_get("total_count").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
```

- [x] **Step 5: Add API route**

Create `services/api/src/routes/listing_marker_counts.rs`:

```rust
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use listing_domain::marker_filter::ListingMarkerFilter;
use listing_domain::repository::ListingRepository;
use serde::{Deserialize, Serialize};

use crate::http::problem::{problem, ProblemResponse};

#[derive(Clone)]
pub struct ListingMarkerCountsState {
    pub listing_repo: Arc<dyn ListingRepository>,
}

#[derive(Debug, Deserialize)]
pub struct ListingMarkerCountHttpQuery {
    pub filter_hash: String,
}

#[derive(Debug, Serialize)]
pub struct ListingMarkerCountResponse {
    pub total_count: i64,
    pub projection_version: Option<i64>,
    pub anchor_snapshot_id: Option<String>,
}

pub async fn get_listing_marker_count(
    State(state): State<ListingMarkerCountsState>,
    Query(query): Query<ListingMarkerCountHttpQuery>,
) -> Result<axum::Json<ListingMarkerCountResponse>, ProblemResponse> {
    let filter = ListingMarkerFilter::try_from_hash(&query.filter_hash).map_err(|_| {
        problem(
            "map/listing-marker-filter-not-found",
            "listing marker filter was not found",
            StatusCode::NOT_FOUND,
            None,
        )
    })?;
    let count = state
        .listing_repo
        .count_listing_markers(filter.into_spec())
        .await
        .map_err(|e| {
            problem(
                "map/listing-marker-count-unavailable",
                "listing marker count is unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                Some(e.to_string()),
            )
        })?;

    Ok(axum::Json(ListingMarkerCountResponse {
        total_count: count.total_count,
        projection_version: count.projection_version,
        anchor_snapshot_id: count.anchor_snapshot_id,
    }))
}
```

- [x] **Step 6: Wire route**

In `services/api/src/main.rs`, add module and route:

```rust
pub mod listing_marker_counts;
```

```rust
.route(
    "/map/v1/marker-counts/listing",
    get(routes::listing_marker_counts::get_listing_marker_count),
)
```

- [x] **Step 7: Run focused checks**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_count
cargo check -p api
```

Expected: count path compiles and DB test passes.

## Task 6: Base Marker Tile Frontend Instant Filter

**Files:**
- Create: `apps/web/lib/map/listing-marker-filter.ts`
- Create: `apps/web/lib/map/listing-marker-filter.test.ts`
- Modify: `apps/web/lib/map/marker-tile-style.ts`
- Modify: `apps/web/components/listings/listing-map.tsx`

- [x] **Step 1: Write failing frontend tests**

Create `apps/web/lib/map/listing-marker-filter.test.ts`:

```ts
// @vitest-environment node
import { describe, expect, it } from "vitest";
import { buildListingMarkerLayerFilter } from "@/lib/map/listing-marker-filter";

describe("buildListingMarkerLayerFilter", () => {
  it("returns no-op all filter when fast filters are empty", () => {
    expect(
      buildListingMarkerLayerFilter({
        types: [],
        transactions: [],
        minAreaM2: undefined,
        maxAreaM2: undefined,
        minPriceKrw: undefined,
        maxPriceKrw: undefined,
        sort: "created_at_desc",
        adminCode: undefined,
        landUseType: undefined,
      }),
    ).toEqual(["all"]);
  });

  it("builds type transaction price and area predicates", () => {
    expect(
      buildListingMarkerLayerFilter({
        types: ["factory", "industrial_land"],
        transactions: ["sale"],
        minAreaM2: 300,
        maxAreaM2: 1000,
        minPriceKrw: 100_000_000,
        maxPriceKrw: 5_000_000_000,
        sort: "created_at_desc",
        adminCode: undefined,
        landUseType: undefined,
      }),
    ).toEqual([
      "all",
      ["in", ["get", "listing_type"], ["literal", ["factory", "industrial_land"]]],
      ["in", ["get", "transaction_type"], ["literal", ["sale"]]],
      [">=", ["to-number", ["get", "area_m2"]], 300],
      ["<=", ["to-number", ["get", "area_m2"]], 1000],
      [">=", ["to-number", ["get", "price_krw"]], 100_000_000],
      ["<=", ["to-number", ["get", "price_krw"]], 5_000_000_000],
    ]);
  });
});
```

- [x] **Step 2: Run failing test**

Run:

```bash
pnpm --filter @gongzzang/web test -- lib/map/listing-marker-filter.test.ts
```

Expected: failure because helper does not exist.

- [x] **Step 3: Implement instant filter helper**

Create `apps/web/lib/map/listing-marker-filter.ts`:

```ts
import type { ListingFilters } from "@/lib/listings/filters";

type MapboxFilterExpression = unknown[];

export function buildListingMarkerLayerFilter(filters: ListingFilters): MapboxFilterExpression {
  const clauses: MapboxFilterExpression = ["all"];

  if (filters.types.length > 0) {
    clauses.push(["in", ["get", "listing_type"], ["literal", filters.types]]);
  }
  if (filters.transactions.length > 0) {
    clauses.push(["in", ["get", "transaction_type"], ["literal", filters.transactions]]);
  }
  if (filters.minAreaM2 !== undefined) {
    clauses.push([">=", ["to-number", ["get", "area_m2"]], filters.minAreaM2]);
  }
  if (filters.maxAreaM2 !== undefined) {
    clauses.push(["<=", ["to-number", ["get", "area_m2"]], filters.maxAreaM2]);
  }
  if (filters.minPriceKrw !== undefined) {
    clauses.push([">=", ["to-number", ["get", "price_krw"]], filters.minPriceKrw]);
  }
  if (filters.maxPriceKrw !== undefined) {
    clauses.push(["<=", ["to-number", ["get", "price_krw"]], filters.maxPriceKrw]);
  }

  return clauses;
}
```

- [x] **Step 4: Wire `setFilter` into listing map**

Extend `MapboxGLLike` in `apps/web/components/listings/listing-map.tsx`:

```ts
setFilter?: (layerId: string, filter: unknown[]) => void;
```

Import filters and helper:

```ts
import { buildListingMarkerLayerFilter } from "@/lib/map/listing-marker-filter";
import { useListingsStore } from "@/stores/listings";
```

Inside `ListingMap`, subscribe to fast filters:

```ts
const filters = useListingsStore((s) => s.filters);
```

Store the Mapbox bridge handle in a React ref and apply the filter from that ref:

```ts
const mapboxRef = useRef<MapboxGLLike | null>(null);
```

Change `setupMapboxRuntime` so it accepts an `onMapboxReady` callback:

```ts
function setupMapboxRuntime(
  map: NaverMapLike,
  onMapboxReady: (mb: MapboxGLLike) => void,
): void {
  void waitForMapbox(map).then((mb) => {
    onMapboxReady(mb);
    addListingMarkerTileSources(mb);
    addListingMarkerTileLayers(mb);
  });
}
```

Then wire the effect:

```ts
useEffect(() => {
  const mb = mapboxRef.current;
  if (!mb?.setFilter || !mb.getLayer?.(LISTING_MARKER_TILE_CIRCLE_LAYER_ID)) return;
  mb.setFilter(LISTING_MARKER_TILE_CIRCLE_LAYER_ID, buildListingMarkerLayerFilter(filters));
}, [filters]);
```

- [x] **Step 5: Run frontend tests**

Run:

```bash
pnpm --filter @gongzzang/web test -- lib/map/listing-marker-filter.test.ts tests/unit/map/marker-tile-style.test.ts
```

Expected: instant filter helper tests pass and marker style tests remain green.
