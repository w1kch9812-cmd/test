//! `PgRawCapture` 통합 테스트 (SP4-iii-d) — UPSERT into `parcel_external_data`.
//!
//! 3 시나리오:
//! 1. 단일 row INSERT → `raw_response` JSONB round-trip
//! 2. 같은 (pnu, source) 재호출 — `UPSERT` row 1개 유지, `fetched_at` 갱신
//! 3. 다른 source 는 별도 row — (pnu, vworld) + (pnu, `data_go_kr_building`) → 2 rows

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::{Duration, Utc};
use db::raw_capture::PgRawCapture;
use raw_capture_client::RawCapture;

use common::{setup_test_pool, truncate_all};

#[tokio::test]
async fn pg_raw_capture_inserts_row() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let capture = PgRawCapture::new(pool.clone());

    let raw = serde_json::json!({"properties": {"pnu": "1111010100100010000"}, "extra": 42});
    let now = Utc::now();
    capture
        .capture("1111010100100010000", "vworld", &raw, now)
        .await
        .expect("capture ok");

    let count: (i64,) =
        sqlx::query_as("select count(*) from parcel_external_data where pnu = $1 and source = $2")
            .bind("1111010100100010000")
            .bind("vworld")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);

    let stored: (serde_json::Value,) = sqlx::query_as(
        "select raw_response from parcel_external_data where pnu = $1 and source = $2",
    )
    .bind("1111010100100010000")
    .bind("vworld")
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(stored.0, raw);
}

#[tokio::test]
async fn pg_raw_capture_upserts_on_same_pnu_source() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let capture = PgRawCapture::new(pool.clone());

    let raw1 = serde_json::json!({"v": 1});
    let raw2 = serde_json::json!({"v": 2});
    let t1 = Utc::now() - Duration::seconds(60);
    let t2 = Utc::now();

    capture
        .capture("1111010100100020000", "vworld", &raw1, t1)
        .await
        .unwrap();
    capture
        .capture("1111010100100020000", "vworld", &raw2, t2)
        .await
        .unwrap();

    let row: (serde_json::Value, chrono::DateTime<Utc>) = sqlx::query_as(
        "select raw_response, fetched_at from parcel_external_data \
         where pnu = $1 and source = $2",
    )
    .bind("1111010100100020000")
    .bind("vworld")
    .fetch_one(&pool)
    .await
    .unwrap();

    // raw_response 갱신 + fetched_at 갱신
    assert_eq!(row.0, raw2);
    assert!(
        (row.1.timestamp_millis() - t2.timestamp_millis()).abs() < 1_000,
        "fetched_at should match t2"
    );

    // row count 는 여전히 1
    let count: (i64,) =
        sqlx::query_as("select count(*) from parcel_external_data where pnu = $1 and source = $2")
            .bind("1111010100100020000")
            .bind("vworld")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn pg_raw_capture_different_sources_separate_rows() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let capture = PgRawCapture::new(pool.clone());

    let pnu = "1111010100100030000";
    let now = Utc::now();
    capture
        .capture(pnu, "vworld", &serde_json::json!({"src": "vworld"}), now)
        .await
        .unwrap();
    capture
        .capture(
            pnu,
            "data_go_kr_building",
            &serde_json::json!({"src": "building"}),
            now,
        )
        .await
        .unwrap();

    let count: (i64,) = sqlx::query_as("select count(*) from parcel_external_data where pnu = $1")
        .bind(pnu)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count.0, 2);
}
