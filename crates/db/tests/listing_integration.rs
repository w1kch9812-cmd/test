//! `PgListingRepository` 통합 테스트 — 21 필드 round-trip, `PostGIS`, `OCC`,
//! `ListingMarker` projection, SP5-iv transactional `audit_log` /
//! `outbox_event` 검증.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
#![cfg(feature = "integration")]

mod common;

use std::sync::Arc;

use chrono::{DateTime, Utc};
use db::listing::PgListingRepository;
use db::user::PgUserRepository;
use listing_domain::entity::Listing;
use listing_domain::repository::{ListingRepository, RepoError};
use shared_kernel::area::AreaM2;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::description::Description;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::email::Email;
use shared_kernel::geometry::PointSrid;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, test_ctx, truncate_all};

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

async fn seed_owner(pool: &sqlx::PgPool, zsub: &str, email: &str) -> Id<UserMarker> {
    let repo = PgUserRepository::new(pool.clone());
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
    repo.save(&owner, test_ctx()).await.unwrap();
    owner_id
}

fn make_listing_sale(owner_id: Id<UserMarker>) -> Listing {
    let now = Utc::now();
    Listing::try_new_draft(
        Id::new(),
        owner_id,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(500_000_000).unwrap(),
        None,
        None,
        AreaM2::try_new(330.58).unwrap(),
        ListingTitle::try_new("강남 공장 매물 (테스트)").unwrap(),
        Description::try_new("샘플 설명").unwrap(),
        Some(PointSrid::try_new_wgs84(127.0276, 37.4979).unwrap()),
        now,
    )
    .expect("listing")
}

#[tokio::test]
async fn round_trip_listing_with_postgis() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-1", "owner1@example.com").await;
    let repo = PgListingRepository::new(pool);

    let listing = make_listing_sale(owner);
    repo.save(&listing, test_ctx()).await.expect("save");

    let fetched = repo.find(&listing.id).await.expect("find").expect("Some");
    assert_eq!(fetched.id.as_str(), listing.id.as_str());
    assert_eq!(fetched.owner_id.as_str(), listing.owner_id.as_str());
    assert_eq!(fetched.listing_type, ListingType::Factory);
    assert_eq!(fetched.transaction_type, TransactionType::Sale);
    assert_eq!(fetched.status, ListingStatus::Draft);
    assert_eq!(fetched.view_count, 0);
    assert_eq!(fetched.bookmark_count, 0);
    assert_eq!(fetched.version, 1);
    assert_eq!(fetched.title.as_str(), listing.title.as_str());
    assert_eq!(fetched.description.as_str(), listing.description.as_str());
    assert!((fetched.area.as_f64() - listing.area.as_f64()).abs() < 0.01);

    let p = fetched.geom_point.expect("geom present");
    assert!((p.lng - 127.0276).abs() < 1e-9);
    assert!((p.lat - 37.4979).abs() < 1e-9);
}

#[tokio::test]
async fn save_without_geom_point() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-2", "owner2@example.com").await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    listing.geom_point = None;
    repo.save(&listing, test_ctx()).await.expect("save");
    let fetched = repo.find(&listing.id).await.unwrap().unwrap();
    assert!(fetched.geom_point.is_none());
}

#[tokio::test]
async fn find_by_owner_returns_owner_listings() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-3", "owner3@example.com").await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    let l2 = make_listing_sale(owner.clone());
    repo.save(&l1, test_ctx()).await.unwrap();
    repo.save(&l2, test_ctx()).await.unwrap();

    let results = repo.find_by_owner(&owner, None).await.expect("ok");
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn find_by_owner_with_status_filter() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-4", "owner4@example.com").await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    repo.save(&l1, test_ctx()).await.unwrap();

    // Draft 만 있음.
    let drafts = repo
        .find_by_owner(&owner, Some(ListingStatus::Draft))
        .await
        .unwrap();
    assert_eq!(drafts.len(), 1);

    let actives = repo
        .find_by_owner(&owner, Some(ListingStatus::Active))
        .await
        .unwrap();
    assert_eq!(actives.len(), 0);
}

#[tokio::test]
async fn find_nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingRepository::new(pool);
    let id = Id::new();
    let fetched = repo.find(&id).await.expect("find");
    assert!(fetched.is_none());
}

#[tokio::test]
async fn occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-5", "owner5@example.com").await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing, test_ctx()).await.unwrap();
    listing.version = 99;
    let err = repo.save(&listing, test_ctx()).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn update_bumps_version() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-6", "owner6@example.com").await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing, test_ctx()).await.unwrap();

    listing.view_count = 5;
    listing.version = 1;
    repo.save(&listing, test_ctx()).await.unwrap();

    let fetched = repo.find(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.version, 2);
    assert_eq!(fetched.view_count, 5);
}

#[tokio::test]
async fn save_monthly_rent_with_deposit_and_rent() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-7", "owner7@example.com").await;
    let repo = PgListingRepository::new(pool);

    let now = Utc::now();
    let listing = Listing::try_new_draft(
        Id::new(),
        owner,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Office,
        TransactionType::MonthlyRent,
        MoneyKrw::try_new(1_000_000).unwrap(),
        Some(MoneyKrw::try_new(50_000_000).unwrap()),
        Some(MoneyKrw::try_new(2_000_000).unwrap()),
        AreaM2::try_new(50.00).unwrap(),
        ListingTitle::try_new("월세 사무실").unwrap(),
        Description::try_new("").unwrap(),
        None,
        now,
    )
    .expect("listing");

    repo.save(&listing, test_ctx()).await.expect("save");
    let fetched = repo.find(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.deposit, listing.deposit);
    assert_eq!(fetched.monthly_rent, listing.monthly_rent);
    assert_eq!(fetched.transaction_type, TransactionType::MonthlyRent);
}

#[tokio::test]
async fn find_markers_in_bbox_returns_active_only() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-8", "owner8@example.com").await;
    let repo = PgListingRepository::new(pool.clone());

    // Draft 매물 — bbox 안 — markers 에 안 잡힘 (status='active' 필터).
    let l1 = make_listing_sale(owner.clone());
    repo.save(&l1, test_ctx()).await.unwrap();

    let bbox = BoundingBox::try_new_wgs84(126.9, 37.4, 127.1, 37.6).unwrap();
    let markers = repo.find_markers_in_bbox(bbox).await.expect("ok");
    assert_eq!(markers.len(), 0); // Draft 라 active 필터 통과 안 함.

    // 직접 SQL 로 status 'active' 변경.
    sqlx::query("update listing set status = 'active' where id = $1")
        .bind(l1.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let markers = repo.find_markers_in_bbox(bbox).await.expect("ok");
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].id.as_str(), l1.id.as_str());
}

// ---- SP5-iv: transactional audit_log + outbox_event 검증 ----

#[tokio::test]
async fn save_inserts_listing_audit_log_in_one_tx() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-aud-1", "audl1@example.com").await;
    let repo = PgListingRepository::new(pool.clone());

    let listing = make_listing_sale(owner.clone());
    let ctx = MutationContext::new_user_action(owner, "corr-listing-aud-1", "create_listing");
    repo.save(&listing, ctx).await.expect("save");

    let audit_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log where resource_kind = 'listing' and resource_id = $1",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count.0, 1);

    let outbox_count: (i64,) =
        sqlx::query_as("select count(*) from outbox_event where aggregate_kind = 'listing'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(outbox_count.0, 0);
}

#[tokio::test]
async fn save_listing_with_events_inserts_outbox_per_event() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-evt-1", "evtl1@example.com").await;
    let repo = PgListingRepository::new(pool.clone());

    let listing = make_listing_sale(owner.clone());
    let event1: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "listing.draft_created",
        aggregate_id: listing.id.as_str().to_owned(),
        payload: serde_json::json!({"price_krw": listing.price.as_i64()}),
        occurred_at: Utc::now(),
    });
    let event2: Arc<dyn DomainEvent> = Arc::new(TestEvent {
        event_type: "listing.indexer_queued",
        aggregate_id: listing.id.as_str().to_owned(),
        payload: serde_json::json!({}),
        occurred_at: Utc::now(),
    });
    let ctx = MutationContext::new_user_action(owner, "corr-listing-evt-1", "create_listing")
        .with_events(vec![event1, event2]);
    repo.save(&listing, ctx).await.expect("save");

    let outbox_count: (i64,) = sqlx::query_as(
        "select count(*) from outbox_event \
         where aggregate_kind = 'listing' and aggregate_id = $1",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(outbox_count.0, 2);
}

#[tokio::test]
async fn save_listing_system_action_records_null_actor() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool, "zsub-listing-sys-1", "sysl1@example.com").await;
    let repo = PgListingRepository::new(pool.clone());

    let listing = make_listing_sale(owner);
    let ctx = MutationContext::new_system_action("corr-listing-sys-1", "import");
    repo.save(&listing, ctx).await.expect("save");

    let null_actor_count: (i64,) = sqlx::query_as(
        "select count(*) from audit_log \
         where resource_kind = 'listing' and resource_id = $1 and actor_id is null",
    )
    .bind(listing.id.as_str())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(null_actor_count.0, 1);
}
