# Gongzzang Marker Data Plane SSS Plan - Part 03: Aggregation And Dirty Queue

Parent index: [Gongzzang Marker Data Plane SSS Implementation Plan](./2026-05-31-gongzzang-marker-data-plane-sss.md).


## Task 6: Implement Truthful Low-Zoom Aggregation

**Files:**

- Modify: `crates/domain/core/listing/src/repository.rs`
- Modify: `crates/db/src/listing/marker_tile.rs`
- Test: `crates/db/tests/listing_marker_tile_integration.rs`

- [ ] **Step 1: Add tests for low zoom**

Add tests:

```rust
#[tokio::test]
async fn listing_marker_tile_aggregates_low_zoom_without_dropping_records() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-aggregate", "marker-aggregate@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100190000";
    seed_anchor(&pool, pnu).await;

    let mut first = make_listing(owner.clone(), pnu, "Aggregate listing one");
    let mut second = make_listing(owner.clone(), pnu, "Aggregate listing two");
    activate_listing(&repo, &mut first, &owner).await;
    activate_listing(&repo, &mut second, &owner).await;

    let tile = repo
        .find_listing_marker_tile(ListingMarkerTileQuery::new(11, 0, 0, ListingMarkerFilter::AllActive))
        .await
        .unwrap();

    assert_eq!(tile.eligible_count, 2);
    assert_eq!(tile.represented_count, 2);
    assert_eq!(tile.aggregate_count, 1);
}
```

- [ ] **Step 2: Accept public low zooms**

Change `LISTING_MARKER_TILE_MIN_ZOOM` from `14` to `0`, and add:

```rust
/// Lowest zoom where exact listing marker features are preferred.
pub const LISTING_MARKER_TILE_EXACT_MIN_ZOOM: u8 = 14;
```

- [ ] **Step 3: Split SQL path**

In `marker_tile.rs`, choose exact or aggregate query by zoom:

```rust
if query.z < LISTING_MARKER_TILE_EXACT_MIN_ZOOM {
    return find_aggregate_listing_marker_tile(pool, query).await;
}
find_exact_listing_marker_tile(pool, query).await
```

Aggregate query must:

- filter from `listing_marker_projection`, not `listing`;
- apply the same normalized filter semantics;
- use PNU anchors only;
- return `represented_count == eligible_count`;
- emit one truthful aggregate feature with `count = eligible_count` for the requested low-zoom tile
  as the first implementation;
- set `aggregate_count = 1` when count > 0.

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_tile_aggregates_low_zoom_without_dropping_records
cargo test -p api listing_marker_tile
```

Expected: PASS.

---

## Task 7: Add Dirty Tile Queue Writes And Metrics

**Files:**

- Modify: `crates/db/src/listing/marker_projection.rs`
- Modify: `services/api/src/state.rs`
- Modify: `services/api/src/routes/mod.rs`
- Test: `crates/db/tests/listing_marker_tile_integration.rs`

- [ ] **Step 1: Add dirty queue test**

Add a test that a public listing update inserts pending dirty tile rows:

```rust
#[tokio::test]
async fn listing_marker_projection_enqueues_dirty_tiles_for_public_change() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-marker-dirty", "marker-dirty@example.com").await;
    let repo = PgListingRepository::new(pool.clone());
    let pnu = "1111010100100200000";
    seed_anchor(&pool, pnu).await;

    let mut listing = make_listing(owner.clone(), pnu, "Dirty tile listing");
    activate_listing(&repo, &mut listing, &owner).await;

    let count: i64 = sqlx::query_scalar(
        "select count(*)::int8 from listing_marker_dirty_tile_queue where status = 'pending'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(count >= 1);
}
```

- [ ] **Step 2: Insert dirty rows from projection sync**

For each public delta or tombstone, enqueue at least z14 and low-zoom parent tiles. Initial parent
set:

```text
z = 0, 6, 10, 11, 12, 13, 14
```

Use `ON CONFLICT DO NOTHING` against the pending unique index.

- [ ] **Step 3: Add metrics**

Expose:

```text
gongzzang_listing_marker_dirty_tiles_pending
gongzzang_listing_marker_dirty_tile_oldest_age_seconds
gongzzang_listing_marker_tombstones_active
gongzzang_listing_marker_deltas_active
```

- [ ] **Step 4: Run tests**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_enqueues_dirty_tiles
cargo test -p api metrics
```

Expected: PASS.

---
