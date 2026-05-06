//! `PgNotificationRepository` 통합 테스트 (SP5-ii) — insert + idempotent
//! `mark_read` + bulk `mark_all_read_by_kind` + audit 검증.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::similar_names
)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::notification::PgNotificationRepository;
use db::user::PgUserRepository;
use notification_domain::entity::Notification;
use notification_domain::kind::NotificationKind;
use notification_domain::repository::NotificationRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, NotificationMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

async fn seed_user(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let user = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Receiver",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let user_id = user.id.clone();
    repo.save(&user, test_ctx()).await.unwrap();
    user_id
}

fn make_notification(user_id: Id<UserMarker>, kind: NotificationKind) -> Notification {
    Notification::new(
        Id::<NotificationMarker>::new(),
        user_id,
        kind,
        serde_json::json!({"listing_id": "lst_x"}),
        Utc::now(),
    )
}

#[tokio::test]
async fn insert_round_trip_with_audit() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-1", "nt1@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    let n = make_notification(user_id.clone(), NotificationKind::ListingBookmarked);
    let ctx = MutationContext::new_system_action("corr-nt-1", "notify");
    repo.insert(&n, ctx).await.expect("insert");

    let unread = repo
        .find_unread_by_user(&user_id)
        .await
        .expect("find unread");
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].kind, NotificationKind::ListingBookmarked);
    assert!(unread[0].read_at.is_none());

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'notification' and resource_id = $1",
    )
    .bind(n.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn mark_read_moves_from_unread_to_read() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-2", "nt2@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    let n = make_notification(user_id.clone(), NotificationKind::ListingApproved);
    repo.insert(&n, test_ctx()).await.unwrap();

    let read_at = Utc::now();
    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-nt-2", "mark_read");
    repo.mark_read(&n.id, read_at, ctx)
        .await
        .expect("mark_read");

    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread.len(), 0);

    let recent = repo.find_recent_by_user(&user_id, 10).await.unwrap();
    assert_eq!(recent.len(), 1);
    assert!(recent[0].read_at.is_some());
}

#[tokio::test]
async fn mark_read_idempotent_on_already_read() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-3", "nt3@example.com").await;
    let repo = PgNotificationRepository::new(pool);

    let n = make_notification(user_id.clone(), NotificationKind::ListingBookmarked);
    repo.insert(&n, test_ctx()).await.unwrap();

    repo.mark_read(&n.id, Utc::now(), test_ctx())
        .await
        .expect("first mark_read");
    // 두 번째 호출 — 멱등 (Ok 반환, NotFound 없음)
    repo.mark_read(&n.id, Utc::now(), test_ctx())
        .await
        .expect("second mark_read idempotent");
}

#[tokio::test]
async fn mark_all_read_by_kind_bulk() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-4", "nt4@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    // 3 개 ListingBookmarked + 1 개 ListingApproved
    for _ in 0..3 {
        repo.insert(
            &make_notification(user_id.clone(), NotificationKind::ListingBookmarked),
            test_ctx(),
        )
        .await
        .unwrap();
    }
    repo.insert(
        &make_notification(user_id.clone(), NotificationKind::ListingApproved),
        test_ctx(),
    )
    .await
    .unwrap();

    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-nt-bulk", "mark_all");
    let rows = repo
        .mark_all_read_by_kind(&user_id, NotificationKind::ListingBookmarked, Utc::now(), ctx)
        .await
        .expect("bulk");
    assert_eq!(rows, 3);

    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread.len(), 1); // ListingApproved 만 남음
    assert_eq!(unread[0].kind, NotificationKind::ListingApproved);

    // bulk audit row 1개 + metadata 검증
    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log \
         where resource_kind = 'notification' and resource_id = $1 \
         order by created_at desc limit 1",
    )
    .bind(user_id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    let meta = after_state.expect("after_state present");
    assert_eq!(meta["kind"], "listing_bookmarked");
    assert_eq!(meta["rows_marked"], 3);
}
