//! `PgAdminActionRepository` 통합 테스트 — transactional `audit_log`/`outbox_event`
//! 패턴 첫 검증 (SP5-iii T5).
//!
//! 4 시나리오:
//! 1. `insert` — `admin_action` + `audit_log` 1행 (events 비어 있음 → outbox 0)
//! 2. `insert` with 2 events — 같은 tx 안에서 `outbox_event` 2행
//! 3. system action — `actor_id` `NULL` 로 기록
//! 4. `ctx.metadata` → `audit_log.after_state` 매핑

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use admin_action_domain::entity::AdminAction;
use admin_action_domain::repository::AdminActionRepository;
use chrono::{DateTime, Utc};
use db::admin_action::PgAdminActionRepository;
use db::user::PgUserRepository;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::email::Email;
use shared_kernel::id::{AdminActionMarker, Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

/// 테스트용 단순 도메인 이벤트.
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

/// admin 사용자 1명 시드 후 `id` 반환.
async fn seed_admin(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let admin = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Admin",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let admin_id = admin.id.clone();
    repo.save(&admin).await.unwrap();
    admin_id
}

fn make_action(admin_id: Id<UserMarker>, kind: &str) -> AdminAction {
    AdminAction::try_new(
        Id::<AdminActionMarker>::new(),
        admin_id,
        kind,
        Some("user"),
        Some("usr_01HXY3NK0Z9F6S1B2C3D4E5F6G"),
        serde_json::json!({"reason": "test"}),
        "corr_01HXY3NK0Z9F6S1B2",
        Utc::now(),
    )
    .expect("admin action")
}

#[tokio::test]
async fn insert_creates_admin_action_and_audit_log_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool, "zsub-aa-1", "admin1@example.com").await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = make_action(admin_id.clone(), "verify_business");
    let ctx = MutationContext::new_user_action(admin_id, "corr_01HXY3NK0Z9F6S1B2", "create");
    repo.insert(&action, ctx).await.expect("insert");

    // admin_action row 1 개
    let action_count: (i64,) = sqlx::query_as("select count(*) from admin_action where id = $1")
        .bind(action.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(action_count.0, 1);

    // audit_log row 1 개 (resource_kind = 'admin_action')
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'admin_action' and resource_id = $1",
    )
    .bind(action.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    // outbox 0 개 (events 비어 있음)
    let outbox_count: (i64,) = sqlx::query_as("select count(*) from outbox_event")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(outbox_count.0, 0);
}

#[tokio::test]
async fn insert_with_events_creates_outbox_in_same_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool, "zsub-aa-2", "admin2@example.com").await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = make_action(admin_id.clone(), "approve_listing");

    let event1: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "admin.listing_approved",
        aggregate_id: action.id.as_str().to_owned(),
        payload: serde_json::json!({"listing_id": "lst_x"}),
        occurred_at: Utc::now(),
    });
    let event2: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "admin.notification_sent",
        aggregate_id: action.id.as_str().to_owned(),
        payload: serde_json::json!({}),
        occurred_at: Utc::now(),
    });

    let ctx = MutationContext::new_user_action(admin_id, "corr_01HXY3NK0Z9F6S1B3", "approve")
        .with_events(vec![event1, event2]);
    repo.insert(&action, ctx).await.expect("insert");

    // outbox 2 개 (aggregate_kind = 'admin_action', aggregate_id = action.id)
    let outbox_count: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'admin_action' and aggregate_id = $1",
    )
    .bind(action.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outbox_count.0, 2);

    // event_type 두 종류 모두 기록
    let approved: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event where event_type = 'admin.listing_approved'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(approved.0, 1);

    let notified: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event where event_type = 'admin.notification_sent'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(notified.0, 1);

    // published_at 은 NULL 로 들어가야 (publisher 미발송 상태)
    let unpublished: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where published_at is null")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(unpublished.0, 2);
}

#[tokio::test]
async fn insert_system_action_records_null_actor() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool, "zsub-aa-3", "admin3@example.com").await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = make_action(admin_id, "system_purge");
    let ctx = MutationContext::new_system_action("corr_01HXY3NK0Z9F6S1B4", "create");
    repo.insert(&action, ctx).await.expect("insert");

    let null_actor_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'admin_action' and actor_id is null",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_actor_count.0, 1);
}

#[tokio::test]
async fn insert_with_metadata_writes_to_after_state() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let admin_id = seed_admin(&pool, "zsub-aa-4", "admin4@example.com").await;
    let repo = PgAdminActionRepository::new(pool.clone());

    let action = make_action(admin_id.clone(), "verify_business");
    let ctx = MutationContext::new_user_action(admin_id, "corr_01HXY3NK0Z9F6S1B5", "create")
        .with_metadata(serde_json::json!({"verification_id": "v-123"}));
    repo.insert(&action, ctx).await.expect("insert");

    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log where resource_kind = 'admin_action' limit 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        after_state,
        Some(serde_json::json!({"verification_id": "v-123"}))
    );
}
