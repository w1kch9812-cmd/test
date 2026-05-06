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
        .mark_all_read_by_kind(
            &user_id,
            NotificationKind::ListingBookmarked,
            Utc::now(),
            ctx,
        )
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

// ── SP6-v: NotificationKind enum round-trip + user isolation ────────────

#[tokio::test]
async fn enum_round_trip_listing_approved() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-enum-1", "ntenum1@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    let n = make_notification(user_id.clone(), NotificationKind::ListingApproved);
    repo.insert(&n, test_ctx()).await.expect("insert");

    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].kind, NotificationKind::ListingApproved);

    // DB column 직접 확인 — varchar(50) 'listing_approved' 저장 검증.
    let kind_str: (String,) = sqlx::query_as("select kind from notification where id = $1")
        .bind(n.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(kind_str.0, "listing_approved");
}

#[tokio::test]
async fn enum_round_trip_listing_rejected() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-enum-2", "ntenum2@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    let n = make_notification(user_id.clone(), NotificationKind::ListingRejected);
    repo.insert(&n, test_ctx()).await.expect("insert");

    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread[0].kind, NotificationKind::ListingRejected);
}

#[tokio::test]
async fn unknown_kind_in_db_falls_back_to_other() {
    // forward-compat: DB 에 새 kind 가 들어와도 reader 가 panic X. Other 매핑.
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-fc-1", "ntfc1@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    // raw INSERT — domain layer 가 막을 수 없는 상태 (외부 시스템/마이그 시나리오).
    let new_id = Id::<NotificationMarker>::new();
    sqlx::query(
        "insert into notification (id, user_id, kind, payload, read_at, created_at) \
         values ($1, $2, $3, $4, NULL, now())",
    )
    .bind(new_id.as_str())
    .bind(user_id.as_str())
    .bind("future_kind_we_dont_know")
    .bind(serde_json::json!({}))
    .execute(&pool)
    .await
    .unwrap();

    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread.len(), 1);
    assert_eq!(unread[0].kind, NotificationKind::Other);
}

#[tokio::test]
async fn notifications_user_isolation() {
    // A user 알림 / B user 알림 격리.
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let alice = seed_user(&pool, "zsub-nt-iso-a", "ntisoA@example.com").await;
    let bob = seed_user(&pool, "zsub-nt-iso-b", "ntisoB@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    repo.insert(
        &make_notification(alice.clone(), NotificationKind::ListingBookmarked),
        test_ctx(),
    )
    .await
    .unwrap();
    repo.insert(
        &make_notification(bob.clone(), NotificationKind::ListingApproved),
        test_ctx(),
    )
    .await
    .unwrap();
    repo.insert(
        &make_notification(bob.clone(), NotificationKind::ListingRejected),
        test_ctx(),
    )
    .await
    .unwrap();

    let alice_unread = repo.find_unread_by_user(&alice).await.unwrap();
    let bob_unread = repo.find_unread_by_user(&bob).await.unwrap();
    assert_eq!(alice_unread.len(), 1);
    assert_eq!(bob_unread.len(), 2);
    assert_eq!(alice_unread[0].kind, NotificationKind::ListingBookmarked);
}

#[tokio::test]
async fn mark_all_by_kind_only_affects_target_kind() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-nt-mka", "ntmka@example.com").await;
    let repo = PgNotificationRepository::new(pool.clone());

    // 2 ListingApproved + 2 ListingBookmarked
    for _ in 0..2 {
        repo.insert(
            &make_notification(user_id.clone(), NotificationKind::ListingApproved),
            test_ctx(),
        )
        .await
        .unwrap();
        repo.insert(
            &make_notification(user_id.clone(), NotificationKind::ListingBookmarked),
            test_ctx(),
        )
        .await
        .unwrap();
    }

    let rows = repo
        .mark_all_read_by_kind(
            &user_id,
            NotificationKind::ListingApproved,
            Utc::now(),
            test_ctx(),
        )
        .await
        .expect("bulk");
    assert_eq!(rows, 2);

    // ListingBookmarked 2개만 unread 남음.
    let unread = repo.find_unread_by_user(&user_id).await.unwrap();
    assert_eq!(unread.len(), 2);
    for n in &unread {
        assert_eq!(n.kind, NotificationKind::ListingBookmarked);
    }
}
