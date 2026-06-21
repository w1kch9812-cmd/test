# Gongzzang Marker Data Plane SSS Plan - Part 01: Overlay Schema And Domain Contracts

Parent index: [Gongzzang Marker Data Plane SSS Implementation Plan](./2026-05-31-gongzzang-marker-data-plane-sss.md).

## Task 1: Add Overlay And Dirty Queue Schema

**Files:**

- Create: `migrations/30017_listing_marker_overlay_and_dirty_queue.sql`
- Test: `crates/db/tests/listing_marker_tile_integration.rs`

- [ ] **Step 1: Add failing schema smoke assertions**

Add this test near the other listing marker projection tests:

```rust
#[tokio::test]
async fn listing_marker_overlay_tables_exist_with_expected_columns() {
    let _guard = lock_marker_tile_tests().await;
    let pool = setup_test_pool().await;

    let rows = sqlx::query(
        r"
        select table_name, column_name
        from information_schema.columns
        where table_name in (
            'listing_marker_tombstone_log',
            'listing_marker_delta_log',
            'listing_marker_dirty_tile_queue'
        )
        order by table_name, ordinal_position
        ",
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    let columns = rows
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("table_name"),
                row.get::<String, _>("column_name"),
            )
        })
        .collect::<Vec<_>>();

    assert!(columns.contains(&(
        "listing_marker_tombstone_log".to_owned(),
        "marker_id".to_owned()
    )));
    assert!(columns.contains(&(
        "listing_marker_delta_log".to_owned(),
        "marker_id".to_owned()
    )));
    assert!(columns.contains(&(
        "listing_marker_dirty_tile_queue".to_owned(),
        "tile_z".to_owned()
    )));
}
```

- [ ] **Step 2: Run the failing test**

Run:

```bash
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_overlay_tables_exist_with_expected_columns
```

Expected: FAIL because the three tables do not exist.

- [ ] **Step 3: Add the migration**

Create `migrations/30017_listing_marker_overlay_and_dirty_queue.sql`:

```sql
-- Listing marker overlay and dirty-tile serving state.
--
-- These tables are Gongzzang-owned serving projections. They do not own canonical
-- marker coordinates; all positions are still derived from platform-core PNU anchors
-- through listing_marker_projection.

create table listing_marker_tombstone_log (
    tombstone_id bigserial primary key,
    marker_id varchar(64) not null,
    listing_id char(30) not null references listing(id) on delete cascade,
    pnu char(19) not null,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    projection_version bigint not null,
    anchor_snapshot_id varchar(128) not null,
    reason varchar(64) not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null default now() + interval '15 minutes',
    constraint listing_marker_tombstone_marker_id_chk
        check (marker_id ~ '^lm_lst_[0-9A-Z]{26}$'),
    constraint listing_marker_tombstone_pnu_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_tombstone_z14_x_chk
        check (z14_tile_x >= 0 and z14_tile_x < 16384),
    constraint listing_marker_tombstone_z14_y_chk
        check (z14_tile_y >= 0 and z14_tile_y < 16384),
    constraint listing_marker_tombstone_projection_version_chk
        check (projection_version >= 1),
    constraint listing_marker_tombstone_reason_chk
        check (reason in ('deleted', 'withdrawn', 'sold', 'expired', 'private', 'rejected'))
);

create unique index listing_marker_tombstone_once_idx
    on listing_marker_tombstone_log(marker_id, projection_version, reason);

create index listing_marker_tombstone_active_tile_idx
    on listing_marker_tombstone_log(z14_tile_x, z14_tile_y, expires_at);

create table listing_marker_delta_log (
    delta_id bigserial primary key,
    marker_id varchar(64) not null,
    listing_id char(30) not null references listing(id) on delete cascade,
    pnu char(19) not null,
    z14_tile_x integer not null,
    z14_tile_y integer not null,
    projection_version bigint not null,
    anchor_snapshot_id varchar(128) not null,
    change_kind varchar(64) not null,
    created_at timestamptz not null default now(),
    expires_at timestamptz not null default now() + interval '5 minutes',
    constraint listing_marker_delta_marker_id_chk
        check (marker_id ~ '^lm_lst_[0-9A-Z]{26}$'),
    constraint listing_marker_delta_pnu_chk
        check (pnu ~ '^[0-9]{19}$'),
    constraint listing_marker_delta_z14_x_chk
        check (z14_tile_x >= 0 and z14_tile_x < 16384),
    constraint listing_marker_delta_z14_y_chk
        check (z14_tile_y >= 0 and z14_tile_y < 16384),
    constraint listing_marker_delta_projection_version_chk
        check (projection_version >= 1),
    constraint listing_marker_delta_change_kind_chk
        check (change_kind in ('created_public', 'updated_public', 'became_public'))
);

create unique index listing_marker_delta_once_idx
    on listing_marker_delta_log(marker_id, projection_version, change_kind);

create index listing_marker_delta_active_tile_idx
    on listing_marker_delta_log(z14_tile_x, z14_tile_y, expires_at);

create table listing_marker_dirty_tile_queue (
    dirty_tile_id bigserial primary key,
    layer varchar(64) not null default 'listing',
    tile_z integer not null,
    tile_x integer not null,
    tile_y integer not null,
    reason varchar(64) not null,
    status varchar(32) not null default 'pending',
    priority integer not null default 100,
    attempts integer not null default 0,
    first_seen_at timestamptz not null default now(),
    next_attempt_at timestamptz not null default now(),
    last_error text,
    constraint listing_marker_dirty_layer_chk
        check (layer = 'listing'),
    constraint listing_marker_dirty_tile_z_chk
        check (tile_z >= 0 and tile_z <= 22),
    constraint listing_marker_dirty_tile_x_chk
        check (tile_x >= 0),
    constraint listing_marker_dirty_tile_y_chk
        check (tile_y >= 0),
    constraint listing_marker_dirty_reason_chk
        check (reason in ('delta', 'tombstone', 'projection_update', 'anchor_snapshot')),
    constraint listing_marker_dirty_status_chk
        check (status in ('pending', 'processing', 'done', 'failed')),
    constraint listing_marker_dirty_attempts_chk
        check (attempts >= 0)
);

create unique index listing_marker_dirty_tile_pending_once_idx
    on listing_marker_dirty_tile_queue(layer, tile_z, tile_x, tile_y, reason)
    where status in ('pending', 'processing');

create index listing_marker_dirty_tile_due_idx
    on listing_marker_dirty_tile_queue(priority asc, next_attempt_at asc, first_seen_at asc)
    where status = 'pending';
```

- [ ] **Step 4: Run migration and schema test**

Run:

```bash
sqlx migrate run
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_overlay_tables_exist_with_expected_columns
```

Expected: PASS.

---

## Task 2: Extend Listing Domain Contracts

**Files:**

- Modify: `crates/domain/core/listing/src/repository.rs`

- [ ] **Step 1: Add domain unit tests first**

Append focused tests near marker query tests:

```rust
#[test]
fn listing_marker_overlay_query_rejects_out_of_range_tiles() {
    assert!(ListingMarkerOverlayTileQuery::try_new(23, 0, 0, None).is_err());
    assert!(ListingMarkerOverlayTileQuery::try_new(4, 16, 0, None).is_err());
    assert!(ListingMarkerOverlayTileQuery::try_new(4, 0, 16, None).is_err());
}
```

- [ ] **Step 2: Add value objects and repository methods**

Add these domain types after `ListingMarkerMask`:

```rust
/// Query for listing marker overlay records addressed by tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerOverlayTileQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Optional base projection version already loaded by the browser.
    pub base_version: Option<i64>,
}

impl ListingMarkerOverlayTileQuery {
    /// Validate public overlay tile-coordinate input.
    ///
    /// # Errors
    /// Returns [`ListingMarkerTileQueryError`] for invalid tile coordinates.
    pub fn try_new(
        z: u8,
        x: u32,
        y: u32,
        base_version: Option<i64>,
    ) -> Result<Self, ListingMarkerTileQueryError> {
        if z > LISTING_MARKER_TILE_MAX_ZOOM {
            return Err(ListingMarkerTileQueryError::InvalidZoom { z });
        }
        let axis_limit = 1_u32 << u32::from(z);
        if x >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidX { z, x });
        }
        if y >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidY { z, y });
        }
        Ok(Self {
            z,
            x,
            y,
            base_version,
        })
    }
}

/// Listing marker tombstone overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTombstones {
    /// Marker ids that must be hidden by the client.
    pub marker_ids: Vec<String>,
    /// Highest projection version represented by this tombstone response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this response.
    pub anchor_snapshot_id: Option<String>,
}

/// Listing marker delta overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerDeltas {
    /// MVT/PBF response bytes for recently changed public markers.
    pub bytes: Vec<u8>,
    /// MVT source-layer name.
    pub layer_name: &'static str,
    /// Number of changed marker features represented.
    pub feature_count: i64,
    /// Highest projection version represented by this delta response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this response.
    pub anchor_snapshot_id: Option<String>,
}
```

Add these methods to `ListingRepository`:

```rust
    /// Return marker ids that must be hidden for a loaded listing marker tile.
    async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError>;

    /// Return recent public marker changes for a loaded listing marker tile.
    async fn find_listing_marker_deltas(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerDeltas, RepoError>;
```

- [ ] **Step 3: Run domain tests**

Run:

```bash
cargo test -p listing-domain listing_marker_overlay_query
```

Expected: PASS.

---
