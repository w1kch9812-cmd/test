//! `PgUserRepository` 통합 테스트 — 18 필드 round-trip + `OCC` + `Conflict`.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use user_domain::entity::{User, UserKind, UserRole};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, test_ctx, truncate_all};

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
