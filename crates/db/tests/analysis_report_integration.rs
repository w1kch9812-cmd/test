//! `PgAnalysisReportRepository` 통합 테스트 (SP5-ii) — OCC + target_pnus[]
//! + audit/outbox 검증.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::similar_names
)]
#![cfg(feature = "integration")]

mod common;

use analysis_report_domain::entity::AnalysisReport;
use analysis_report_domain::repository::{AnalysisReportRepository, RepoError as ArRepoError};
use chrono::Utc;
use db::analysis_report::PgAnalysisReportRepository;
use db::user::PgUserRepository;
use shared_kernel::email::Email;
use shared_kernel::id::{AnalysisReportMarker, Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
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
        "Analyst",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let user_id = user.id.clone();
    repo.save(&user, test_ctx()).await.unwrap();
    user_id
}

fn make_report(user_id: Id<UserMarker>, title: &str) -> AnalysisReport {
    AnalysisReport::try_new(
        Id::<AnalysisReportMarker>::new(),
        user_id,
        title,
        vec![
            Pnu::try_new("1111010100100010000").unwrap(),
            Pnu::try_new("1111010100100020000").unwrap(),
        ],
        serde_json::json!({"jiga_avg": 1_500_000}),
        Utc::now(),
    )
    .expect("report")
}

#[tokio::test]
async fn round_trip_with_target_pnus() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-ar-1", "ar1@example.com").await;
    let repo = PgAnalysisReportRepository::new(pool.clone());

    let report = make_report(user_id.clone(), "성남 후보지 분석");
    let ctx = MutationContext::new_user_action(user_id.clone(), "corr-ar-1", "create_report");
    repo.save(&report, ctx).await.expect("save");

    let fetched = repo
        .find_by_id(&report.id)
        .await
        .expect("find")
        .expect("Some");
    assert_eq!(fetched.title, "성남 후보지 분석");
    assert_eq!(fetched.target_pnus.len(), 2);
    assert_eq!(fetched.version, 1);
    assert_eq!(fetched.target_pnus[0].as_str(), "1111010100100010000");

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'analysis_report' and resource_id = $1",
    )
    .bind(report.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);
}

#[tokio::test]
async fn save_update_bumps_version() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-ar-2", "ar2@example.com").await;
    let repo = PgAnalysisReportRepository::new(pool);

    let mut report = make_report(user_id, "원래 제목");
    repo.save(&report, test_ctx()).await.expect("v1");

    report.update_snapshot(serde_json::json!({"v": 2}), Utc::now());
    // domain bumps version internally; pass v1 for OCC where clause
    report.version = 1;
    repo.save(&report, test_ctx()).await.expect("v2");

    let fetched = repo.find_by_id(&report.id).await.unwrap().unwrap();
    assert_eq!(fetched.version, 2);
}

#[tokio::test]
async fn save_occ_conflict_returns_error() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-ar-3", "ar3@example.com").await;
    let repo = PgAnalysisReportRepository::new(pool);

    let mut report = make_report(user_id, "OCC test");
    repo.save(&report, test_ctx()).await.expect("v1");

    // 인위적으로 stale version
    report.version = 99;
    let err = repo.save(&report, test_ctx()).await.unwrap_err();
    assert!(matches!(err, ArRepoError::Conflict));
}

#[tokio::test]
async fn delete_removes_report_and_audits() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-ar-4", "ar4@example.com").await;
    let repo = PgAnalysisReportRepository::new(pool.clone());

    let report = make_report(user_id.clone(), "to delete");
    repo.save(&report, test_ctx()).await.expect("save");

    let delete_ctx = MutationContext::new_user_action(user_id, "corr-ar-del", "delete");
    repo.delete(&report.id, delete_ctx).await.expect("delete");

    let after = repo.find_by_id(&report.id).await.unwrap();
    assert!(after.is_none());

    // 2 audit rows: save (action=create_report 등 test_ctx의 'create') + delete
    let delete_audit: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'analysis_report' and action = 'delete' and resource_id = $1",
    )
    .bind(report.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(delete_audit.0, 1);
}

#[tokio::test]
async fn delete_nonexistent_returns_not_found() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgAnalysisReportRepository::new(pool);

    let id: Id<AnalysisReportMarker> = Id::new();
    let err = repo.delete(&id, test_ctx()).await.unwrap_err();
    assert!(matches!(err, ArRepoError::NotFound));
}

#[tokio::test]
async fn find_by_user_returns_user_reports() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let user_id = seed_user(&pool, "zsub-ar-6", "ar6@example.com").await;
    let repo = PgAnalysisReportRepository::new(pool);

    repo.save(&make_report(user_id.clone(), "r1"), test_ctx())
        .await
        .unwrap();
    repo.save(&make_report(user_id.clone(), "r2"), test_ctx())
        .await
        .unwrap();
    repo.save(&make_report(user_id.clone(), "r3"), test_ctx())
        .await
        .unwrap();

    let reports = repo.find_by_user(&user_id, 10).await.expect("find");
    assert_eq!(reports.len(), 3);
}
