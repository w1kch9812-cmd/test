//! Integration tests for Platform Core anchor inbox and read-model import.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use db::platform_core_anchor::{
    AnchorArtifactRow, PlatformCoreAnchorImport, PlatformCoreEventInboxInsert,
    PlatformCoreEventInboxStatus,
};
use sqlx::Row;
use tokio::sync::{Mutex, MutexGuard};

use common::{setup_test_pool, truncate_all};

static PLATFORM_CORE_ANCHOR_IMPORT_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

async fn lock_platform_core_anchor_import_tests() -> MutexGuard<'static, ()> {
    PLATFORM_CORE_ANCHOR_IMPORT_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .await
}

fn published_at() -> DateTime<Utc> {
    "2026-05-28T12:00:00Z".parse().unwrap()
}

#[tokio::test]
async fn inbox_insert_is_idempotent_by_platform_core_event_id() {
    let _guard = lock_platform_core_anchor_import_tests().await;
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
async fn inbox_event_payload_can_be_loaded_for_artifact_importer() {
    let _guard = lock_platform_core_anchor_import_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let event = PlatformCoreEventInboxInsert {
        event_id: "0196f0b0-3e01-7000-8000-000000000007".to_owned(),
        event_type: "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned(),
        scope: "catalog".to_owned(),
        effect: "enqueue_anchor_projection_import".to_owned(),
        status: PlatformCoreEventInboxStatus::PendingImport,
        payload: serde_json::json!({
            "anchor_snapshot_id": "anchor-snapshot-loader",
            "artifact_manifest_url": "https://platform-core.example.com/artifacts/manifest.json"
        }),
        anchor_snapshot_id: Some("anchor-snapshot-loader".to_owned()),
        source_geometry_version: Some("silver.parcel_boundaries@loader".to_owned()),
    };
    db::platform_core_anchor::insert_inbox_event(&pool, &event)
        .await
        .unwrap();

    let payload = db::platform_core_anchor::find_inbox_event_payload(&pool, &event.event_id)
        .await
        .unwrap()
        .expect("payload");

    assert_eq!(
        payload["artifact_manifest_url"],
        "https://platform-core.example.com/artifacts/manifest.json"
    );
    assert!(db::platform_core_anchor::find_inbox_event_payload(
        &pool,
        "0196f0b0-3e01-7000-8000-000000000099",
    )
    .await
    .unwrap()
    .is_none());
}

#[tokio::test]
async fn pending_anchor_import_event_ids_are_loaded_in_received_order_with_limit() {
    let _guard = lock_platform_core_anchor_import_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    for (event_id, status) in [
        (
            "0196f0b0-3e01-7000-8000-000000000011",
            PlatformCoreEventInboxStatus::PendingImport,
        ),
        (
            "0196f0b0-3e01-7000-8000-000000000012",
            PlatformCoreEventInboxStatus::Processing,
        ),
        (
            "0196f0b0-3e01-7000-8000-000000000013",
            PlatformCoreEventInboxStatus::Processed,
        ),
        (
            "0196f0b0-3e01-7000-8000-000000000014",
            PlatformCoreEventInboxStatus::Accepted,
        ),
    ] {
        let event = PlatformCoreEventInboxInsert {
            event_id: event_id.to_owned(),
            event_type: if status == PlatformCoreEventInboxStatus::Accepted {
                "catalog.industrial_complex.gold_pointer.published.v1".to_owned()
            } else {
                "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned()
            },
            scope: "catalog".to_owned(),
            effect: if status == PlatformCoreEventInboxStatus::Accepted {
                "invalidate_catalog_cache".to_owned()
            } else {
                "enqueue_anchor_projection_import".to_owned()
            },
            status,
            payload: serde_json::json!({"schema_version": 1, "event_id": event_id}),
            anchor_snapshot_id: if status == PlatformCoreEventInboxStatus::Accepted {
                None
            } else {
                Some(format!("anchor-snapshot-{event_id}"))
            },
            source_geometry_version: if status == PlatformCoreEventInboxStatus::Accepted {
                None
            } else {
                Some("silver.parcel_boundaries@batch".to_owned())
            },
        };
        db::platform_core_anchor::insert_inbox_event(&pool, &event)
            .await
            .unwrap();
    }

    let ids = db::platform_core_anchor::find_pending_anchor_import_event_ids(&pool, 1)
        .await
        .unwrap();

    assert_eq!(ids, vec!["0196f0b0-3e01-7000-8000-000000000011"]);

    let ids = db::platform_core_anchor::find_pending_anchor_import_event_ids(&pool, 10)
        .await
        .unwrap();

    assert_eq!(
        ids,
        vec![
            "0196f0b0-3e01-7000-8000-000000000011",
            "0196f0b0-3e01-7000-8000-000000000012",
        ]
    );
}

#[tokio::test]
async fn inbox_event_records_processing_processed_and_failed_states() {
    let _guard = lock_platform_core_anchor_import_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let processed_event = PlatformCoreEventInboxInsert {
        event_id: "0196f0b0-3e01-7000-8000-000000000003".to_owned(),
        event_type: "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned(),
        scope: "catalog".to_owned(),
        effect: "enqueue_anchor_projection_import".to_owned(),
        status: PlatformCoreEventInboxStatus::PendingImport,
        payload: serde_json::json!({"schema_version": 1}),
        anchor_snapshot_id: Some("anchor-snapshot-processed".to_owned()),
        source_geometry_version: Some("silver.parcel_boundaries@processed".to_owned()),
    };
    db::platform_core_anchor::insert_inbox_event(&pool, &processed_event)
        .await
        .unwrap();

    let claimed =
        db::platform_core_anchor::mark_inbox_event_processing(&pool, &processed_event.event_id)
            .await
            .unwrap();
    assert!(claimed);

    db::platform_core_anchor::mark_inbox_event_processed(&pool, &processed_event.event_id)
        .await
        .unwrap();
    let processed = sqlx::query(
        r#"
        select status, processed_at is not null as has_processed_at, failed_at is null as no_failed_at
        from platform_core_event_inbox
        where event_id = $1::uuid
        "#,
    )
    .bind(&processed_event.event_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(processed.get::<String, _>("status"), "processed");
    assert!(processed.get::<bool, _>("has_processed_at"));
    assert!(processed.get::<bool, _>("no_failed_at"));

    let failed_event = PlatformCoreEventInboxInsert {
        event_id: "0196f0b0-3e01-7000-8000-000000000004".to_owned(),
        event_type: "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned(),
        scope: "catalog".to_owned(),
        effect: "enqueue_anchor_projection_import".to_owned(),
        status: PlatformCoreEventInboxStatus::PendingImport,
        payload: serde_json::json!({"schema_version": 1}),
        anchor_snapshot_id: Some("anchor-snapshot-failed".to_owned()),
        source_geometry_version: Some("silver.parcel_boundaries@failed".to_owned()),
    };
    db::platform_core_anchor::insert_inbox_event(&pool, &failed_event)
        .await
        .unwrap();
    db::platform_core_anchor::mark_inbox_event_processing(&pool, &failed_event.event_id)
        .await
        .unwrap();
    db::platform_core_anchor::mark_inbox_event_failed(
        &pool,
        &failed_event.event_id,
        "checksum mismatch",
    )
    .await
    .unwrap();

    let failed = sqlx::query(
        r#"
        select status, failed_at is not null as has_failed_at, failure_reason
        from platform_core_event_inbox
        where event_id = $1::uuid
        "#,
    )
    .bind(&failed_event.event_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failed.get::<String, _>("status"), "failed");
    assert!(failed.get::<bool, _>("has_failed_at"));
    assert_eq!(
        failed.get::<String, _>("failure_reason"),
        "checksum mismatch"
    );
}

#[tokio::test]
async fn processing_inbox_event_can_be_reclaimed_for_retry_after_worker_exit() {
    let _guard = lock_platform_core_anchor_import_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let event = PlatformCoreEventInboxInsert {
        event_id: "0196f0b0-3e01-7000-8000-000000000005".to_owned(),
        event_type: "catalog.parcel_marker_anchor.snapshot.published.v1".to_owned(),
        scope: "catalog".to_owned(),
        effect: "enqueue_anchor_projection_import".to_owned(),
        status: PlatformCoreEventInboxStatus::PendingImport,
        payload: serde_json::json!({"schema_version": 1}),
        anchor_snapshot_id: Some("anchor-snapshot-retry".to_owned()),
        source_geometry_version: Some("silver.parcel_boundaries@retry".to_owned()),
    };
    db::platform_core_anchor::insert_inbox_event(&pool, &event)
        .await
        .unwrap();

    let first_claim = db::platform_core_anchor::mark_inbox_event_processing(&pool, &event.event_id)
        .await
        .unwrap();
    let retry_claim = db::platform_core_anchor::mark_inbox_event_processing(&pool, &event.event_id)
        .await
        .unwrap();

    assert!(first_claim);
    assert!(retry_claim);

    db::platform_core_anchor::mark_inbox_event_processed(&pool, &event.event_id)
        .await
        .unwrap();
    let post_processed_claim =
        db::platform_core_anchor::mark_inbox_event_processing(&pool, &event.event_id)
            .await
            .unwrap();

    assert!(!post_processed_claim);
}

#[tokio::test]
async fn anchor_import_upserts_long_algorithm_version_and_refreshes_listing_projection() {
    let _guard = lock_platform_core_anchor_import_tests().await;
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;

    let pnu = "1111010100100090000";
    sqlx::query(
        r#"
        insert into "user" (id, zitadel_sub, email, display_name, user_kind, created_at, updated_at, version)
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
    assert_eq!(
        row.get::<String, _>("source_geometry_checksum_sha256"),
        "b".repeat(64)
    );
}
