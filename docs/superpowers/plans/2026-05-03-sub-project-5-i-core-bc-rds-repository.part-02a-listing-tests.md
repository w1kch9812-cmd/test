# Sub-project 5-i Core BC RDS Repository - Part 02A: Listing Tests

Parent index: [Sub-project 5-i Core BC RDS Repository - Part 02](./2026-05-03-sub-project-5-i-core-bc-rds-repository.part-02.md).
### Task 3: `PgListingRepository` (PostGIS + OCC + soft-delete)

**Files:**
- Modify: `crates/db/src/listing.rs` (stub → full impl)
- Create: `crates/db/tests/listing_integration.rs`

- [ ] **Step 1: 통합 테스트 작성 (`crates/db/tests/listing_integration.rs`)**

```rust
//! `PgListingRepository` 통합 테스트 — 21 필드 round-trip + PostGIS + OCC + soft-delete.

#![allow(clippy::expect_used, clippy::unwrap_used)]
#![cfg(feature = "integration")]

mod common;

use chrono::Utc;
use db::listing::PgListingRepository;
use db::user::PgUserRepository;
use geo_types::Point;
use listing_domain::contact_visibility::ContactVisibility;
use listing_domain::description::Description;
use listing_domain::entity::Listing;
use listing_domain::listing_status::ListingStatus;
use listing_domain::listing_title::ListingTitle;
use listing_domain::listing_type::ListingType;
use listing_domain::repository::{ListingRepository, RepoError};
use listing_domain::transaction_type::TransactionType;
use shared_kernel::area_m2::AreaM2;
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::money::MoneyKrw;
use shared_kernel::point_srid::PointSrid;
use shared_kernel::pnu::Pnu;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use common::{setup_test_pool, truncate_all};

async fn seed_owner(pool: &sqlx::PgPool) -> Id<UserMarker> {
    let user_repo = PgUserRepository::new(pool.clone());
    let now = Utc::now();
    let owner = User::try_new(
        Id::new(),
        "owner-zsub",
        Email::try_new("owner@example.com").unwrap(),
        "Owner",
        UserKind::Individual,
        now,
    )
    .unwrap();
    user_repo.save(&owner).await.unwrap();
    owner.id
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
        None, // deposit
        None, // monthly_rent
        AreaM2::try_new(rust_decimal::Decimal::new(33058, 2)).unwrap(),
        ListingTitle::try_new("강남 공장 매물 (테스트)").unwrap(),
        Description::new("샘플 설명"),
        Some(PointSrid::new(Point::new(127.0276, 37.4979))), // 강남
        now,
    )
    .expect("listing")
}

#[tokio::test]
async fn round_trip_listing_with_postgis() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let listing = make_listing_sale(owner);
    repo.save(&listing).await.expect("save");

    let fetched = repo.find_by_id(&listing.id).await.expect("find").expect("Some");
    assert_eq!(fetched.id, listing.id);
    assert_eq!(fetched.owner_id, listing.owner_id);
    assert_eq!(fetched.parcel_pnu, listing.parcel_pnu);
    assert_eq!(fetched.listing_type, listing.listing_type);
    assert_eq!(fetched.transaction_type, listing.transaction_type);
    assert_eq!(fetched.price, listing.price);
    assert_eq!(fetched.title, listing.title);
    assert_eq!(fetched.status, ListingStatus::Draft);
    assert_eq!(fetched.contact_visibility, ContactVisibility::LoginRequired);
    assert_eq!(fetched.view_count, 0);
    assert_eq!(fetched.bookmark_count, 0);
    assert_eq!(fetched.version, 1);
    // PostGIS 정확 round-trip (lat/lng float)
    let p = fetched.geom_point.expect("geom present");
    assert!((p.0.x() - 127.0276).abs() < 1e-9);
    assert!((p.0.y() - 37.4979).abs() < 1e-9);
}

#[tokio::test]
async fn save_without_geom_point() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    listing.geom_point = None;
    repo.save(&listing).await.expect("save");

    let fetched = repo.find_by_id(&listing.id).await.expect("find").expect("Some");
    assert!(fetched.geom_point.is_none());
}

#[tokio::test]
async fn find_by_owner_returns_owner_listings() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    let l2 = make_listing_sale(owner.clone());
    repo.save(&l1).await.unwrap();
    repo.save(&l2).await.unwrap();

    let results = repo.find_by_owner(&owner, 10).await.expect("find_by_owner");
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn find_nonexistent_returns_none() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let repo = PgListingRepository::new(pool);
    let id = Id::new();
    let fetched = repo.find_by_id(&id).await.expect("find");
    assert!(fetched.is_none());
}

#[tokio::test]
async fn occ_version_mismatch_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    listing.version = 99;
    let err = repo.save(&listing).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn duplicate_id_returns_conflict() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let l1 = make_listing_sale(owner.clone());
    let mut l2 = make_listing_sale(owner);
    l2.id = l1.id.clone();
    l2.version = 1; // 같은 version 으로 INSERT 시도 → unique violation but version match 분기 동작 — 실은 ON CONFLICT DO UPDATE 이라 update 됨
    // 다른 owner 로 변경했다면 업데이트 통과 — 이 시나리오는 정확히 의도된 흐름

    repo.save(&l1).await.unwrap();
    repo.save(&l2).await.unwrap(); // 같은 id, version=1 → upsert success

    // 이번엔 진짜 duplicate ID 다른 데이터: version 안 맞춰 conflict
    let mut l3 = make_listing_sale(l1.owner_id.clone());
    l3.id = l1.id.clone();
    l3.version = 99;
    let err = repo.save(&l3).await.unwrap_err();
    assert!(matches!(err, RepoError::Conflict));
}

#[tokio::test]
async fn update_changes_version_and_fields() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let mut listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    let fetched = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.version, 1);

    // 도메인 메서드로 update — view_count 증가
    listing.view_count = 5;
    listing.version = 1; // OCC: 현재 DB 버전
    repo.save(&listing).await.unwrap();

    let fetched2 = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched2.version, 2);
    assert_eq!(fetched2.view_count, 5);
}

#[tokio::test]
async fn soft_deleted_listing_excluded_from_find() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool.clone());

    let listing = make_listing_sale(owner);
    repo.save(&listing).await.unwrap();

    // 직접 SQL 로 soft-delete (도메인 메서드는 SP5-i 범위 외 — 도메인이 deleted_at 컬럼 모름)
    sqlx::query(r#"update listing set deleted_at = now() where id = $1"#)
        .bind(listing.id.as_str())
        .execute(&pool)
        .await
        .unwrap();

    let fetched = repo.find_by_id(&listing.id).await.unwrap();
    // 현재 Listing entity 는 deleted_at 컬럼 없음 — find_by_id 가 별도로 필터링.
    // V001_01 listing 테이블에 deleted_at 없음을 확인했으면 본 테스트 skip.
    // (확인: V001_01 listing 테이블은 deleted_at 미포함 — User 만 soft-delete)
    // 따라서 본 테스트는 의도 다른 시나리오로 변경 필요.
    let _ = fetched; // listing 은 soft-delete 미지원 → 테스트 의미 없음
}
```

> **주의**: V001_01 의 `listing` 테이블에는 `deleted_at` 컬럼이 *없어요* (User 만 soft-delete). 위 마지막 테스트는 의미 없으므로 **삭제 또는 다른 시나리오로 대체**. 구현 단계에서 확인 후 결정. 본 plan 은 8 tests 로 계산.

이 테스트는 삭제하고 다음으로 대체:

```rust
#[tokio::test]
async fn save_with_deposit_and_monthly_rent_for_monthly_rent_type() {
    let pool = setup_test_pool().await;
    truncate_all(&pool).await;
    let owner = seed_owner(&pool).await;
    let repo = PgListingRepository::new(pool);

    let now = Utc::now();
    let listing = Listing::try_new_draft(
        Id::new(),
        owner,
        Pnu::try_new("1111010100100070000").unwrap(),
        ListingType::Office,
        TransactionType::MonthlyRent,
        MoneyKrw::try_new(1_000_000).unwrap(), // price not used much for monthly_rent
        Some(MoneyKrw::try_new(50_000_000).unwrap()), // deposit
        Some(MoneyKrw::try_new(2_000_000).unwrap()),  // monthly_rent
        AreaM2::try_new(rust_decimal::Decimal::new(5000, 2)).unwrap(),
        ListingTitle::try_new("월세 사무실").unwrap(),
        Description::new(""),
        None,
        now,
    )
    .expect("listing");

    repo.save(&listing).await.expect("save");
    let fetched = repo.find_by_id(&listing.id).await.unwrap().unwrap();
    assert_eq!(fetched.deposit, listing.deposit);
    assert_eq!(fetched.monthly_rent, listing.monthly_rent);
    assert_eq!(fetched.transaction_type, TransactionType::MonthlyRent);
}
```

총 9 tests.
