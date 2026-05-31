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

```powershell
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

```powershell
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

```powershell
cargo test -p listing-domain listing_marker_overlay_query
```

Expected: PASS.

---

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

```powershell
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

```powershell
cargo test -p db --features integration --test listing_marker_tile_integration listing_marker_projection_enqueues_dirty_tiles
cargo test -p api metrics
```

Expected: PASS.

---

## Task 8: Update SSOT Registries And Guardrails

**Files:**

- Modify: `docs/architecture/traffic-auth-policy-registry.v1.json`
- Modify: `docs/architecture/platform-integration/route-exposure-policy.v1.json`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
- Test: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`

- [ ] **Step 1: Add route policies**

Add public derived route policies:

```json
{
  "id": "gongzzang.public_map.listing_marker_delta",
  "owner": "gongzzang",
  "backend_route": "/map/v1/marker-deltas/listing/{z}/{x}/{y}.pbf",
  "methods": ["GET"],
  "auth_policy": { "method": "anonymous_public", "session_required": false },
  "data_exposure_policy": {
    "exposure_class": "public_derived",
    "allowed_data_classes": ["derived_marker_tile"],
    "forbidden_data_classes": ["private_listing", "business_verified_listing_detail", "contact_data"]
  }
}
```

```json
{
  "id": "gongzzang.public_map.listing_marker_tombstone",
  "owner": "gongzzang",
  "backend_route": "/map/v1/marker-tombstones/listing/{z}/{x}/{y}",
  "methods": ["GET"],
  "auth_policy": { "method": "anonymous_public", "session_required": false },
  "data_exposure_policy": {
    "exposure_class": "public_derived",
    "allowed_data_classes": ["marker_id_mask"],
    "forbidden_data_classes": ["private_listing", "business_verified_listing_detail", "contact_data"]
  }
}
```

- [ ] **Step 2: Extend guardrail**

The guardrail must reject:

- `bbox`, `bounds`, `south`, `west`, `north`, `east` in public marker route shapes;
- listing coordinate ownership such as `listing.latitude`, `listing.longitude`, `geom_point`;
- platform-core direct database imports from Gongzzang;
- public listing tile routes that do not require `filter_hash`;
- public tombstone/delta routes returning private data fields.

- [ ] **Step 3: Run guardrail**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
```

Expected: PASS.

---

## Task 9: Update Frontend Map Composition

**Files:**

- Modify: `apps/web/lib/map/marker-tile-style.ts`
- Modify: `apps/web/lib/map/vector-tile-manifest.ts`
- Modify: `apps/web/lib/map/listing-map-runtime.ts`
- Modify: `apps/web/components/listings/listing-map.tsx`
- Test: `apps/web/tests/unit/map/marker-tile-style.test.ts`
- Test: `apps/web/tests/unit/map/vector-tile-manifest.test.ts`

- [ ] **Step 1: Add composition model**

The frontend map state must track:

```ts
type ListingMarkerOverlayState = {
  baseVersion: number | null;
  tombstoneIds: Set<string>;
  deltaSourceId: string;
};
```

- [ ] **Step 2: Apply tombstones before display**

The rendered visible marker set must apply:

```text
visible = base + delta - tombstone
```

The client must never treat tombstone failure as permission to display a stale private/deleted
marker. If tombstones fail for a tile, the client should refresh the base tile or hide the affected
listing layer for that tile until a safe response arrives.

- [ ] **Step 3: Add delta source/layer**

Register a `listing_delta` vector source and layer. Use the same visual style as `listing`, with a
stable source id and layer id generated from the route policy or marker layer registry.

- [ ] **Step 4: Run frontend checks**

Run:

```powershell
pnpm --filter web test
pnpm --filter web exec playwright test
```

Expected: PASS. If no Playwright marker smoke exists, add a minimal smoke before claiming complete.

---

## Task 10: Verification And Release Gate

**Files:**

- Modify: `docs/testing/load.md`
- Modify: `scripts/load/run-k6.ps1`

- [ ] **Step 1: Run backend tests**

Run:

```powershell
cargo test -p listing-domain
cargo test -p db --features integration --test listing_marker_tile_integration
cargo test -p api listing_marker
```

Expected: PASS.

- [ ] **Step 2: Run guardrails**

Run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1
powershell -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-boundary.ps1
powershell -ExecutionPolicy Bypass -File scripts/ci/check-platform-core-dependency-boundary.ps1
```

Expected: PASS.

- [ ] **Step 3: Run load proof**

Run the existing map marker load scenario with:

```text
base tile
base + tombstone
base + delta
cache hit
cache miss
```

Acceptance:

- public stale delete/private marker exposure: 0 known cases;
- successful tile response with silent marker drop: 0;
- tombstone endpoint p95 within public route budget;
- delta endpoint p95 within public route budget;
- DB pool saturation absent under accepted launch RPS.

- [ ] **Step 4: Update docs**

Update:

- `docs/frontend/listings-search.md`
- `docs/runbooks/platform-core-integration-operations.md`
- `docs/testing/load.md`

Mention:

```text
visible markers = base tile + delta overlay - tombstone overlay - unauthorized records
```

---

## Execution Order

1. Task 1 and Task 2 establish schema and typed contracts.
2. Task 3 makes write paths emit structural facts.
3. Task 4 ships tombstone first to prevent stale private/deleted exposure.
4. Task 5 ships delta after stale exposure is controlled.
5. Task 6 makes low zoom truthful.
6. Task 7 adds rebuild/backlog control.
7. Task 8 locks SSOT and guardrails.
8. Task 9 updates frontend composition.
9. Task 10 verifies the full path.

Do not start artifact promote/rollback before tombstone, delta, aggregation, and dirty queue are
working. Static artifacts without tombstones can make stale exposure harder to correct.
