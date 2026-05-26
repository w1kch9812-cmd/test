# Listing Marker Serving Index And Filter Mask Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build scalable Gongzzang listing marker serving so common filters apply instantly in the browser while exact counts, unseen tiles, and advanced filters are backed by a server-side marker projection/index.

**Architecture:** Extend the existing PNU-anchor listing PBF path from `all-active-v1` into a typed normalized filter model. Keep platform-core as PNU anchor SSOT, make Gongzzang own listing marker projection/index, expose base marker tiles with safe filter properties, add count/mask companion APIs, and apply fast map filters through the map layer before server results arrive.

**Tech Stack:** Rust, Axum, SQLx, PostgreSQL/PostGIS, Mapbox GL source/layer API through Naver GL bridge, Next.js 16, React 19, Zustand, Vitest.

---

## File Structure

Backend domain:

- Create: `crates/domain/core/listing/src/marker_filter.rs`
  - Owns typed marker filter input, canonicalization, stable `filter_hash`, and parse errors.
- Modify: `crates/domain/core/listing/src/lib.rs`
  - Exposes the new module through the listing-domain crate.
- Modify: `crates/domain/core/listing/src/repository.rs`
  - Keeps repository trait, tile query/count/mask DTOs, and re-exports marker filter types.
- Modify: `crates/domain/core/listing/Cargo.toml`
  - Adds existing workspace dependencies needed by the new filter module.

Backend database:

- Create: `migrations/30013_listing_marker_projection.sql`
  - Adds `listing_marker_projection`, the map-serving read model.
  - This is a DB schema change. Execution requires explicit user approval before applying.
- Modify: `tests/migrations/test_v001_full.sh`
  - Verifies projection table/indexes and no listing-owned coordinate regression.
- Modify: `crates/db/src/listing.rs`
  - Registers focused modules.
- Create: `crates/db/src/listing/marker_projection.rs`
  - Upserts projection rows from listing + `parcel_marker_anchor`.
- Modify: `crates/db/src/listing/marker_tile.rs`
  - Reads from `listing_marker_projection`, emits safe marker properties.
- Create: `crates/db/src/listing/marker_count.rs`
  - Exact count/facet read path for current normalized filters.
- Create: `crates/db/src/listing/marker_mask.rs`
  - Optional filter mask read path keyed by tile/filter/projection version.
- Modify: `crates/db/src/listing/repository.rs`
  - Implements the new repository methods.
- Modify: `crates/db/tests/listing_marker_tile_integration.rs`
  - Adds projection-backed tile/count/mask integration tests.

Backend API:

- Modify: `services/api/src/routes/listing_marker_tiles.rs`
  - Accepts stable registered filter hashes and surfaces projection metadata headers.
- Create: `services/api/src/routes/listing_marker_filters.rs`
  - Normalizes filter payloads and returns `filter_hash`.
- Create: `services/api/src/routes/listing_marker_counts.rs`
  - Returns exact counts for normalized filters.
- Create: `services/api/src/routes/listing_marker_masks.rs`
  - Returns optional show/hide marker id masks.
- Modify: `services/api/src/main.rs`
  - Wires new public map routes.

Frontend:

- Modify: `apps/web/lib/routes.ts`
  - Adds API proxy route constants for filter registration, counts, and masks.
- Modify: `apps/web/lib/listings/filters.ts`
  - Adds canonical fast-filter helpers shared by URL, map filter, and server payloads.
- Create: `apps/web/lib/map/listing-marker-filter.ts`
  - Builds Mapbox layer filter expressions from visible fast filters.
- Modify: `apps/web/lib/map/marker-tile-contract.ts`
  - Allows stable listing filter hashes beyond `all-active-v1`.
- Modify: `apps/web/lib/map/marker-tile-style.ts`
  - Includes paint/filter helpers for listing marker features with type/transaction/price/area.
- Modify: `apps/web/components/listings/listing-map.tsx`
  - Applies browser instant filters with `setFilter` and wires server authoritative refresh hooks.
- Modify: `apps/web/stores/listings.ts`
  - Separates fast applied filters from advanced modal draft filters.
- Modify: `apps/web/components/listings/filter-bar.tsx`
  - Keeps fast filters immediate.
- Create: `apps/web/lib/map/listing-marker-filter.test.ts`
  - Unit tests for instant filter expression generation.
- Modify: `apps/web/tests/unit/map/marker-tile-contract.test.ts`
- Modify: `apps/web/tests/unit/map/marker-tile-style.test.ts`
- Modify: `apps/web/tests/unit/listings/filters.test.ts`

Guardrails/docs:

- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`
- Modify: `docs/frontend/listings-search.md`
- Modify: `docs/superpowers/next-actions.md`

## Task 1: Domain Filter Contract

**Files:**
- Create: `crates/domain/core/listing/src/marker_filter.rs`
- Modify: `crates/domain/core/listing/src/lib.rs`
- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/domain/core/listing/Cargo.toml`

- [x] **Step 1: Add failing unit tests for canonicalization**

Create `crates/domain/core/listing/src/marker_filter.rs` with tests first. The module should compile-fail until the types are implemented.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use shared_kernel::listing_type::ListingType;
    use shared_kernel::transaction_type::TransactionType;

    #[test]
    fn equivalent_filter_order_produces_same_hash() {
        let first = ListingMarkerFilterSpec {
            types: vec![ListingType::Warehouse, ListingType::Factory],
            transactions: vec![TransactionType::Sale, TransactionType::Jeonse],
            min_area_m2: Some(100),
            max_area_m2: Some(5000),
            min_price_krw: None,
            max_price_krw: Some(5_000_000_000),
        };
        let second = ListingMarkerFilterSpec {
            types: vec![ListingType::Factory, ListingType::Warehouse],
            transactions: vec![TransactionType::Jeonse, TransactionType::Sale],
            min_area_m2: Some(100),
            max_area_m2: Some(5000),
            min_price_krw: None,
            max_price_krw: Some(5_000_000_000),
        };

        let first = match first.try_normalized() {
            Ok(value) => value,
            Err(err) => panic!("valid first filter rejected: {err}"),
        };
        let second = match second.try_normalized() {
            Ok(value) => value,
            Err(err) => panic!("valid second filter rejected: {err}"),
        };

        assert_eq!(first.filter_hash(), second.filter_hash());
    }

    #[test]
    fn all_active_v1_stays_supported() {
        let filter = match ListingMarkerFilter::try_from_hash("all-active-v1") {
            Ok(value) => value,
            Err(err) => panic!("all-active-v1 rejected: {err}"),
        };

        assert_eq!(filter.hash(), "all-active-v1");
        assert_eq!(filter.into_spec().types, Vec::<ListingType>::new());
    }

    #[test]
    fn invalid_range_is_rejected() {
        let err = ListingMarkerFilterSpec {
            types: vec![],
            transactions: vec![],
            min_area_m2: Some(5000),
            max_area_m2: Some(100),
            min_price_krw: None,
            max_price_krw: None,
        }
        .try_normalized()
        .expect_err("invalid range");

        assert!(err.to_string().contains("min_area_m2"));
    }
}
```

- [x] **Step 2: Run the failing tests**

Run:

```bash
cargo test -p listing-domain marker_filter
```

Expected: failure because `ListingMarkerFilterSpec`, `ListingMarkerFilter`, and range validation are not implemented.

- [x] **Step 3: Implement the filter module**

Implement the module with these public types:

```rust
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;
use thiserror::Error;

pub const ALL_ACTIVE_LISTING_MARKER_FILTER_HASH: &str = "all-active-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingMarkerFilterSpec {
    pub types: Vec<ListingType>,
    pub transactions: Vec<TransactionType>,
    pub min_area_m2: Option<i64>,
    pub max_area_m2: Option<i64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedListingMarkerFilterSpec {
    pub types: Vec<ListingType>,
    pub transactions: Vec<TransactionType>,
    pub min_area_m2: Option<i64>,
    pub max_area_m2: Option<i64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListingMarkerFilter {
    AllActive,
    Normalized(NormalizedListingMarkerFilterSpec),
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ListingMarkerFilterError {
    #[error("unsupported listing marker filter hash: {0}")]
    UnsupportedHash(String),
    #[error("invalid listing marker filter range: {field}")]
    InvalidRange { field: &'static str },
}
```

Normalization rules:

```rust
impl ListingMarkerFilterSpec {
    pub fn try_normalized(self) -> Result<NormalizedListingMarkerFilterSpec, ListingMarkerFilterError> {
        validate_range("min_area_m2", self.min_area_m2, self.max_area_m2)?;
        validate_range("min_price_krw", self.min_price_krw, self.max_price_krw)?;

        let mut types = self.types;
        types.sort_by_key(|v| v.as_str().to_owned());
        types.dedup();

        let mut transactions = self.transactions;
        transactions.sort_by_key(|v| v.as_str().to_owned());
        transactions.dedup();

        Ok(NormalizedListingMarkerFilterSpec {
            types,
            transactions,
            min_area_m2: self.min_area_m2,
            max_area_m2: self.max_area_m2,
            min_price_krw: self.min_price_krw,
            max_price_krw: self.max_price_krw,
        })
    }
}
```

Stable hash rule:

```rust
impl NormalizedListingMarkerFilterSpec {
    #[must_use]
    pub fn filter_hash(&self) -> String {
        if self.is_all_active() {
            return ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned();
        }

        let canonical = format!(
            "v1|types={}|tx={}|area={}:{}|price={}:{}",
            self.types.iter().map(ListingType::as_str).collect::<Vec<_>>().join(","),
            self.transactions.iter().map(TransactionType::as_str).collect::<Vec<_>>().join(","),
            opt_i64(self.min_area_m2),
            opt_i64(self.max_area_m2),
            opt_i64(self.min_price_krw),
            opt_i64(self.max_price_krw),
        );
        let digest = Sha256::digest(canonical.as_bytes());
        format!("lst_filter_v1_{:x}", digest)
    }

    #[must_use]
    pub fn is_all_active(&self) -> bool {
        self.types.is_empty()
            && self.transactions.is_empty()
            && self.min_area_m2.is_none()
            && self.max_area_m2.is_none()
            && self.min_price_krw.is_none()
            && self.max_price_krw.is_none()
    }
}
```

Filter hash parsing must not reconstruct arbitrary historical hashes from the hash string alone. The API
registers normalized filter payloads and stores the hash/spec mapping in the serving layer. `all-active-v1`
is the only built-in hash because it has no payload:

```rust
impl ListingMarkerFilter {
    pub fn try_from_hash(value: &str) -> Result<Self, ListingMarkerFilterError> {
        if value == ALL_ACTIVE_LISTING_MARKER_FILTER_HASH {
            return Ok(Self::AllActive);
        }
        Err(ListingMarkerFilterError::UnsupportedHash(value.to_owned()))
    }

    #[must_use]
    pub fn hash(&self) -> String {
        match self {
            Self::AllActive => ALL_ACTIVE_LISTING_MARKER_FILTER_HASH.to_owned(),
            Self::Normalized(spec) => spec.filter_hash(),
        }
    }

    #[must_use]
    pub fn into_spec(self) -> NormalizedListingMarkerFilterSpec {
        match self {
            Self::AllActive => NormalizedListingMarkerFilterSpec {
                types: Vec::new(),
                transactions: Vec::new(),
                min_area_m2: None,
                max_area_m2: None,
                min_price_krw: None,
                max_price_krw: None,
            },
            Self::Normalized(spec) => spec,
        }
    }
}
```

- [x] **Step 4: Wire module exports**

Update `crates/domain/core/listing/src/lib.rs`:

```rust
pub mod marker_filter;
```

Update `crates/domain/core/listing/src/repository.rs` to import and re-export marker filter types from the module instead of defining `ListingMarkerFilter` inline. Keep `LISTING_MARKER_TILE_LAYER`, `LISTING_MARKER_TILE_CONTENT_TYPE`, and tile query types in `repository.rs`.

- [x] **Step 5: Add crate dependencies**

Update `crates/domain/core/listing/Cargo.toml`:

```toml
sha2 = { workspace = true }
```

`serde` and `thiserror` are already present.

- [x] **Step 6: Run domain tests**

Run:

```bash
cargo test -p listing-domain marker_filter
```

Expected: marker filter tests pass.

## Task 2: Projection Migration

**Files:**
- Create: `migrations/30013_listing_marker_projection.sql`
- Modify: `tests/migrations/test_v001_full.sh`

This task is a DB schema change. Stop before executing it unless the user has explicitly approved creating migration `30013_listing_marker_projection.sql`.

- [x] **Step 1: Add migration smoke assertions first**

Modify `tests/migrations/test_v001_full.sh`:

```bash
EXPECTED_TABLES=(
  # existing entries
  listing_marker_projection
)

LMP_SRID_CHECK=$(psql "$DATABASE_URL" -t -A -c "select 1 from pg_constraint where conrelid='listing_marker_projection'::regclass and conname='listing_marker_projection_anchor_srid_chk';")
if [ "$LMP_SRID_CHECK" != "1" ]; then
  echo "FAIL: listing_marker_projection_anchor_srid_chk missing" >&2
  exit 1
fi

LMP_TILE_IDX=$(psql "$DATABASE_URL" -t -A -c "select count(*) from pg_indexes where schemaname='public' and tablename='listing_marker_projection' and indexname='listing_marker_projection_z14_tile_idx';")
if [ "$LMP_TILE_IDX" != "1" ]; then
  echo "FAIL: listing_marker_projection_z14_tile_idx missing" >&2
  exit 1
fi

LMP_LNG_LAT_COLUMNS=$(psql "$DATABASE_URL" -t -A -c "select count(*) from information_schema.columns where table_schema='public' and table_name='listing_marker_projection' and column_name in ('listing_lng', 'listing_lat', 'geom_point');")
if [ "$LMP_LNG_LAT_COLUMNS" != "0" ]; then
  echo "FAIL: listing_marker_projection must not introduce listing-owned coordinate columns, got $LMP_LNG_LAT_COLUMNS" >&2
  exit 1
fi
```

- [x] **Step 2: Run migration smoke and confirm failure**

Run:

```bash
bash tests/migrations/test_v001_full.sh
```

Expected: fails because `listing_marker_projection` does not exist.

- [x] **Step 3: Add projection migration**

Create `migrations/30013_listing_marker_projection.sql`:

```sql
-- Gongzzang listing marker map-serving projection.
--
-- This table is not the listing source of truth. It is a read model fed by listing
-- publish/update/withdraw events and platform-core parcel_marker_anchor snapshots.

create table listing_marker_projection (
    marker_id varchar(64) primary key,
    listing_id varchar(64) not null unique references listing(id) on delete cascade,
    pnu char(19) not null,
    anchor_point geometry(Point, 4326) not null,
    anchor_snapshot_id varchar(128) not null,
    source_geometry_version varchar(128) not null,
    projection_version bigint not null default 1,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    listing_status varchar(32) not null,
    visibility_scope varchar(32) not null default 'public',
    listing_type varchar(40) not null,
    transaction_type varchar(32) not null,
    price_krw bigint not null,
    area_m2 bigint not null,
    rank_score integer not null default 0,
    updated_at timestamptz not null default now(),
    constraint listing_marker_projection_pnu_format_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_projection_anchor_srid_chk
        check (ST_SRID(anchor_point) = 4326),
    constraint listing_marker_projection_scope_chk
        check (visibility_scope in ('public', 'authenticated', 'owner_private')),
    constraint listing_marker_projection_status_chk
        check (listing_status in ('draft', 'pending_review', 'active', 'sold', 'expired')),
    constraint listing_marker_projection_area_positive_chk
        check (area_m2 >= 0),
    constraint listing_marker_projection_price_nonnegative_chk
        check (price_krw >= 0)
);

create index listing_marker_projection_anchor_gist_idx
    on listing_marker_projection using gist(anchor_point);

create index listing_marker_projection_z14_tile_idx
    on listing_marker_projection(z14_tile_x, z14_tile_y, listing_status, visibility_scope);

create index listing_marker_projection_type_tx_idx
    on listing_marker_projection(listing_type, transaction_type)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_price_idx
    on listing_marker_projection(price_krw)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_area_idx
    on listing_marker_projection(area_m2)
    where listing_status = 'active' and visibility_scope = 'public';

create index listing_marker_projection_version_idx
    on listing_marker_projection(projection_version desc, updated_at desc);
```

- [x] **Step 4: Run migration smoke**

Run:

```bash
bash tests/migrations/test_v001_full.sh
```

Expected: full migration chain passes with `listing_marker_projection` checks.

## Task 3: Projection Upsert Read Model

**Files:**
- Modify: `crates/db/src/listing.rs`
- Create: `crates/db/src/listing/marker_projection.rs`
- Modify: `crates/db/src/listing/repository.rs`
- Modify: `crates/db/tests/listing_marker_tile_integration.rs`

- [x] **Step 1: Write failing integration test for projection upsert**

Add this test to `crates/db/tests/listing_marker_tile_integration.rs`:

```rust
#[tokio::test]
async fn listing_marker_projection_upsert_uses_platform_core_anchor_snapshot() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-projection-1", "marker-projection-1@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100090000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Projection listing");
    activate_listing(&repo, &mut listing, &owner).await;

    repo.upsert_listing_marker_projection(&listing.id).await.unwrap();

    let row = sqlx::query(
        "select listing_id, pnu, anchor_snapshot_id, listing_status, listing_type, transaction_type, price_krw, area_m2 \
         from listing_marker_projection where listing_id = $1",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("listing_id"), listing.id.as_str());
    assert_eq!(row.get::<String, _>("pnu"), pnu);
    assert_eq!(row.get::<String, _>("anchor_snapshot_id"), "snapshot-test-v1");
    assert_eq!(row.get::<String, _>("listing_status"), "active");
    assert_eq!(row.get::<i64, _>("price_krw"), 500_000_000);
}
```

- [x] **Step 2: Run test and confirm failure**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_upsert
```

Expected: failure because `upsert_listing_marker_projection` is not implemented.

- [x] **Step 3: Add repository trait method**

Add to `ListingRepository` in `crates/domain/core/listing/src/repository.rs`:

```rust
async fn upsert_listing_marker_projection(
    &self,
    id: &Id<ListingIdMarker>,
) -> Result<(), RepoError>;
```

- [x] **Step 4: Implement projection module**

Create `crates/db/src/listing/marker_projection.rs`:

```rust
use listing_domain::repository::RepoError;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use sqlx::PgPool;

use crate::error_map::map_sqlx_err;

pub(super) async fn upsert_listing_marker_projection(
    pool: &PgPool,
    id: &Id<ListingIdMarker>,
) -> Result<(), RepoError> {
    let result = sqlx::query(
        r"
        insert into listing_marker_projection (
            marker_id,
            listing_id,
            pnu,
            anchor_point,
            anchor_snapshot_id,
            source_geometry_version,
            projection_version,
            z14_tile_x,
            z14_tile_y,
            listing_status,
            visibility_scope,
            listing_type,
            transaction_type,
            price_krw,
            area_m2,
            rank_score,
            updated_at
        )
        select
            'lm_' || l.id,
            l.id,
            l.parcel_pnu,
            a.anchor_point,
            a.anchor_snapshot_id,
            a.source_geometry_version,
            extract(epoch from now())::bigint,
            floor((ST_X(ST_Transform(a.anchor_point, 3857)) + 20037508.342789244) / (40075016.68557849 / 16384))::int,
            floor((20037508.342789244 - ST_Y(ST_Transform(a.anchor_point, 3857))) / (40075016.68557849 / 16384))::int,
            l.status,
            'public',
            l.listing_type,
            l.transaction_type,
            l.price_krw,
            floor(l.area_m2)::bigint,
            0,
            now()
        from listing l
        join parcel_marker_anchor a on a.pnu = l.parcel_pnu
        where l.id = $1
        on conflict (listing_id) do update set
            pnu = excluded.pnu,
            anchor_point = excluded.anchor_point,
            anchor_snapshot_id = excluded.anchor_snapshot_id,
            source_geometry_version = excluded.source_geometry_version,
            projection_version = excluded.projection_version,
            z14_tile_x = excluded.z14_tile_x,
            z14_tile_y = excluded.z14_tile_y,
            listing_status = excluded.listing_status,
            visibility_scope = excluded.visibility_scope,
            listing_type = excluded.listing_type,
            transaction_type = excluded.transaction_type,
            price_krw = excluded.price_krw,
            area_m2 = excluded.area_m2,
            rank_score = excluded.rank_score,
            updated_at = excluded.updated_at
        ",
    )
    .bind(id.as_str())
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    if result.rows_affected() == 0 {
        return Err(RepoError::NotFound);
    }
    Ok(())
}
```

- [x] **Step 5: Register module and trait implementation**

Update `crates/db/src/listing.rs`:

```rust
mod marker_projection;
```

Update `crates/db/src/listing/repository.rs`:

```rust
#[instrument(skip(self), fields(listing_id = %id.as_str()))]
async fn upsert_listing_marker_projection(
    &self,
    id: &Id<ListingIdMarker>,
) -> Result<(), RepoError> {
    marker_projection::upsert_listing_marker_projection(&self.pool, id).await
}
```

- [x] **Step 6: Run projection test**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_upsert
```

Expected: projection row is created from listing + anchor.

## Task 4: Projection-Backed Marker Tile

**Files:**
- Modify: `crates/db/src/listing/marker_tile.rs`
- Modify: `crates/db/tests/listing_marker_tile_integration.rs`
- Modify: `apps/web/tests/unit/map/marker-tile-style.test.ts`

- [x] **Step 1: Add failing integration test for safe feature fields**

Add an assertion that the returned tile is generated from projection rows and includes safe filter fields:

```rust
#[tokio::test]
async fn listing_marker_tile_uses_projection_safe_filter_properties() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-tile-fields", "marker-tile-fields@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100100000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Marker field listing");
    activate_listing(&repo, &mut listing, &owner).await;
    repo.upsert_listing_marker_projection(&listing.id).await.unwrap();

    let tile = repo
        .find_listing_marker_tile(ListingMarkerTileQuery::new(
            0,
            0,
            0,
            ListingMarkerFilter::AllActive,
        ))
        .await
        .unwrap();

    assert_eq!(tile.eligible_count, 1);
    assert_eq!(tile.represented_count, 1);
    assert_eq!(tile.feature_count, 1);
    assert_eq!(tile.anchor_snapshot_id.as_deref(), Some("snapshot-test-v1"));
}
```

- [x] **Step 2: Run test and confirm failure**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_tile_uses_projection_safe_filter_properties
```

Expected: failure until marker tile reads from `listing_marker_projection`.

- [x] **Step 3: Replace OLTP read with projection read**

Update `crates/db/src/listing/marker_tile.rs` SQL so `eligible` reads from `listing_marker_projection p`.

The represented feature select must include:

```sql
marker_id,
listing_id,
pnu,
'listing'::text as kind,
1::int4 as count,
rank_score as rank,
listing_id::text as detail_ref,
anchor_snapshot_id,
projection_version,
listing_type,
transaction_type,
price_krw,
area_m2
```

Filter conditions for `AllActive`:

```sql
where p.listing_status = 'active'
  and p.visibility_scope = 'public'
  and ST_Intersects(
      ST_Transform(p.anchor_point, 3857),
      ST_TileEnvelope($1, $2, $3)
  )
```

Keep the completeness check:

```rust
if eligible_count != represented_count {
    return Err(RepoError::Database(format!(
        "listing marker tile completeness violation: eligible_count={eligible_count}, represented_count={represented_count}"
    )));
}
```

- [x] **Step 4: Run focused tile tests**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_tile
```

Expected: existing all-active tests and new projection-backed field test pass.

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
