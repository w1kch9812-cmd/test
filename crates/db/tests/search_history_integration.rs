//! `PgSearchHistoryRepository` 통합 테스트 (SP5-ii) — append + bulk
//! pseudonymize + audit 검증.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::similar_names
)]
#![cfg(feature = "integration")]

mod common;

use chrono::{Duration, Utc};
use db::search_history::PgSearchHistoryRepository;
use db::user::PgUserRepository;
use search_history_domain::entity::SearchHistory;
use search_history_domain::repository::SearchHistoryRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, SearchHistoryMarker, UserMarker};
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
        "Searcher",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let user_id = user.id.clone();
    repo.save(&user, test_ctx()).await.unwrap();
    user_id
}

#[tokio::test]
async fn insert_round_trip_with_audit() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-sh-1", "sh1@example.com").await;
    let repo = PgSearchHistoryRepository::new(pool.clone());

    let history = SearchHistory::try_new(
        Id::<SearchHistoryMarker>::new(),
        Some(user_id.clone()),
        "성남 지식산업센터",
        serde_json::json!({"region": "성남시"}),
        42,
        "req-sh-1",
        Utc::now(),
    )
    .expect("history");
    let history_id = history.id.clone();
    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-sh-1", "search");
    repo.insert(&history, ctx).await.expect("insert");

    let recent = repo.find_recent_by_user(&user_id, 10).await.expect("find");
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0].query, "성남 지식산업센터");
    assert_eq!(recent[0].result_count, 42);

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'search_history' and resource_id = $1",
    )
    .bind(history_id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn insert_anonymous_user_id_null() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgSearchHistoryRepository::new(pool.clone());

    let history = SearchHistory::try_new(
        Id::<SearchHistoryMarker>::new(),
        None,
        "공장 임대",
        serde_json::json!({}),
        0,
        "req-sh-anon",
        Utc::now(),
    )
    .expect("history");
    let history_id = history.id.clone();
    let ctx = MutationContext::new_system_action("corr-sh-anon", "search");
    repo.insert(&history, ctx).await.expect("insert");

    let null_user: (i64,) =
        sqlx::query_as("select count(*) from search_history where id = $1 and user_id is null")
            .bind(history_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(null_user.0, 1);
}

#[tokio::test]
async fn pseudonymize_older_than_clears_user_id_in_bulk() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-sh-3", "sh3@example.com").await;
    let repo = PgSearchHistoryRepository::new(pool.clone());

    let now = Utc::now();
    let old_at = now - Duration::days(100);

    // 오래된 row 2개 (created_at 직접 set — 도메인 try_new 후 row INSERT 시점 변경 필요)
    for i in 0..2 {
        let h = SearchHistory::try_new(
            Id::<SearchHistoryMarker>::new(),
            Some(user_id.clone()),
            &format!("old query {i}"),
            serde_json::json!({}),
            0,
            "req-old",
            old_at,
        )
        .expect("history");
        repo.insert(&h, test_ctx()).await.expect("insert old");
    }

    // 최신 row 1개 — pseudonymize 영향 없어야
    let recent = SearchHistory::try_new(
        Id::<SearchHistoryMarker>::new(),
        Some(user_id.clone()),
        "recent query",
        serde_json::json!({}),
        0,
        "req-new",
        now,
    )
    .expect("history");
    repo.insert(&recent, test_ctx()).await.expect("insert new");

    // pseudonymize: cutoff = now - 90 days → 100일전 row 들만 영향
    let cutoff = now - Duration::days(90);
    let ctx = MutationContext::new_system_action("corr-sh-pseudo", "pseudonymize");
    let rows = repo
        .pseudonymize_older_than(cutoff, ctx)
        .await
        .expect("pseudonymize");
    assert_eq!(rows, 2);

    // 최신 row 의 user_id 는 보존
    let still_attributed: (i64,) =
        sqlx::query_as("select count(*) from search_history where user_id = $1")
            .bind(user_id.as_str())
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(still_attributed.0, 1);

    // bulk audit row 1개 — resource_kind = 'search_history', resource_id 는
    // cutoff_<ts> prefix 로 시작
    let bulk_audit: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'search_history' and resource_id like 'cutoff_%'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bulk_audit.0, 1);
}

#[tokio::test]
async fn pseudonymize_metadata_contains_rows_pseudonymized() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-sh-4", "sh4@example.com").await;
    let repo = PgSearchHistoryRepository::new(pool.clone());

    let old_at = Utc::now() - Duration::days(120);
    let h = SearchHistory::try_new(
        Id::<SearchHistoryMarker>::new(),
        Some(user_id),
        "to be pseudonymized",
        serde_json::json!({}),
        0,
        "req-old-meta",
        old_at,
    )
    .expect("history");
    repo.insert(&h, test_ctx()).await.unwrap();

    let cutoff = Utc::now() - Duration::days(90);
    let ctx = MutationContext::new_system_action("corr-meta", "pseudonymize");
    repo.pseudonymize_older_than(cutoff, ctx).await.unwrap();

    let after_state: Option<serde_json::Value> = sqlx::query_scalar(
        "select after_state from audit_log \
         where resource_kind = 'search_history' and resource_id like 'cutoff_%' \
         order by created_at desc limit 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let meta = after_state.expect("after_state present");
    assert_eq!(meta["rows_pseudonymized"], 1);
    assert!(meta["cutoff_iso"].is_string());
}
