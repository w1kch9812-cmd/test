# Platform Core Anchor Projection Import Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import Platform Core PNU anchor snapshot artifacts into Gongzzang's local read model with durable event idempotency and listing marker projection refresh.

**Architecture:** Platform Core still owns parcel geometry and anchor coordinates. Gongzzang stores only a read-model copy in `parcel_marker_anchor`, records inbound Platform Core events in a durable inbox, and refreshes Gongzzang-owned `listing_marker_projection` rows from listing semantics joined to the copied anchors. The public Next.js receiver validates Platform Core headers, then forwards accepted events to the Rust API internal route for durable storage; the artifact importer runs from the Rust API package against the same database.

**Tech Stack:** Next.js route handler, Rust Axum, SQLx/PostGIS, existing `reqwest`, existing workspace `sha2`, Vitest, Rust integration tests, PowerShell guardrails.

---

## Approval Gate

Do not create the migration file in this plan until the user explicitly approves DB schema changes.

The 2026-05-28 DB approval handoff only approved
`migrations/30015_drop_platform_core_legacy_schema.sql`. It does not approve the
durable inbox/read-model import migration in this plan. Because `30015` is
already used, this plan reserves the next forward migration number,
`30016_platform_core_event_inbox_anchor_import.sql`, after explicit approval.
`scripts/ci/check-migration-version-prefixes.ps1` now guards the actual
`migrations/` directory against duplicate numeric prefixes before this plan can
land a new migration file.

The required DB schema changes are:

- widen `parcel_marker_anchor.algorithm_version` from `varchar(32)` to `varchar(128)`;
- create `platform_core_event_inbox` for durable event idempotency, traceability, retry state, and failure state.

No new external package version is required. If implementation uses `sha2` from the workspace in `services/api`, add it as a package dependency only because the workspace already pins the version.

## File Structure

- Create after DB approval: `migrations/30016_platform_core_event_inbox_anchor_import.sql`
  - Widens the existing anchor projection column.
  - Creates the event inbox table and indexes.
- Create: `crates/db/src/platform_core_anchor.rs`
  - Owns SQLx persistence for the inbox, anchor artifact row upsert, and affected listing projection refresh.
- Modify: `crates/db/src/lib.rs`
  - Exposes the new `platform_core_anchor` module.
- Test: `crates/db/tests/platform_core_anchor_import_integration.rs`
  - Proves inbox idempotency, long algorithm versions, anchor row upsert, and listing projection refresh.
- Create: `services/api/src/routes/platform_core_events.rs`
  - Owns `/internal/platform-core/events` and shared-secret validation.
- Modify: `services/api/src/main.rs`
  - Wires the internal Platform Core event route with DB state.
- Create: `services/api/src/bin/platform_core_anchor_import.rs`
  - Processes pending anchor snapshot events by fetching the immutable manifest and JSONL objects.
- Create: `services/api/src/platform_core_anchor_import.rs`
  - Parses manifests/entries, validates checksum and row counts, and calls `db::platform_core_anchor`.
- Modify: `services/api/Cargo.toml`
  - Adds existing workspace `sha2` if checksum code lives in `services/api`.
- Modify: `apps/web/app/platform-core/events/route.ts`
  - Keeps public validation and cache invalidation, forwards supported events to Rust internal API.
- Test: `apps/web/tests/unit/platform-core-events.test.ts`
  - Proves forwarding success, upstream failure retry behavior, and duplicate ack pass-through.
- Modify: `scripts/ci/check-platform-core-boundary.ps1`
  - Requires the durable inbox migration and Rust internal route.
- Modify: `scripts/ci/check-platform-core-boundary.tests.ps1`
  - Adds fixtures for the required inbox/importer paths.
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
  - Requires `algorithm_version varchar(128)` and the anchor import integration test.
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`
  - Updates fixture expectations.

## Task 1: DB Migration Contract

**Files:**
- Create after approval: `migrations/30016_platform_core_event_inbox_anchor_import.sql`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`
- Modify: `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`

- [ ] **Step 1: Write the failing guardrail test**

In `scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1`, update the clean migration fixture for `migrations\30012_parcel_marker_anchor_projection.sql` so it contains:

```sql
algorithm_version varchar(128) not null
```

Add a required file fixture:

```powershell
Write-File -Root $Root -RelativePath "migrations\30016_platform_core_event_inbox_anchor_import.sql" -Content @'
alter table parcel_marker_anchor
    alter column algorithm_version type varchar(128);

create table platform_core_event_inbox (
    event_id uuid primary key,
    event_type varchar(128) not null,
    scope varchar(32) not null,
    effect varchar(64) not null,
    status varchar(32) not null,
    payload jsonb not null,
    anchor_snapshot_id varchar(128),
    source_geometry_version varchar(128),
    received_at timestamptz not null default now(),
    processed_at timestamptz,
    failed_at timestamptz,
    failure_reason text,
    constraint platform_core_event_inbox_scope_chk
        check (scope = 'catalog'),
    constraint platform_core_event_inbox_status_chk
        check (status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')),
    constraint platform_core_event_inbox_effect_chk
        check (effect in ('invalidate_catalog_cache', 'enqueue_anchor_projection_import'))
);

create index platform_core_event_inbox_pending_idx
    on platform_core_event_inbox(event_type, received_at)
    where status = 'pending_import';
create index platform_core_event_inbox_anchor_snapshot_idx
    on platform_core_event_inbox(anchor_snapshot_id)
    where anchor_snapshot_id is not null;
'@
```

- [ ] **Step 2: Run the guardrail test and verify RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests.ps1
```

Expected: fail because `check-pnu-anchor-pbf-marker-contract.ps1` does not yet require migration `30015` or `algorithm_version varchar(128)`.

- [ ] **Step 3: Implement the guardrail requirement**

In `scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1`, update the `migrations/30012_parcel_marker_anchor_projection.sql` tokens from:

```powershell
"anchor_snapshot_id",
```

to include:

```powershell
"algorithm_version varchar(128) not null",
"anchor_snapshot_id",
```

Add a new contract entry:

```powershell
[pscustomobject]@{
    RelativePath = "migrations/30016_platform_core_event_inbox_anchor_import.sql"
    Tokens = @(
        "alter table parcel_marker_anchor",
        "alter column algorithm_version type varchar(128)",
        "create table platform_core_event_inbox",
        "event_id uuid primary key",
        "payload jsonb not null",
        "status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')",
        "platform_core_event_inbox_pending_idx"
    )
}
```

- [ ] **Step 4: Create the approved migration**

After user DB approval, create `migrations/30016_platform_core_event_inbox_anchor_import.sql` with exactly this SQL:

```sql
-- Durable inbound Platform Core event inbox and anchor importer compatibility.
--
-- `parcel_marker_anchor` remains a Gongzzang-local read model copied from
-- Platform Core. The inbox records Platform Core webhook events by event id so
-- replays are idempotent and import failures are inspectable.

alter table parcel_marker_anchor
    alter column algorithm_version type varchar(128);

create table platform_core_event_inbox (
    event_id uuid primary key,
    event_type varchar(128) not null,
    scope varchar(32) not null,
    effect varchar(64) not null,
    status varchar(32) not null,
    payload jsonb not null,
    anchor_snapshot_id varchar(128),
    source_geometry_version varchar(128),
    received_at timestamptz not null default now(),
    processed_at timestamptz,
    failed_at timestamptz,
    failure_reason text,
    constraint platform_core_event_inbox_scope_chk
        check (scope = 'catalog'),
    constraint platform_core_event_inbox_status_chk
        check (status in ('accepted', 'pending_import', 'processing', 'processed', 'failed')),
    constraint platform_core_event_inbox_effect_chk
        check (effect in ('invalidate_catalog_cache', 'enqueue_anchor_projection_import')),
    constraint platform_core_event_inbox_anchor_payload_chk
        check (
            event_type <> 'catalog.parcel_marker_anchor.snapshot.published.v1'
            or (
                anchor_snapshot_id is not null
                and source_geometry_version is not null
                and effect = 'enqueue_anchor_projection_import'
            )
        )
);

create index platform_core_event_inbox_pending_idx
    on platform_core_event_inbox(event_type, received_at)
    where status = 'pending_import';

create index platform_core_event_inbox_anchor_snapshot_idx
    on platform_core_event_inbox(anchor_snapshot_id)
    where anchor_snapshot_id is not null;
```

- [ ] **Step 5: Run the guardrail test and verify GREEN**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests.ps1
```

Expected: `check-pnu-anchor-pbf-marker-contract-tests-ok`.

## Task 2: DB Anchor Import Repository

**Files:**
- Create: `crates/db/src/platform_core_anchor.rs`
- Modify: `crates/db/src/lib.rs`
- Test: `crates/db/tests/platform_core_anchor_import_integration.rs`

- [ ] **Step 1: Write the failing DB integration test**

Create `crates/db/tests/platform_core_anchor_import_integration.rs`:

```rust
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::{DateTime, Utc};
use db::platform_core_anchor::{
    AnchorArtifactRow, PlatformCoreAnchorImport, PlatformCoreEventInboxInsert,
    PlatformCoreEventInboxStatus,
};
use sqlx::Row;

use common::{setup_test_pool, truncate_all};

fn published_at() -> DateTime<Utc> {
    "2026-05-28T12:00:00Z".parse().unwrap()
}

#[tokio::test]
async fn inbox_insert_is_idempotent_by_platform_core_event_id() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let event = PlatformCoreEventInboxInsert {
        event_id: "0196f0b0-3e01-7000-8000-000000000002".to_owned(),
        event_type: "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned(),
        scope: "catalog".to_owned(),
        effect: "enqueue_anchor_projection_import".to_owned(),
        status: PlatformCoreEventInboxStatus::PendingImport,
        payload: serde_json::json!({"schema_version": 1}),
        anchor_snapshot_id: Some("anchor-snapshot-20260528T120000Z".to_owned()),
        source_geometry_version: Some("silver.parcel_boundaries@20260528".to_owned()),
    };

    let first = db::platform_core_anchor::insert_inbox_event(&pool, &event)
        .await
        .unwrap();
    let second = db::platform_core_anchor::insert_inbox_event(&pool, &event)
        .await
        .unwrap();

    assert!(first.inserted);
    assert!(!second.inserted);

    let count: i64 = sqlx::query_scalar("select count(*) from platform_core_event_inbox")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn anchor_import_upserts_long_algorithm_version_and_refreshes_listing_projection() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let pnu = "1111010100100090000";
    sqlx::query(
        r#"
        insert into "user" (id, zsub, email, display_name, kind, created_at, updated_at, version)
        values ('usr_01HXY3NK0Z9F6S1B2C3D4E5F6G', 'zsub-anchor-import', 'anchor-import@example.com',
                'Anchor Import Owner', 'individual', now(), now(), 1)
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        insert into listing (
            id, owner_id, parcel_pnu, status, listing_type, transaction_type, price_krw, area_m2,
            title, description, created_at, updated_at, version
        )
        values (
            'lst_01HXY3NK0Z9F6S1B2C3D4E5F6G',
            'usr_01HXY3NK0Z9F6S1B2C3D4E5F6G',
            $1,
            'active',
            'factory',
            'sale',
            500000000,
            330.58,
            'Anchor imported listing',
            'listing marker projection refresh test',
            now(),
            now(),
            1
        )
        "#,
    )
    .bind(pnu)
    .execute(&pool)
    .await
    .unwrap();

    let import = PlatformCoreAnchorImport {
        anchor_snapshot_id: "anchor-snapshot-20260528T120000Z".to_owned(),
        source_geometry_version: "silver.parcel_boundaries@20260528".to_owned(),
        platform_core_updated_at: published_at(),
        rows: vec![AnchorArtifactRow {
            pnu: pnu.to_owned(),
            anchor_lng: 126.9780,
            anchor_lat: 37.5665,
            algorithm: "polylabel".to_owned(),
            algorithm_version: "postgis-st_maximuminscribedcircle-v1".to_owned(),
            source_geometry_checksum_sha256: "b".repeat(64),
        }],
    };

    let report = db::platform_core_anchor::import_anchor_rows(&pool, &import)
        .await
        .unwrap();

    assert_eq!(report.upserted_anchor_count, 1);
    assert_eq!(report.refreshed_listing_projection_count, 1);

    let row = sqlx::query(
        r#"
        select
            a.algorithm_version,
            p.anchor_snapshot_id,
            p.source_geometry_version,
            p.source_geometry_checksum_sha256
        from parcel_marker_anchor a
        join listing_marker_projection p on p.pnu = a.pnu
        where a.pnu = $1
        "#,
    )
    .bind(pnu)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        row.get::<String, _>("algorithm_version"),
        "postgis-st_maximuminscribedcircle-v1"
    );
    assert_eq!(
        row.get::<String, _>("anchor_snapshot_id"),
        "anchor-snapshot-20260528T120000Z"
    );
    assert_eq!(
        row.get::<String, _>("source_geometry_version"),
        "silver.parcel_boundaries@20260528"
    );
    assert_eq!(row.get::<String, _>("source_geometry_checksum_sha256"), "b".repeat(64));
}
```

- [ ] **Step 2: Run the DB test and verify RED**

Run with a migrated test database:

```powershell
cargo test -p db --features integration --test platform_core_anchor_import_integration
```

Expected: compile failure because `db::platform_core_anchor` does not exist.

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

## Task 3: Rust Internal Event Route

**Files:**
- Create: `services/api/src/routes/platform_core_events.rs`
- Modify: `services/api/src/main.rs`

- [ ] **Step 1: Write the route unit test**

Create route tests in `services/api/src/routes/platform_core_events.rs` under `#[cfg(test)]` that call a pure function:

```rust
#[test]
fn anchor_snapshot_event_maps_to_pending_import_inbox_row() {
    let event = PlatformCoreEventEnvelope {
        event_id: "0196f0b0-3e01-7000-8000-000000000002".to_owned(),
        event_type: PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE.to_owned(),
        occurred_at: "2026-05-28T12:00:00Z".to_owned(),
        scope: "catalog".to_owned(),
        payload: serde_json::json!({
            "type": PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE,
            "schema_version": 1,
            "anchor_snapshot_id": "anchor-snapshot-20260528T120000Z",
            "source_geometry_version": "silver.parcel_boundaries@20260528",
            "artifact_manifest_url": "https://platform-core.example.com/artifacts/anchor-snapshot.json",
            "artifact_checksum_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "row_count": 1,
            "published_at": "2026-05-28T12:00:00Z"
        }),
    };

    let row = inbox_insert_from_event(&event).expect("inbox row");
    assert_eq!(row.event_id, event.event_id);
    assert_eq!(row.effect, "enqueue_anchor_projection_import");
    assert_eq!(row.anchor_snapshot_id.as_deref(), Some("anchor-snapshot-20260528T120000Z"));
}
```

- [ ] **Step 2: Run the route test and verify RED**

Run:

```powershell
cargo test -p api platform_core_events
```

Expected: compile failure because `platform_core_events` route does not exist.

- [ ] **Step 3: Implement the route module**

Create `services/api/src/routes/platform_core_events.rs` with:

```rust
//! Internal receiver for Platform Core events forwarded by the Next.js public route.

use axum::{extract::State, http::HeaderMap, response::IntoResponse, Json};
use db::platform_core_anchor::{
    self, PlatformCoreEventInboxInsert, PlatformCoreEventInboxStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

const INTERNAL_AUTH_HEADER: &str = "x-internal-auth";
const GOLD_POINTER_EVENT_TYPE: &str = "catalog.industrial_complex.gold_pointer.published.v1";
const PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE: &str =
    "catalog.parcel_marker_anchor.snapshot.published.v1";

/// Route state.
#[derive(Clone)]
pub struct PlatformCoreEventsState {
    /// Shared Postgres pool.
    pub pool: PgPool,
    /// Shared internal auth secret.
    pub internal_auth_secret: String,
}

#[derive(Debug, Deserialize)]
struct PlatformCoreEventEnvelope {
    event_id: String,
    event_type: String,
    occurred_at: String,
    scope: String,
    payload: Value,
}

#[derive(Debug, Serialize)]
struct PlatformCoreEventAck {
    event_id: String,
    effect: &'static str,
    status: &'static str,
}

/// Persist an inbound Platform Core event.
pub async fn post_platform_core_event(
    State(state): State<PlatformCoreEventsState>,
    headers: HeaderMap,
    Json(event): Json<PlatformCoreEventEnvelope>,
) -> impl IntoResponse {
    if !internal_auth_ok(&headers, &state.internal_auth_secret) {
        return (
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"status": "rejected", "reason": "unauthorized"})),
        );
    }

    let Ok(row) = inbox_insert_from_event(&event) else {
        return (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "rejected", "reason": "invalid_event"})),
        );
    };
    let effect = row.effect.clone();
    match platform_core_anchor::insert_inbox_event(&state.pool, &row).await {
        Ok(_) => (
            axum::http::StatusCode::ACCEPTED,
            Json(serde_json::json!({
                "event_id": event.event_id,
                "effect": effect,
                "status": "accepted"
            })),
        ),
        Err(error) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "rejected",
                "reason": "inbox_write_failed",
                "detail": error.to_string()
            })),
        ),
    }
}
```

Add helper functions in the same file:

```rust
fn internal_auth_ok(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(INTERNAL_AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|actual| actual == expected)
}

fn inbox_insert_from_event(
    event: &PlatformCoreEventEnvelope,
) -> Result<PlatformCoreEventInboxInsert, &'static str> {
    if event.scope != "catalog" {
        return Err("scope");
    }

    match event.event_type.as_str() {
        GOLD_POINTER_EVENT_TYPE => Ok(PlatformCoreEventInboxInsert {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            scope: event.scope.clone(),
            effect: "invalidate_catalog_cache".to_owned(),
            status: PlatformCoreEventInboxStatus::Accepted,
            payload: event.payload.clone(),
            anchor_snapshot_id: None,
            source_geometry_version: None,
        }),
        PARCEL_ANCHOR_SNAPSHOT_EVENT_TYPE => Ok(PlatformCoreEventInboxInsert {
            event_id: event.event_id.clone(),
            event_type: event.event_type.clone(),
            scope: event.scope.clone(),
            effect: "enqueue_anchor_projection_import".to_owned(),
            status: PlatformCoreEventInboxStatus::PendingImport,
            payload: event.payload.clone(),
            anchor_snapshot_id: event
                .payload
                .get("anchor_snapshot_id")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            source_geometry_version: event
                .payload
                .get("source_geometry_version")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
        }),
        _ => Err("event_type"),
    }
}
```

- [ ] **Step 4: Wire the route in `main.rs`**

Add module export:

```rust
pub mod platform_core_events;
```

Add router:

```rust
let platform_core_events_router: Router<()> = Router::new()
    .route(
        "/internal/platform-core/events",
        axum::routing::post(routes::platform_core_events::post_platform_core_event),
    )
    .with_state(routes::platform_core_events::PlatformCoreEventsState {
        pool: auth_event_state.pool.clone(),
        internal_auth_secret: auth_event_state.internal_auth_secret.clone(),
    });
```

Merge it before `internal`:

```rust
.merge(platform_core_events_router)
```

- [ ] **Step 5: Run route tests and verify GREEN**

Run:

```powershell
cargo test -p api platform_core_events
```

Expected: route tests pass.

## Task 4: Anchor Artifact Importer

**Files:**
- Create: `services/api/src/platform_core_anchor_import.rs`
- Create: `services/api/src/bin/platform_core_anchor_import.rs`
- Modify: `services/api/Cargo.toml`

- [ ] **Step 1: Write parser/checksum unit tests**

Create tests in `services/api/src/platform_core_anchor_import.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_anchor_entry_into_db_row() {
        let entry = parse_anchor_entry(
            r#"{"schema_version":"platform-core.parcel_marker_anchor_artifact_entry.v1","pnu":"1111010100100090000","anchor_lng":126.978,"anchor_lat":37.5665,"anchor_srid":"EPSG:4326","algorithm":"polylabel","algorithm_version":"postgis-st_maximuminscribedcircle-v1","source_geometry_checksum_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
        )
        .expect("entry");

        assert_eq!(entry.pnu, "1111010100100090000");
        assert_eq!(entry.algorithm_version, "postgis-st_maximuminscribedcircle-v1");
    }

    #[test]
    fn rejects_wrong_entry_srid() {
        let err = parse_anchor_entry(
            r#"{"schema_version":"platform-core.parcel_marker_anchor_artifact_entry.v1","pnu":"1111010100100090000","anchor_lng":126.978,"anchor_lat":37.5665,"anchor_srid":"EPSG:3857","algorithm":"polylabel","algorithm_version":"postgis-st_maximuminscribedcircle-v1","source_geometry_checksum_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#,
        )
        .unwrap_err();

        assert!(err.to_string().contains("EPSG:4326"));
    }
}
```

- [ ] **Step 2: Run parser tests and verify RED**

Run:

```powershell
cargo test -p api platform_core_anchor_import
```

Expected: compile failure because the importer module does not exist.

- [ ] **Step 3: Add existing workspace checksum dependency if needed**

In `services/api/Cargo.toml`, add:

```toml
sha2 = { workspace = true }
```

- [ ] **Step 4: Implement manifest and entry parsing**

Create `services/api/src/platform_core_anchor_import.rs` with:

```rust
//! Platform Core anchor artifact importer.

use db::platform_core_anchor::{AnchorArtifactRow, PlatformCoreAnchorImport};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

const MANIFEST_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_manifest.v1";
const ENTRY_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_entry.v1";

#[derive(Debug, Error)]
pub enum AnchorImportError {
    #[error("invalid anchor artifact json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("anchor artifact contract mismatch: {0}")]
    Contract(&'static str),
    #[error("anchor artifact checksum mismatch")]
    Checksum,
}

#[derive(Debug, Deserialize)]
struct AnchorArtifactEntry {
    schema_version: String,
    pnu: String,
    anchor_lng: f64,
    anchor_lat: f64,
    anchor_srid: String,
    algorithm: String,
    algorithm_version: String,
    source_geometry_checksum_sha256: String,
}

pub fn parse_anchor_entry(line: &str) -> Result<AnchorArtifactRow, AnchorImportError> {
    let entry: AnchorArtifactEntry = serde_json::from_str(line)?;
    if entry.schema_version != ENTRY_SCHEMA_VERSION {
        return Err(AnchorImportError::Contract("entry schema_version"));
    }
    if entry.anchor_srid != "EPSG:4326" {
        return Err(AnchorImportError::Contract("entry anchor_srid must be EPSG:4326"));
    }
    if entry.pnu.len() != 19 || !entry.pnu.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(AnchorImportError::Contract("entry pnu"));
    }
    if !entry.anchor_lng.is_finite() || !(-180.0..=180.0).contains(&entry.anchor_lng) {
        return Err(AnchorImportError::Contract("entry anchor_lng"));
    }
    if !entry.anchor_lat.is_finite() || !(-90.0..=90.0).contains(&entry.anchor_lat) {
        return Err(AnchorImportError::Contract("entry anchor_lat"));
    }

    Ok(AnchorArtifactRow {
        pnu: entry.pnu,
        anchor_lng: entry.anchor_lng,
        anchor_lat: entry.anchor_lat,
        algorithm: entry.algorithm,
        algorithm_version: entry.algorithm_version,
        source_geometry_checksum_sha256: entry.source_geometry_checksum_sha256,
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::with_capacity(64), |mut out, byte| {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
        out
    })
}
```

- [ ] **Step 5: Run importer tests and verify GREEN**

Run:

```powershell
cargo test -p api platform_core_anchor_import
```

Expected: parser tests pass.

## Task 5: Next.js Public Receiver Forwarding

**Files:**
- Modify: `apps/web/app/platform-core/events/route.ts`
- Modify: `apps/web/tests/unit/platform-core-events.test.ts`

- [ ] **Step 1: Write failing forwarding tests**

In `apps/web/tests/unit/platform-core-events.test.ts`, mock `global.fetch` and add:

```ts
it("returns retryable failure when Rust API inbox write fails for anchor events", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ reason: "inbox_write_failed", status: "rejected" }), {
        status: 500,
        headers: { "content-type": "application/json" },
      }),
    ),
  );

  const res = await POST(
    makeRequest(anchorEventBody(), {
      "x-platform-core-event-type": anchorEventType,
    }),
  );
  const json = await res.json();

  expect(res.status).toBe(503);
  expect(json).toEqual({ reason: "durable_inbox_unavailable", status: "rejected" });
});
```

- [ ] **Step 2: Run the focused test and verify RED**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: new test fails because the route currently acknowledges anchor events without forwarding.

- [ ] **Step 3: Forward accepted events to Rust API**

In `apps/web/app/platform-core/events/route.ts`, import `env`:

```ts
import { env } from "@/lib/env";
```

Add:

```ts
async function persistPlatformCoreEvent(event: PlatformCoreEventEnvelope): Promise<boolean> {
  const res = await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/platform-core/events`, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "x-internal-auth": env.INTERNAL_AUTH_SECRET,
    },
    body: JSON.stringify(event),
  });
  return res.ok;
}
```

In `handleParcelAnchorSnapshotEvent`, persist before returning the ack:

```ts
async function handleParcelAnchorSnapshotEvent(
  value: unknown,
): Promise<AcceptedResponse | undefined | "durable_inbox_unavailable"> {
  const parsed = ParcelAnchorSnapshotEventSchema.safeParse(value);
  if (!parsed.success) return undefined;

  const persisted = await persistPlatformCoreEvent(parsed.data);
  if (!persisted) return "durable_inbox_unavailable";

  return accepted(parsed.data, "enqueue_anchor_projection_import");
}
```

Change `EventHandler` and `POST` to await async handlers and return status 503 for `"durable_inbox_unavailable"`.

- [ ] **Step 4: Run tests and verify GREEN**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts
```

Expected: all receiver tests pass.

## Task 6: Boundary Guardrails

**Files:**
- Modify: `docs/architecture/platform-core-boundary.v1.json`
- Modify: `scripts/ci/check-platform-core-boundary.ps1`
- Modify: `scripts/ci/check-platform-core-boundary.tests.ps1`

- [ ] **Step 1: Add failing boundary test fixtures**

In `scripts/ci/check-platform-core-boundary.tests.ps1`, add required ownership entries for:

```json
{"path":"migrations/30016_platform_core_event_inbox_anchor_import.sql","owner":"gongzzang","classification":"platform_core_event_inbox"},
{"path":"crates/db/src/platform_core_anchor.rs","owner":"gongzzang","classification":"platform_core_read_model_import"},
{"path":"services/api/src/routes/platform_core_events.rs","owner":"gongzzang","classification":"platform_core_event_receiver"},
{"path":"services/api/src/bin/platform_core_anchor_import.rs","owner":"gongzzang","classification":"platform_core_read_model_importer"}
```

- [ ] **Step 2: Run boundary tests and verify RED**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests.ps1
```

Expected: fail because checker does not require the new entries.

- [ ] **Step 3: Update boundary SSOT and checker**

Add the same entries to `docs/architecture/platform-core-boundary.v1.json` and `$RequiredPathOwnership` in `scripts/ci/check-platform-core-boundary.ps1`.

- [ ] **Step 4: Run boundary tests and verify GREEN**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.ps1 -Root .
```

Expected:

```text
check-platform-core-boundary-tests-ok
platform-core-boundary-ok
```

## Task 7: Focused Verification

**Files:**
- Verify only.

- [ ] **Step 1: Run Gongzzang web receiver tests**

Run:

```powershell
pnpm --filter @gongzzang/web test -- tests/unit/platform-core-events.test.ts tests/unit/platform-core-proxy.test.ts tests/unit/map/vector-tile-manifest.test.ts tests/unit/map/marker-tile-style.test.ts
```

Expected: all non-live tests pass; live manifest test remains skipped unless `PLATFORM_CORE_MANIFEST_LIVE_BASE_URL` is set.

- [ ] **Step 2: Run web typecheck**

Run:

```powershell
pnpm --filter @gongzzang/web typecheck
```

Expected: `tsc --noEmit` exits 0.

- [ ] **Step 3: Run Rust focused tests**

Run:

```powershell
cargo test -p api platform_core_events
cargo test -p api platform_core_anchor_import
cargo test -p db --features integration --test platform_core_anchor_import_integration
```

Expected: all tests pass when `DATABASE_URL` points at a migrated PostGIS database.

- [ ] **Step 4: Run guardrails**

Run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-platform-core-boundary.ps1 -Root .
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.tests.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\ci\check-pnu-anchor-pbf-marker-contract.ps1 -Root .
```

Expected: all four commands exit 0.

- [ ] **Step 5: Run diff whitespace checks**

Run:

```powershell
git diff --check -- migrations/30016_platform_core_event_inbox_anchor_import.sql crates/db/src/platform_core_anchor.rs crates/db/src/lib.rs crates/db/tests/platform_core_anchor_import_integration.rs services/api/src/routes/platform_core_events.rs services/api/src/main.rs services/api/src/bin/platform_core_anchor_import.rs services/api/src/platform_core_anchor_import.rs services/api/Cargo.toml apps/web/app/platform-core/events/route.ts apps/web/tests/unit/platform-core-events.test.ts docs/architecture/platform-core-boundary.v1.json scripts/ci/check-platform-core-boundary.ps1 scripts/ci/check-platform-core-boundary.tests.ps1 scripts/ci/check-pnu-anchor-pbf-marker-contract.ps1 scripts/ci/check-pnu-anchor-pbf-marker-contract.tests.ps1
```

Expected: no output and exit 0.

## Self-Review

- Spec coverage: Covers durable event idempotency, Platform Core anchor artifact import, checksum/row validation entry points, read-model upsert, listing projection refresh, receiver forwarding, and guardrails.
- Approval constraints: Migration creation is explicitly blocked until user DB approval. The plan does not require new external package versions.
- Boundary consistency: Platform Core owns anchor coordinates and artifact publication; Gongzzang owns listing projection and listing marker tiles. No Platform Core database access is introduced.
