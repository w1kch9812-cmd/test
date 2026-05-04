//! `PgListingReportRepository` 통합 테스트 — no OCC + transactional + anonymous
//! reporter (SP5-iii T8).
//!
//! 4 시나리오:
//! 1. `save` (INSERT) — `listing_report` + `audit_log` 1행 (`resource_kind = 'listing_report'`)
//! 2. anonymous reporter (`reporter_id` `None`) round-trip
//! 3. `find_open` — terminal (`Confirmed`) 제외, `Open` 만 잔존 검증
//! 4. `find_by_listing` — listing FK 로 모든 신고 조회 (terminal 포함)

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::listing_report::PgListingReportRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::ListingRepository;
use listing_report_domain::entity::ListingReport;
use listing_report_domain::reason::ListingReportReason;
use listing_report_domain::repository::ListingReportRepository;
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

/// `User` + `Listing` 시드 — `listing_report.listing_id` `FK` 충족.
async fn seed_listing_with_owner(
    pool: &sqlx::PgPool,
    zsub: &str,
    email: &str,
) -> (Id<UserMarker>, Id<ListingMarker>) {
    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::new(),
        zsub,
        Email::try_new(email).unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    let owner_id = owner.id.clone();
    user_repo.save(&owner, test_ctx()).await.unwrap();

    let listing_repo = PgListingRepository::new(pool.clone());
    let listing = Listing::try_new_draft(
        Id::new(),
        owner_id.clone(),
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(100_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(100.00).unwrap(),
        ListingTitle::try_new("report test").unwrap(),
        Description::try_new("").unwrap(),
        None,
        now,
    )
    .expect("listing");
    let listing_id = listing.id.clone();
    listing_repo.save(&listing, test_ctx()).await.unwrap();

    (owner_id, listing_id)
}

/// `handler_id` `FK` 용 admin 사용자 시드.
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
    repo.save(&admin, test_ctx()).await.unwrap();
    admin_id
}

#[tokio::test]
async fn save_inserts_report_audit_outbox_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let (reporter_id, listing_id) =
        seed_listing_with_owner(&pool, "zsub-rep-1", "rep1@example.com").await;
    let repo = PgListingReportRepository::new(pool.clone());

    let report = ListingReport::try_new(
        listing_id,
        Some(reporter_id.clone()),
        ListingReportReason::WrongPrice,
        Some("price way off".to_owned()),
        Utc::now(),
    )
    .expect("report");
    let ctx = MutationContext::new_user_action(reporter_id, "corr_01HXY8RRPT4F8S1L01", "create");
    repo.save(&report, ctx).await.expect("save");

    // listing_report row 1 개
    let report_count: (i64,) = sqlx::query_as("select count(*) from listing_report where id = $1")
        .bind(report.id.as_str())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(report_count.0, 1);

    // audit_log row 1 개 (resource_kind = 'listing_report')
    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'listing_report' \
         and resource_id = $1",
    )
    .bind(report.id.as_str())
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

    // round-trip 검증
    let fetched = repo.find_by_id(&report.id).await.unwrap().expect("present");
    assert_eq!(fetched.reason, ListingReportReason::WrongPrice);
    assert_eq!(fetched.detail.as_deref(), Some("price way off"));
    assert!(fetched.handler_id.is_none());
    assert!(fetched.resolved_at.is_none());
}

#[tokio::test]
async fn save_with_anonymous_reporter() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let (_, listing_id) = seed_listing_with_owner(&pool, "zsub-rep-2", "rep2@example.com").await;
    let repo = PgListingReportRepository::new(pool.clone());

    let report = ListingReport::try_new(
        listing_id,
        None, // 익명
        ListingReportReason::Spam,
        None,
        Utc::now(),
    )
    .expect("report");
    let ctx = MutationContext::new_system_action("corr_01HXY8RRPT4F8S1L02", "create");
    repo.save(&report, ctx).await.expect("save");

    // round-trip — reporter_id 가 NULL → None 으로 복원
    let fetched = repo.find_by_id(&report.id).await.unwrap().expect("present");
    assert!(fetched.reporter_id.is_none());
    assert_eq!(fetched.reason, ListingReportReason::Spam);
    assert!(fetched.detail.is_none());

    // audit_log 의 actor_id 도 NULL (시스템 액션)
    let actor: Option<String> = sqlx::query_scalar(
        "select actor_id from audit_log where resource_kind = 'listing_report' \
         and resource_id = $1",
    )
    .bind(report.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(actor.is_none());
}

#[tokio::test]
async fn find_open_excludes_terminal_status() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let (reporter_id, listing_id) =
        seed_listing_with_owner(&pool, "zsub-rep-3", "rep3@example.com").await;
    let admin_id = seed_admin(&pool, "zsub-rep-3-admin", "rep3admin@example.com").await;
    let repo = PgListingReportRepository::new(pool.clone());

    // 1) 두 신고 INSERT — 둘 다 Open
    let mut r1 = ListingReport::try_new(
        listing_id.clone(),
        Some(reporter_id.clone()),
        ListingReportReason::FakeListing,
        None,
        Utc::now(),
    )
    .expect("r1");
    let r2 = ListingReport::try_new(
        listing_id,
        Some(reporter_id.clone()),
        ListingReportReason::Other,
        None,
        Utc::now(),
    )
    .expect("r2");
    repo.save(
        &r1,
        MutationContext::new_user_action(reporter_id.clone(), "corr_01HXY8RRPT4F8S1L03", "create"),
    )
    .await
    .unwrap();
    repo.save(
        &r2,
        MutationContext::new_user_action(reporter_id.clone(), "corr_01HXY8RRPT4F8S1L04", "create"),
    )
    .await
    .unwrap();

    // 2) r1 → Confirmed (terminal). r2 는 Open 그대로.
    r1.mark_confirmed(admin_id, "verified".to_owned(), Utc::now())
        .expect("mark_confirmed");
    repo.save(
        &r1,
        MutationContext::new_user_action(reporter_id, "corr_01HXY8RRPT4F8S1L05", "confirm"),
    )
    .await
    .unwrap();

    // 3) find_open — Open + Investigating 만. r1 (Confirmed) 제외, r2 만 1건.
    let open = repo.find_open(10).await.unwrap();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].id.as_str(), r2.id.as_str());

    // 4) r1 round-trip — resolved_at 기록됨
    let fetched_r1 = repo.find_by_id(&r1.id).await.unwrap().expect("r1 present");
    assert!(fetched_r1.resolved_at.is_some());
    assert_eq!(fetched_r1.handler_note.as_deref(), Some("verified"));
}

#[tokio::test]
async fn find_by_listing_returns_all_reports() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let (reporter_id, listing_id) =
        seed_listing_with_owner(&pool, "zsub-rep-4", "rep4@example.com").await;
    let repo = PgListingReportRepository::new(pool.clone());

    // 3가지 사유로 신고 3건
    for reason in [
        ListingReportReason::WrongPrice,
        ListingReportReason::WrongLocation,
        ListingReportReason::InappropriateContent,
    ] {
        let report = ListingReport::try_new(
            listing_id.clone(),
            Some(reporter_id.clone()),
            reason,
            None,
            Utc::now(),
        )
        .expect("report");
        repo.save(
            &report,
            MutationContext::new_user_action(
                reporter_id.clone(),
                "corr_01HXY8RRPT4F8S1L06",
                "create",
            ),
        )
        .await
        .unwrap();
    }

    let all = repo.find_by_listing(&listing_id).await.unwrap();
    assert_eq!(all.len(), 3);
    // 모두 같은 listing_id
    for r in &all {
        assert_eq!(r.listing_id.as_str(), listing_id.as_str());
    }
}
