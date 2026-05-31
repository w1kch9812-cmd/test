# Platform Core Anchor Projection Import Plan - Part 03: DB Repository Implementation

Parent index: [Platform Core Anchor Projection Import Implementation Plan](./2026-05-28-platform-core-anchor-projection-import.md).


- [ ] **Step 3: Implement the DB repository module**

Create `crates/db/src/platform_core_anchor.rs` with these public types and functions:

```rust
//! Platform Core anchor read-model import persistence.

use chrono::{DateTime, Utc};
use listing_domain::repository::RepoError;
use serde_json::Value;
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

/// Inbox status for inbound Platform Core events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformCoreEventInboxStatus {
    /// Event was accepted and does not need artifact import.
    Accepted,
    /// Event was accepted and waits for anchor artifact import.
    PendingImport,
}

impl PlatformCoreEventInboxStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::PendingImport => "pending_import",
        }
    }
}

/// Insert payload for a durable Platform Core event inbox row.
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformCoreEventInboxInsert {
    /// Platform Core event id.
    pub event_id: String,
    /// Platform Core event type.
    pub event_type: String,
    /// Event scope, initially `catalog`.
    pub scope: String,
    /// Gongzzang effect.
    pub effect: String,
    /// Initial processing status.
    pub status: PlatformCoreEventInboxStatus,
    /// Full event payload for replay/debug.
    pub payload: Value,
    /// Anchor snapshot id for anchor imports.
    pub anchor_snapshot_id: Option<String>,
    /// Source geometry version for anchor imports.
    pub source_geometry_version: Option<String>,
}

/// Result of an inbox insert attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCoreEventInboxInsertResult {
    /// True when this call inserted the row; false when event id already existed.
    pub inserted: bool,
}

/// One validated Platform Core anchor artifact row.
#[derive(Debug, Clone, PartialEq)]
pub struct AnchorArtifactRow {
    /// Parcel identity.
    pub pnu: String,
    /// EPSG:4326 longitude.
    pub anchor_lng: f64,
    /// EPSG:4326 latitude.
    pub anchor_lat: f64,
    /// Anchor algorithm name.
    pub algorithm: String,
    /// Anchor algorithm version.
    pub algorithm_version: String,
    /// Source geometry checksum.
    pub source_geometry_checksum_sha256: String,
}

/// Batch import into Gongzzang's Platform Core anchor read model.
#[derive(Debug, Clone, PartialEq)]
pub struct PlatformCoreAnchorImport {
    /// Platform Core anchor snapshot id.
    pub anchor_snapshot_id: String,
    /// Platform Core source geometry version.
    pub source_geometry_version: String,
    /// Platform Core publish timestamp.
    pub platform_core_updated_at: DateTime<Utc>,
    /// Validated artifact rows.
    pub rows: Vec<AnchorArtifactRow>,
}

/// Import result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCoreAnchorImportReport {
    /// Rows inserted or updated in `parcel_marker_anchor`.
    pub upserted_anchor_count: u64,
    /// Listing marker projections refreshed from affected PNUs.
    pub refreshed_listing_projection_count: u64,
}

/// Insert an inbound Platform Core event idempotently.
///
/// # Errors
///
/// Returns [`RepoError::Database`] when Postgres rejects the write.
pub async fn insert_inbox_event(
    pool: &PgPool,
    event: &PlatformCoreEventInboxInsert,
) -> Result<PlatformCoreEventInboxInsertResult, RepoError> {
    let result = sqlx::query(
        r#"
        insert into platform_core_event_inbox (
            event_id, event_type, scope, effect, status, payload,
            anchor_snapshot_id, source_geometry_version
        )
        values ($1::uuid, $2, $3, $4, $5, $6, $7, $8)
        on conflict (event_id) do nothing
        "#,
    )
    .bind(&event.event_id)
    .bind(&event.event_type)
    .bind(&event.scope)
    .bind(&event.effect)
    .bind(event.status.as_str())
    .bind(&event.payload)
    .bind(&event.anchor_snapshot_id)
    .bind(&event.source_geometry_version)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(PlatformCoreEventInboxInsertResult {
        inserted: result.rows_affected() == 1,
    })
}

/// Upsert Platform Core anchor rows and refresh affected Gongzzang listing projections.
///
/// # Errors
///
/// Returns [`RepoError::Database`] when validation or Postgres persistence fails.
pub async fn import_anchor_rows(
    pool: &PgPool,
    import: &PlatformCoreAnchorImport,
) -> Result<PlatformCoreAnchorImportReport, RepoError> {
    let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
    sqlx::query(
        r#"
        create temporary table platform_core_anchor_import_stage (
            pnu char(19) primary key,
            anchor_lng double precision not null,
            anchor_lat double precision not null,
            algorithm varchar(64) not null,
            algorithm_version varchar(128) not null,
            source_geometry_checksum_sha256 char(64) not null
        ) on commit drop
        "#,
    )
    .execute(&mut *tx)
    .await
    .map_err(map_sqlx_err)?;

    for row in &import.rows {
        sqlx::query(
            r#"
            insert into platform_core_anchor_import_stage (
                pnu, anchor_lng, anchor_lat, algorithm, algorithm_version,
                source_geometry_checksum_sha256
            )
            values ($1, $2, $3, $4, $5, $6)
            on conflict (pnu) do update set
                anchor_lng = excluded.anchor_lng,
                anchor_lat = excluded.anchor_lat,
                algorithm = excluded.algorithm,
                algorithm_version = excluded.algorithm_version,
                source_geometry_checksum_sha256 = excluded.source_geometry_checksum_sha256
            "#,
        )
        .bind(&row.pnu)
        .bind(row.anchor_lng)
        .bind(row.anchor_lat)
        .bind(&row.algorithm)
        .bind(&row.algorithm_version)
        .bind(&row.source_geometry_checksum_sha256)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
    }

    let anchor_result = sqlx::query(
        r#"
        insert into parcel_marker_anchor (
            pnu,
            anchor_point,
            algorithm,
            algorithm_version,
            anchor_snapshot_id,
            source_geometry_version,
            source_geometry_checksum_sha256,
            platform_core_updated_at,
            synced_at
        )
        select
            pnu,
            ST_SetSRID(ST_MakePoint(anchor_lng, anchor_lat), 4326),
            algorithm,
            algorithm_version,
            $1,
            $2,
            source_geometry_checksum_sha256,
            $3,
            now()
        from platform_core_anchor_import_stage
        on conflict (pnu) do update set
            anchor_point = excluded.anchor_point,
            algorithm = excluded.algorithm,
            algorithm_version = excluded.algorithm_version,
            anchor_snapshot_id = excluded.anchor_snapshot_id,
            source_geometry_version = excluded.source_geometry_version,
            source_geometry_checksum_sha256 = excluded.source_geometry_checksum_sha256,
            platform_core_updated_at = excluded.platform_core_updated_at,
            synced_at = now()
        "#,
    )
    .bind(&import.anchor_snapshot_id)
    .bind(&import.source_geometry_version)
    .bind(import.platform_core_updated_at)
    .execute(&mut *tx)
    .await
    .map_err(map_sqlx_err)?;

    let projection_row = sqlx::query(
        r#"
        with affected as (
            select pnu from platform_core_anchor_import_stage
        ),
        candidate as (
            select l.id
            from listing l
            join affected a on a.pnu = l.parcel_pnu
            where l.status = 'active'
        )
        select count(*)::int8 as count
        from candidate
        "#,
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(map_sqlx_err)?;
    let refreshed_listing_projection_count: i64 =
        projection_row.try_get("count").map_err(map_sqlx_err)?;

    sqlx::query(
        r#"
        insert into listing_marker_projection (
            marker_id,
            listing_id,
            pnu,
            anchor_point,
            anchor_snapshot_id,
            source_geometry_version,
            source_geometry_checksum_sha256,
            source_listing_version,
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
            listing_updated_at,
            updated_at
        )
        select
            'lm_' || l.id,
            l.id,
            l.parcel_pnu,
            a.anchor_point,
            a.anchor_snapshot_id,
            a.source_geometry_version,
            a.source_geometry_checksum_sha256,
            l.version,
            1,
            least(16383, greatest(0, floor(((ST_X(a.anchor_point) + 180.0) / 360.0) * 16384.0)::integer)),
            least(
                16383,
                greatest(
                    0,
                    floor((((1.0 - (ln(tan(radians(ST_Y(a.anchor_point))) + (1.0 / cos(radians(ST_Y(a.anchor_point))))) / pi())) / 2.0) * 16384.0)::integer)
                )
            ),
            l.status,
            case when l.status = 'active' then 'public' else 'owner_private' end,
            l.listing_type,
            l.transaction_type,
            l.price_krw,
            l.area_m2,
            0,
            l.updated_at,
            now()
        from listing l
        join platform_core_anchor_import_stage s on s.pnu = l.parcel_pnu
        join parcel_marker_anchor a on a.pnu = l.parcel_pnu
        where l.status = 'active'
        on conflict (listing_id) do update set
            marker_id = excluded.marker_id,
            pnu = excluded.pnu,
            anchor_point = excluded.anchor_point,
            anchor_snapshot_id = excluded.anchor_snapshot_id,
            source_geometry_version = excluded.source_geometry_version,
            source_geometry_checksum_sha256 = excluded.source_geometry_checksum_sha256,
            source_listing_version = excluded.source_listing_version,
            projection_version = listing_marker_projection.projection_version + 1,
            z14_tile_x = excluded.z14_tile_x,
            z14_tile_y = excluded.z14_tile_y,
            listing_status = excluded.listing_status,
            visibility_scope = excluded.visibility_scope,
            listing_type = excluded.listing_type,
            transaction_type = excluded.transaction_type,
            price_krw = excluded.price_krw,
            area_m2 = excluded.area_m2,
            rank_score = excluded.rank_score,
            listing_updated_at = excluded.listing_updated_at,
            updated_at = now()
        "#,
    )
    .execute(&mut *tx)
    .await
    .map_err(map_sqlx_err)?;

    tx.commit().await.map_err(map_sqlx_err)?;
    Ok(PlatformCoreAnchorImportReport {
        upserted_anchor_count: anchor_result.rows_affected(),
        refreshed_listing_projection_count: u64::try_from(refreshed_listing_projection_count)
            .map_err(|err| RepoError::Database(err.to_string()))?,
    })
}
```

- [ ] **Step 4: Expose the DB module**

In `crates/db/src/lib.rs`, add:

```rust
pub mod platform_core_anchor;
```

- [ ] **Step 5: Run the DB test and verify GREEN**

Run:

```powershell
cargo test -p db --features integration --test platform_core_anchor_import_integration
```

Expected: both integration tests pass.
