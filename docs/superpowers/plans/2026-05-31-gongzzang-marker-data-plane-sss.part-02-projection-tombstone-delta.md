# Gongzzang Marker Data Plane SSS Plan - Part 02: Projection, Tombstone, And Delta

Parent index: [Gongzzang Marker Data Plane SSS Implementation Plan](./2026-05-31-gongzzang-marker-data-plane-sss.md).


## Task 3: Write Tombstone And Delta Logs From Projection Sync

**Files:**

- Modify: `crates/db/src/listing/marker_projection.rs`
- Test: `crates/db/tests/listing_marker_tile_integration.rs`

- [ ] **Step 1: Add failing integration tests**

Add two tests:

```rust
#[tokio::test]
async fn listing_marker_projection_writes_tombstone_when_public_listing_becomes_sold() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-tombstone", "marker-tombstone@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100160000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Tombstone listing");
    activate_listing(&repo, &mut listing, &owner).await;

    listing.mark_sold(Utc::now()).unwrap();
    repo.save(
        &listing,
        MutationContext::new_user_action(owner.clone(), "corr-marker-sold", "mark_sold"),
    )
    .await
    .unwrap();

    let row = sqlx::query(
        r"
        select marker_id, reason, projection_version
        from listing_marker_tombstone_log
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("marker_id"), format!("lm_{}", listing.id.as_str()));
    assert_eq!(row.get::<String, _>("reason"), "sold");
    assert_eq!(row.get::<i64, _>("projection_version"), 2);
}

#[tokio::test]
async fn listing_marker_projection_writes_delta_when_listing_becomes_public() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-delta", "marker-delta@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100170000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Delta listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let row = sqlx::query(
        r"
        select marker_id, change_kind, projection_version
        from listing_marker_delta_log
        where listing_id = $1
        ",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(row.get::<String, _>("marker_id"), format!("lm_{}", listing.id.as_str()));
    assert_eq!(row.get::<String, _>("change_kind"), "became_public");
    assert_eq!(row.get::<i64, _>("projection_version"), 1);
}
```

- [ ] **Step 2: Run failing tests**

Run:

```powershell
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_writes_
```

Expected: FAIL because projection sync does not write overlay logs.

- [ ] **Step 3: Update projection SQL**

Modify `sync_listing_marker_projection` to capture previous row state before upsert. The implementation should:

- identify `old_public = existing.listing_status = 'active' and existing.visibility_scope = 'public'`;
- identify `new_public = l.status = 'active'`;
- insert tombstone when `old_public and not new_public`;
- insert delta when `new_public and (existing is null or source_listing_version changed)`;
- enqueue dirty tiles for z14 and parent aggregate zooms.

Use `ON CONFLICT DO NOTHING` for overlay log idempotency.

- [ ] **Step 4: Run integration tests**

Run:

```powershell
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_writes_
```

Expected: PASS.

---

## Task 4: Add Tombstone Repository And API

**Files:**

- Create: `crates/db/src/listing/marker_tombstone.rs`
- Modify: `crates/db/src/listing.rs`
- Modify: `crates/db/src/listing/repository.rs`
- Create: `services/api/src/routes/listing_marker_tombstones.rs`
- Modify: `services/api/src/routes/mod.rs`
- Modify: `services/api/src/listing_marker_serving.rs`

- [ ] **Step 1: Add DB repository test**

Add to `filter_index.rs`:

```rust
#[tokio::test]
async fn listing_marker_tombstones_returns_ids_for_loaded_tile() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-tombstone-api", "marker-tombstone-api@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100180000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Tombstone API listing");
    activate_listing(&repo, &mut listing, &owner).await;
    listing.mark_sold(chrono::Utc::now()).unwrap();
    repo.save(
        &listing,
        shared_kernel::mutation::MutationContext::new_user_action(
            owner.clone(),
            "corr-marker-tombstone-api",
            "mark_sold",
        ),
    )
    .await
    .unwrap();

    let tombstones = repo
        .find_listing_marker_tombstones(listing_domain::repository::ListingMarkerOverlayTileQuery::try_new(0, 0, 0, None).unwrap())
        .await
        .unwrap();

    assert_eq!(tombstones.marker_ids, vec![format!("lm_{}", listing.id.as_str())]);
}
```

- [ ] **Step 2: Implement DB query**

Create `marker_tombstone.rs`:

```rust
use listing_domain::repository::{
    ListingMarkerOverlayTileQuery, ListingMarkerTombstones, RepoError,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn find_listing_marker_tombstones(
    pool: &PgPool,
    query: ListingMarkerOverlayTileQuery,
) -> Result<ListingMarkerTombstones, RepoError> {
    let row = sqlx::query(
        r"
        with matching as (
            select marker_id, projection_version, anchor_snapshot_id
            from listing_marker_tombstone_log
            where expires_at > now()
              and ($4::bigint is null or projection_version > $4::bigint)
              and ST_Intersects(
                  ST_Transform(
                      ST_SetSRID(ST_MakePoint(
                          ((z14_tile_x::float8 + 0.5) / 16384.0) * 360.0 - 180.0,
                          degrees(atan(sinh(pi() * (1.0 - 2.0 * ((z14_tile_y::float8 + 0.5) / 16384.0)))))
                      ), 4326),
                      3857
                  ),
                  ST_TileEnvelope($1, $2, $3)
              )
        )
        select
            coalesce(array_agg(marker_id order by marker_id), array[]::text[]) as marker_ids,
            max(projection_version)::int8 as projection_version,
            max(anchor_snapshot_id) as anchor_snapshot_id
        from matching
        ",
    )
    .bind(i32::from(query.z))
    .bind(i32::try_from(query.x).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(i32::try_from(query.y).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(query.base_version)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerTombstones {
        marker_ids: row.try_get("marker_ids").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
```

- [ ] **Step 3: Wire repository**

Add module import in `crates/db/src/listing.rs`:

```rust
mod marker_tombstone;
```

Add method in `crates/db/src/listing/repository.rs`:

```rust
    async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError> {
        marker_tombstone::find_listing_marker_tombstones(&self.pool, query).await
    }
```

- [ ] **Step 4: Add API response**

Create route returning JSON:

```json
{
  "encoding": "hide",
  "marker_ids": ["lm_lst_..."],
  "projection_version": 2,
  "anchor_snapshot_id": "snapshot-test-v1"
}
```

Route:

```text
GET /map/v1/marker-tombstones/listing/{z}/{x}/{y}?base_version={version}
```

- [ ] **Step 5: Run tests**

Run:

```powershell
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_tombstones
cargo test -p api listing_marker_tombstone
```

Expected: PASS.

---

## Task 5: Add Delta Repository And API

**Files:**

- Create: `crates/db/src/listing/marker_delta.rs`
- Modify: `crates/db/src/listing.rs`
- Modify: `crates/db/src/listing/repository.rs`
- Create: `services/api/src/routes/listing_marker_deltas.rs`
- Modify: `services/api/src/routes/mod.rs`
- Modify: `services/api/src/listing_marker_serving.rs`

- [ ] **Step 1: Add DB test**

Add to `filter_index.rs`:

```rust
#[tokio::test]
async fn listing_marker_deltas_returns_recent_public_features_for_loaded_tile() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-delta-api", "marker-delta-api@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100210000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Delta API listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let deltas = repo
        .find_listing_marker_deltas(listing_domain::repository::ListingMarkerOverlayTileQuery::try_new(0, 0, 0, None).unwrap())
        .await
        .unwrap();

    assert!(!deltas.bytes.is_empty());
    assert_eq!(deltas.layer_name, "listing_delta");
    assert_eq!(deltas.feature_count, 1);
    assert_eq!(deltas.projection_version, Some(1));
    assert_eq!(deltas.anchor_snapshot_id.as_deref(), Some("snapshot-test-v1"));
}
```

- [ ] **Step 2: Implement delta MVT query**

The delta query joins `listing_marker_delta_log` to `listing_marker_projection`, filters by active
unexpired delta records, applies `base_version`, and encodes a `listing_delta` MVT layer with the
same safe fields as the base listing marker tile.

Required SQL shape:

```sql
with matching as (
    select
        p.marker_id,
        p.listing_id,
        p.pnu,
        p.anchor_point,
        p.anchor_snapshot_id,
        p.projection_version,
        p.listing_type,
        p.transaction_type,
        p.price_krw,
        p.area_m2,
        p.rank_score
    from listing_marker_delta_log d
    join listing_marker_projection p on p.marker_id = d.marker_id
    where d.expires_at > now()
      and p.listing_status = 'active'
      and p.visibility_scope = 'public'
      and ($4::bigint is null or p.projection_version > $4::bigint)
      and ST_Intersects(ST_Transform(p.anchor_point, 3857), ST_TileEnvelope($1, $2, $3))
),
features as (
    select
        marker_id as id,
        pnu,
        'listing_delta'::text as kind,
        1::int4 as count,
        rank_score as rank,
        listing_id::text as detail_ref,
        projection_version,
        anchor_snapshot_id,
        ST_AsMVTGeom(
            ST_Transform(anchor_point, 3857),
            ST_TileEnvelope($1, $2, $3),
            4096,
            256,
            true
        ) as geom
    from matching
)
select
    coalesce((select ST_AsMVT(features, 'listing_delta', 4096, 'geom') from features), '\x'::bytea) as bytes,
    (select count(*)::int8 from features where geom is not null) as feature_count,
    (select max(projection_version)::int8 from matching) as projection_version,
    (select max(anchor_snapshot_id) from matching) as anchor_snapshot_id
```

- [ ] **Step 3: Add API route**

Route:

```text
GET /map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf?base_version={version}
```

Headers:

```text
Content-Type: application/vnd.mapbox-vector-tile
Cache-Control: public, max-age=5, stale-while-revalidate=10
```

- [ ] **Step 4: Run tests**

Run:

```powershell
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_delta
cargo test -p api listing_marker_delta
```

Expected: PASS.

---
