# Platform Core Anchor Projection Import Plan - Part 02: DB Repository Tests

Parent index: [Platform Core Anchor Projection Import Implementation Plan](./2026-05-28-platform-core-anchor-projection-import.md).


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

```bash
cargo test -p db --features integration --test platform_core_anchor_import_integration
```

Expected: compile failure because `db::platform_core_anchor` does not exist.
