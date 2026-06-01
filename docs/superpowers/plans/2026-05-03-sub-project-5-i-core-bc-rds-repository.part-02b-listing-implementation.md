# Sub-project 5-i Core BC RDS Repository - Part 02B: Listing Implementation

Parent index: [Sub-project 5-i Core BC RDS Repository - Part 02](./2026-05-03-sub-project-5-i-core-bc-rds-repository.part-02.md).

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
