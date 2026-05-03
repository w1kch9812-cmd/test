//! `map_sqlx_err` unique violation 분기 검증 — 진짜 `PG` `INSERT` 중복으로 검증.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

use common::{setup_test_pool, truncate_all};

#[tokio::test]
async fn unique_violation_zitadel_sub_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "same-zsub",
        Email::try_new("a@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "same-zsub", // 같은 zitadel_sub — `UNIQUE` 위반
        Email::try_new("b@x.com").unwrap(),
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn unique_violation_email_maps_to_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgUserRepository::new(pool);

    let now = Utc::now();
    let u1 = User::try_new(
        Id::new(),
        "zsub-1",
        Email::try_new("dup@x.com").unwrap(),
        "User1",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let u2 = User::try_new(
        Id::new(),
        "zsub-2",
        Email::try_new("dup@x.com").unwrap(), // 같은 email — `UNIQUE`
        "User2",
        UserKind::Individual,
        now,
    )
    .unwrap();

    repo.save(&u1).await.expect("first save");
    let err = repo.save(&u2).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}
