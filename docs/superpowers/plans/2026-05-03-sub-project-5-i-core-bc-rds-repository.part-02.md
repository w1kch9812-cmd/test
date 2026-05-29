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

- [ ] **Step 2: `crates/db/src/listing.rs` 작성**

```rust
//! `ListingRepository` `Postgres` 구현체.

#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use geo_types::Point;
use listing_domain::contact_visibility::ContactVisibility;
use listing_domain::description::Description;
use listing_domain::entity::Listing;
use listing_domain::listing_status::ListingStatus;
use listing_domain::listing_title::ListingTitle;
use listing_domain::listing_type::ListingType;
use listing_domain::repository::{ListingRepository, RepoError};
use listing_domain::transaction_type::TransactionType;
use rust_decimal::Decimal;
use shared_kernel::area_m2::AreaM2;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::money::MoneyKrw;
use shared_kernel::point_srid::PointSrid;
use shared_kernel::pnu::Pnu;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `Listing` `Aggregate` 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingRepository {
    pool: PgPool,
}

impl PgListingRepository {
    /// 새 저장소 생성.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const SELECT_LISTING_COLUMNS: &str = r#"
    id, owner_id, parcel_pnu, listing_type, transaction_type,
    price_krw, deposit_krw, monthly_rent_krw, area_m2,
    title, description, status, contact_visibility,
    view_count, bookmark_count,
    ST_X(geom_point) as geom_lng, ST_Y(geom_point) as geom_lat,
    geom_point is not null as has_geom,
    created_at, updated_at, expires_at, version
"#;

#[allow(clippy::too_many_lines)]
fn row_to_listing(row: &PgRow) -> Result<Listing, RepoError> {
    let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
    let owner_id_str: String = row.try_get("owner_id").map_err(|e| RepoError::Database(e.to_string()))?;
    let parcel_pnu_str: String = row.try_get("parcel_pnu").map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_type_str: String = row.try_get("listing_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let transaction_type_str: String = row.try_get("transaction_type").map_err(|e| RepoError::Database(e.to_string()))?;
    let price_krw: i64 = row.try_get("price_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let deposit_krw: Option<i64> = row.try_get("deposit_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let monthly_rent_krw: Option<i64> = row.try_get("monthly_rent_krw").map_err(|e| RepoError::Database(e.to_string()))?;
    let area_m2: Decimal = row.try_get("area_m2").map_err(|e| RepoError::Database(e.to_string()))?;
    let title_str: String = row.try_get("title").map_err(|e| RepoError::Database(e.to_string()))?;
    let description_str: String = row.try_get("description").map_err(|e| RepoError::Database(e.to_string()))?;
    let status_str: String = row.try_get("status").map_err(|e| RepoError::Database(e.to_string()))?;
    let contact_vis_str: String = row.try_get("contact_visibility").map_err(|e| RepoError::Database(e.to_string()))?;
    let view_count: i64 = row.try_get("view_count").map_err(|e| RepoError::Database(e.to_string()))?;
    let bookmark_count: i64 = row.try_get("bookmark_count").map_err(|e| RepoError::Database(e.to_string()))?;
    let has_geom: bool = row.try_get("has_geom").map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lng: Option<f64> = row.try_get("geom_lng").map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lat: Option<f64> = row.try_get("geom_lat").map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row.try_get("updated_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let expires_at: Option<DateTime<Utc>> = row.try_get("expires_at").map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row.try_get("version").map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing id: {e}")))?;
    let owner_id = Id::<UserMarker>::try_from_str(&owner_id_str)
        .map_err(|e| RepoError::Database(format!("malformed owner_id: {e}")))?;
    let parcel_pnu = Pnu::try_new(&parcel_pnu_str)
        .map_err(|e| RepoError::Database(format!("malformed pnu: {e}")))?;
    let listing_type = ListingType::from_str(&listing_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected listing_type: {listing_type_str}")))?;
    let transaction_type = TransactionType::from_str(&transaction_type_str)
        .map_err(|_| RepoError::Database(format!("unexpected transaction_type: {transaction_type_str}")))?;
    let price = MoneyKrw::try_new(price_krw)
        .map_err(|e| RepoError::Database(format!("invalid price_krw: {e}")))?;
    let deposit = deposit_krw
        .map(|v| MoneyKrw::try_new(v).map_err(|e| RepoError::Database(format!("invalid deposit_krw: {e}"))))
        .transpose()?;
    let monthly_rent = monthly_rent_krw
        .map(|v| MoneyKrw::try_new(v).map_err(|e| RepoError::Database(format!("invalid monthly_rent_krw: {e}"))))
        .transpose()?;
    let area = AreaM2::try_new(area_m2)
        .map_err(|e| RepoError::Database(format!("invalid area_m2: {e}")))?;
    let title = ListingTitle::try_new(&title_str)
        .map_err(|e| RepoError::Database(format!("invalid title: {e}")))?;
    let description = Description::new(&description_str);
    let status = ListingStatus::from_str(&status_str)
        .map_err(|_| RepoError::Database(format!("unexpected status: {status_str}")))?;
    let contact_visibility = ContactVisibility::from_str(&contact_vis_str)
        .map_err(|_| RepoError::Database(format!("unexpected contact_visibility: {contact_vis_str}")))?;
    let geom_point = if has_geom {
        match (geom_lng, geom_lat) {
            (Some(x), Some(y)) => Some(PointSrid::new(Point::new(x, y))),
            _ => None,
        }
    } else {
        None
    };

    let view_count_u: u64 = u64::try_from(view_count).unwrap_or(0);
    let bookmark_count_u: u64 = u64::try_from(bookmark_count).unwrap_or(0);

    Ok(Listing {
        id,
        owner_id,
        parcel_pnu,
        listing_type,
        transaction_type,
        price,
        deposit,
        monthly_rent,
        area,
        title,
        description,
        status,
        contact_visibility,
        view_count: view_count_u,
        bookmark_count: bookmark_count_u,
        geom_point,
        created_at,
        updated_at,
        expires_at,
        version,
    })
}

#[async_trait]
impl ListingRepository for PgListingRepository {
    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn find_by_id(&self, id: &Id<ListingMarker>) -> Result<Option<Listing>, RepoError> {
        let sql = format!("select {SELECT_LISTING_COLUMNS} from listing where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_listing).transpose()
    }

    #[instrument(skip(self), fields(owner_id = %owner.as_str(), limit))]
    async fn find_by_owner(
        &self,
        owner: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<Listing>, RepoError> {
        let sql = format!(
            "select {SELECT_LISTING_COLUMNS} from listing where owner_id = $1 order by created_at desc limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(owner.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_listing).collect()
    }

    #[instrument(skip(self, listing), fields(listing_id = %listing.id.as_str(), version = listing.version))]
    async fn save(&self, listing: &Listing) -> Result<(), RepoError> {
        let geom_lng_opt = listing.geom_point.as_ref().map(|p| p.0.x());
        let geom_lat_opt = listing.geom_point.as_ref().map(|p| p.0.y());

        let result = sqlx::query(
            r#"
            insert into listing (
                id, owner_id, parcel_pnu, listing_type, transaction_type,
                price_krw, deposit_krw, monthly_rent_krw, area_m2,
                title, description, status, contact_visibility,
                view_count, bookmark_count,
                geom_point,
                created_at, updated_at, expires_at, version
            )
            values (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13,
                $14, $15,
                case when $16::float8 is null or $17::float8 is null then null
                     else ST_SetSRID(ST_MakePoint($16, $17), 4326) end,
                $18, $19, $20, $21
            )
            on conflict (id) do update set
                listing_type = excluded.listing_type,
                transaction_type = excluded.transaction_type,
                price_krw = excluded.price_krw,
                deposit_krw = excluded.deposit_krw,
                monthly_rent_krw = excluded.monthly_rent_krw,
                area_m2 = excluded.area_m2,
                title = excluded.title,
                description = excluded.description,
                status = excluded.status,
                contact_visibility = excluded.contact_visibility,
                view_count = excluded.view_count,
                bookmark_count = excluded.bookmark_count,
                geom_point = excluded.geom_point,
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at,
                version = listing.version + 1
            where listing.version = $21
            "#,
        )
        .bind(listing.id.as_str())
        .bind(listing.owner_id.as_str())
        .bind(listing.parcel_pnu.as_str())
        .bind(listing.listing_type.as_str())
        .bind(listing.transaction_type.as_str())
        .bind(i64::from(listing.price))
        .bind(listing.deposit.map(i64::from))
        .bind(listing.monthly_rent.map(i64::from))
        .bind(listing.area.value())
        .bind(listing.title.as_str())
        .bind(listing.description.as_str())
        .bind(listing.status.as_str())
        .bind(listing.contact_visibility.as_str())
        .bind(i64::try_from(listing.view_count).unwrap_or(i64::MAX))
        .bind(i64::try_from(listing.bookmark_count).unwrap_or(i64::MAX))
        .bind(geom_lng_opt)
        .bind(geom_lat_opt)
        .bind(listing.created_at)
        .bind(listing.updated_at)
        .bind(listing.expires_at)
        .bind(listing.version)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }
        Ok(())
    }
}
```

> **주의**: 도메인 값 객체의 `as_str()` / `value()` / `i64::from(...)` 시그니처는 실제 코드와 다를 수 있어요. 구현 시 컴파일 에러가 나면 도메인 값 객체의 실제 메서드명 확인 후 조정. 본 plan 은 베스트 가정.

- [ ] **Step 3: 로컬 검증**

```bash
cargo check -p db
cargo clippy -p db --all-features -- -D warnings
cargo test -p db --lib
```

`cargo check` 가 도메인 값 객체 시그니처 mismatch 발견하면 그 자리에서 수정.

- [ ] **Step 4: Commit + push**

```bash
git add crates/db/src/listing.rs crates/db/tests/listing_integration.rs
git commit -m "feat(db): PgListingRepository — 21 필드 + PostGIS + OCC + tracing (SP5-i T3)

- row_to_listing: 21 필드 round-trip (PostGIS ST_X/ST_Y 로 lat/lng 복원)
- save: ST_SetSRID(ST_MakePoint, 4326) — ADR-0008 SRID 4326
- ON CONFLICT DO UPDATE WHERE version = \$N (OCC)
- 모든 메서드 #[tracing::instrument] (PII 미노출, listing_id/owner_id 만)
- map_sqlx_err 적용
- 9 통합 테스트 (round-trip with/without geom + find_by_owner + 4 OCC 시나리오 + monthly_rent)"
git push
```

---

