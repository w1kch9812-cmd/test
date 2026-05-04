//! `PgUserRepository` 통합 테스트 — 18 필드 round-trip + `OCC` + `Conflict`
//! + SP5-iv transactional `audit_log` / `outbox_event` 검증.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::user::PgUserRepository;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use user_domain::entity::{User, UserKind, UserRole};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, test_ctx, truncate_all};

/// 테스트용 단순 도메인 이벤트 (`MutationContext.events` 검증용).
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

fn make_user(zsub: &str, email: &str) -> User {
    let now = Utc::now();
    User::try_new_full(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        None, // phone_kr_hash
        "Test User",
        UserKind::Individual,
        None, // business_number
        None, // business_verified_at
        None, // broker_license_number
        None, // broker_verified_at
        vec![UserRole::Buyer, UserRole::Seller],
        None, // nice_verified_at
        None, // marketing_consent_at
        now,
    )
    .expect("user")
}

#[tokio::test]
async fn round_trip_user_with_18_fields() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-1", "alice@example.com");
    repo.save(&user, test_ctx()).await.expect("save");

    let fetched = repo
        .find_by_id(&user.id)
        .await
        .expect("find")
        .expect("Some");
    assert_eq!(fetched.zitadel_sub, user.zitadel_sub);
    assert_eq!(fetched.email.as_str(), user.email.as_str());
    assert_eq!(fetched.display_name, user.display_name);
    assert_eq!(fetched.user_kind, user.user_kind);
    assert_eq!(fetched.roles, user.roles); // SP3 에서 누락됐던 round-trip
    assert_eq!(fetched.version, 1);
    assert!(fetched.deleted_at.is_none());
    assert!(fetched.last_login_at.is_none());
}

#[tokio::test]
async fn find_by_zitadel_sub_returns_user() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-2", "bob@example.com");
    repo.save(&user, test_ctx()).await.expect("save");

    let fetched = repo
        .find_by_zitadel_sub("zsub-2")
        .await
        .expect("find")
        .expect("Some");
    assert_eq!(fetched.id.as_str(), user.id.as_str());
}

#[tokio::test]
async fn find_by_email_returns_user() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let user = make_user("zsub-3", "carol@example.com");
    repo.save(&user, test_ctx()).await.expect("save");

    let email = Email::try_new("carol@example.com").unwrap();
    let fetched = repo
        .find_by_email(&email)
        .await
        .expect("find")
        .expect("Some");
    assert_eq!(fetched.id.as_str(), user.id.as_str());
}

#[tokio::test]
async fn duplicate_zitadel_sub_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let u1 = make_user("zsub-dup", "u1@example.com");
    let u2 = make_user("zsub-dup", "u2@example.com");
    repo.save(&u1, test_ctx()).await.expect("first save ok");

    let err = repo.save(&u2, test_ctx()).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let mut user = make_user("zsub-occ", "occ@example.com");
    repo.save(&user, test_ctx()).await.expect("save v1");

    // 직접 version 을 안 맞게 조작 — 동시 update 시뮬레이션
    user.version = 99;
    let err = repo.save(&user, test_ctx()).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn find_nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let id: Id<UserMarker> = Id::new();
    let fetched = repo.find_by_id(&id).await.expect("find");
    assert!(fetched.is_none());
}

// ---- SP5-iv: transactional audit_log + outbox_event 검증 ----

#[tokio::test]
async fn save_inserts_user_audit_log_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool.clone());

    let user = make_user("zsub-audit-1", "audit1@example.com");
    let ctx = MutationContext::new_user_action(user.id.clone(), "corr-audit-1", "create");
    repo.save(&user, ctx).await.expect("save");

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'user' and resource_id = $1",
    )
    .bind(user.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    // events 비어 있음 → outbox 0
    let outbox_count: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where aggregate_kind = 'user'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(outbox_count.0, 0);
}

#[tokio::test]
async fn save_user_with_events_inserts_outbox_per_event() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool.clone());

    let user = make_user("zsub-events-1", "events1@example.com");
    let event1: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "user.created",
        aggregate_id: user.id.as_str().to_owned(),
        payload: serde_json::json!({"sub": "z1"}),
        occurred_at: Utc::now(),
    });
    let event2: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "user.welcome_email_queued",
        aggregate_id: user.id.as_str().to_owned(),
        payload: serde_json::json!({}),
        occurred_at: Utc::now(),
    });
    let ctx = MutationContext::new_user_action(user.id.clone(), "corr-events-1", "create")
        .with_events(vec![event1, event2]);
    repo.save(&user, ctx).await.expect("save");

    let outbox_count: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'user' and aggregate_id = $1",
    )
    .bind(user.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outbox_count.0, 2);

    let unpublished: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'user' and published_at is null",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unpublished.0, 2);
}

#[tokio::test]
async fn save_user_system_action_records_null_actor() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool.clone());

    let user = make_user("zsub-sys-1", "sys1@example.com");
    let ctx = MutationContext::new_system_action("corr-sys-1", "first_sign_in");
    repo.save(&user, ctx).await.expect("save");

    let null_actor_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'user' and resource_id = $1 and actor_id is null",
    )
    .bind(user.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_actor_count.0, 1);
}

#[tokio::test]
async fn save_user_with_metadata_writes_to_after_state() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool.clone());

    let user = make_user("zsub-meta-1", "meta1@example.com");
    let ctx = MutationContext::new_system_action("corr-meta-1", "first_sign_in")
        .with_metadata(serde_json::json!({"zitadel_sub": "zsub-meta-1"}));
    repo.save(&user, ctx).await.expect("save");

    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log \
         where resource_kind = 'user' and resource_id = $1",
    )
    .bind(user.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        after_state,
        Some(serde_json::json!({"zitadel_sub": "zsub-meta-1"}))
    );
}
