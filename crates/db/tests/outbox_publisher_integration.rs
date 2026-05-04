//! `outbox_publisher::tick` 통합 테스트 (SP4-i).
//!
//! 4 시나리오:
//! 1. `tick_publishes_unpublished_rows` — 3 row INSERT → tick → 모두 published
//! 2. `tick_skips_already_published` — 이미 published 된 row 는 fetch 안 잡힘
//! 3. `tick_returns_zero_when_no_rows` — 빈 테이블에서 tick → 0
//! 4. `tick_failure_leaves_row_unpublished` — `FailingSink` → row 그대로

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use async_trait::async_trait;
use chrono::Utc;
use db::outbox::PgOutboxRepository;
use outbox_event_domain::entity::OutboxEvent;
use outbox_event_domain::repository::OutboxRepository;
use outbox_publisher::{tick, CountingSink, Sink, SinkError};
use shared_kernel::id::{Id, OutboxEventMarker};

use common::{setup_test_pool, truncate_all};

/// 시드용 — 단일 outbox event row INSERT.
async fn insert_event(
    pool: &sqlx::PgPool,
    aggregate_id: &str,
    kind: &str,
) -> Id<OutboxEventMarker> {
    let repo = PgOutboxRepository::new(pool.clone());
    let event = OutboxEvent {
        id: Id::<OutboxEventMarker>::new(),
        event_type: format!("{kind}.test_event"),
        aggregate_kind: kind.to_owned(),
        aggregate_id: aggregate_id.to_owned(),
        payload: serde_json::json!({"k": "v"}),
        occurred_at: Utc::now(),
        published_at: None,
        correlation_id: "corr-test".to_owned(),
    };
    let id = event.id.clone();
    repo.save(&event).await.expect("seed save");
    id
}

#[tokio::test]
async fn tick_publishes_unpublished_rows() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool.clone());

    insert_event(&pool, "agg-1", "user").await;
    insert_event(&pool, "agg-2", "listing").await;
    insert_event(&pool, "agg-3", "listing_photo").await;

    let sink = CountingSink::new();
    let report = tick(&repo, &sink, 100).await.expect("tick");

    assert_eq!(report.fetched, 3);
    assert_eq!(report.published, 3);
    assert_eq!(report.failed, 0);
    assert_eq!(sink.count(), 3);

    let published_count: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where published_at is not null")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(published_count.0, 3);
}

#[tokio::test]
async fn tick_skips_already_published() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool.clone());

    let already_id = insert_event(&pool, "agg-already", "user").await;
    repo.mark_published(&already_id, Utc::now())
        .await
        .expect("mark");

    insert_event(&pool, "agg-new", "user").await;

    let sink = CountingSink::new();
    let report = tick(&repo, &sink, 100).await.expect("tick");

    assert_eq!(report.fetched, 1);
    assert_eq!(report.published, 1);
    assert_eq!(sink.count(), 1);

    let published_count: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where published_at is not null")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(published_count.0, 2);
}

#[tokio::test]
async fn tick_returns_zero_when_no_rows() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let sink = CountingSink::new();
    let report = tick(&repo, &sink, 100).await.expect("tick");

    assert_eq!(report.fetched, 0);
    assert_eq!(report.published, 0);
    assert_eq!(report.failed, 0);
    assert_eq!(sink.count(), 0);
}

/// 항상 실패하는 sink — `tick_failure_leaves_row_unpublished` 시나리오용.
struct FailingSink;

#[async_trait]
impl Sink for FailingSink {
    async fn publish(&self, _event: &OutboxEvent) -> Result<(), SinkError> {
        Err(SinkError::Publish("intentional test failure".to_owned()))
    }
}

#[tokio::test]
async fn tick_failure_leaves_row_unpublished() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool.clone());

    insert_event(&pool, "agg-fail", "user").await;

    let sink = FailingSink;
    let report = tick(&repo, &sink, 100).await.expect("tick");

    assert_eq!(report.fetched, 1);
    assert_eq!(report.published, 0);
    assert_eq!(report.failed, 1);

    // row 는 미발행 그대로
    let unpublished_count: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where published_at is null")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(unpublished_count.0, 1);
}
