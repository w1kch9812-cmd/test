//! `PgOutboxRepository` 통합 테스트 — `save` + `fetch_unpublished` +
//! `mark_published` + `NotFound` + `ORDER BY` `ASC` 검증.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::{DateTime, Duration, Utc};
use db::outbox::PgOutboxRepository;
use outbox_event_domain::entity::OutboxEvent;
use outbox_event_domain::repository::{OutboxRepository, RepoError};
use serde_json::json;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::id::{Id, OutboxEventMarker};

use common::{setup_test_pool, truncate_all};

/// 테스트 전용 [`DomainEvent`] — 실제 `Aggregate` 이벤트가 sub-project 6 에서
/// 추가되기 전까지 통합 테스트에서 [`OutboxEvent::from_domain`] 호출용.
#[derive(Debug)]
struct TestEvent {
    event_type: &'static str,
    aggregate_id: String,
    payload: serde_json::Value,
    occurred_at: DateTime<Utc>,
}

impl DomainEvent for TestEvent {
    fn event_type(&self) -> &'static str {
        self.event_type
    }
    fn aggregate_id(&self) -> String {
        self.aggregate_id.clone()
    }
    fn payload(&self) -> serde_json::Value {
        self.payload.clone()
    }
    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }
}

fn make_outbox(event_type: &'static str) -> OutboxEvent {
    make_outbox_at(event_type, Utc::now())
}

fn make_outbox_at(event_type: &'static str, occurred_at: DateTime<Utc>) -> OutboxEvent {
    let event = TestEvent {
        event_type,
        aggregate_id: "agg-test-1".to_owned(),
        payload: json!({ "sample": "payload" }),
        occurred_at,
    };
    OutboxEvent::from_domain(
        Id::<OutboxEventMarker>::new(),
        &event,
        "test_aggregate",
        "corr-1",
    )
    .expect("outbox")
}

#[tokio::test]
async fn save_persists_event() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let event = make_outbox("test.created");
    repo.save(&event).await.expect("save");

    let unpublished = repo.fetch_unpublished(10).await.expect("fetch");
    assert_eq!(unpublished.len(), 1);
    assert_eq!(unpublished[0].event_type, "test.created");
    assert_eq!(unpublished[0].aggregate_kind, "test_aggregate");
    assert_eq!(unpublished[0].correlation_id, "corr-1");
    assert!(unpublished[0].published_at.is_none());
    assert_eq!(unpublished[0].id.as_str(), event.id.as_str());
    assert_eq!(unpublished[0].payload["sample"], "payload");
}

#[tokio::test]
async fn fetch_unpublished_excludes_published() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool.clone());

    let e1 = make_outbox("a.created");
    let e2 = make_outbox("b.created");
    repo.save(&e1).await.expect("save e1");
    repo.save(&e2).await.expect("save e2");

    sqlx::query("update outbox_event set published_at = now() where id = $1")
        .bind(e1.id.as_str())
        .execute(&pool)
        .await
        .expect("manual mark published");

    let unpublished = repo.fetch_unpublished(10).await.expect("fetch");
    assert_eq!(unpublished.len(), 1);
    assert_eq!(unpublished[0].id.as_str(), e2.id.as_str());
}

#[tokio::test]
async fn mark_published_updates_timestamp() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let event = make_outbox("test.created");
    repo.save(&event).await.expect("save");

    let now = Utc::now();
    repo.mark_published(&event.id, now).await.expect("mark");

    // After publishing, fetch_unpublished must not return it.
    let unpublished = repo.fetch_unpublished(10).await.expect("fetch");
    assert_eq!(unpublished.len(), 0);
}

#[tokio::test]
async fn mark_published_nonexistent_returns_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let id: Id<OutboxEventMarker> = Id::new();
    let err = repo
        .mark_published(&id, Utc::now())
        .await
        .expect_err("must fail with NotFound");
    assert!(matches!(err, RepoError::NotFound));
}

#[tokio::test]
async fn mark_published_already_published_returns_not_found() {
    // `WHERE published_at IS NULL` 가 idempotency 가드 — 이미 발행된 row 를
    // 다시 mark 하려 하면 0 rows affected → `NotFound`.
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    let event = make_outbox("test.created");
    repo.save(&event).await.expect("save");
    repo.mark_published(&event.id, Utc::now())
        .await
        .expect("first mark ok");

    let err = repo
        .mark_published(&event.id, Utc::now())
        .await
        .expect_err("second mark must be NotFound");
    assert!(matches!(err, RepoError::NotFound));
}

#[tokio::test]
async fn fetch_unpublished_orders_by_occurred_at_asc() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgOutboxRepository::new(pool);

    // Insert 3 events with explicit decreasing occurred_at so the saved row
    // 의 `created_at` (= entity `occurred_at`) 순서가 결정적이에요.
    let base = Utc::now();
    let early = make_outbox_at("a.early", base - Duration::seconds(20));
    let middle = make_outbox_at("b.middle", base - Duration::seconds(10));
    let late = make_outbox_at("c.late", base);

    // Insert order is intentionally NOT chronological — verifies ORDER BY
    // applies to occurred_at, not insertion order.
    repo.save(&late).await.expect("save late");
    repo.save(&early).await.expect("save early");
    repo.save(&middle).await.expect("save middle");

    let unpublished = repo.fetch_unpublished(10).await.expect("fetch");
    assert_eq!(unpublished.len(), 3);
    assert_eq!(unpublished[0].event_type, "a.early");
    assert_eq!(unpublished[1].event_type, "b.middle");
    assert_eq!(unpublished[2].event_type, "c.late");
}
