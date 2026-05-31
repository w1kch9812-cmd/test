# Listing Marker Serving Index And Filter Mask Plan - Part 2: Projection And Tiles

> Extracted from `2026-05-26-listing-marker-serving-index-filter-mask.md` to keep each plan file below the 500-line SSS guardrail.
> See the index file for the full sequence and cross-links.

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
