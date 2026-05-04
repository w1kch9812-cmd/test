//! `ListingRepository` `Postgres` 구현체 (spec § 5.1 — 21 필드 + `PostGIS` + `OCC`).
//!
//! `geom_point` 은 `PostGIS` `geometry(Point, 4326)`. SQL 경계에서 `ST_MakePoint`
//! / `ST_X` / `ST_Y` 로 lng/lat 쌍과 변환. `area_m2` `numeric(12, 2)` 는
//! `BigDecimal` 로 받아 `f64` 로 변환 (`AreaM2` 이 `f64` 기반).
//!
//! `find_markers_in_bbox` 는 `ListingMarker` projection 만 가져오는 lightweight
//! 쿼리 — 지도 렌더용. `status = 'active'` + `geom_point is not null` 필터 +
//! `ST_Within(geom, ST_MakeEnvelope(..., 4326))`.
//!
//! SP5-iv: `save` 가 트랜잭션 안에서 `listing` UPSERT + `audit_log` +
//! `outbox_event` 를 함께 기록 — `MutationContext` 패턴 (`PgAdminActionRepository`
//! 와 동일).

// `PgListingRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use std::str::FromStr;

use async_trait::async_trait;
use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use listing_domain::entity::Listing;
use listing_domain::repository::{ListingMarker, ListingRepository, RepoError};
use shared_kernel::area::AreaM2;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::geometry::PointSrid;
use shared_kernel::id::{
    AuditLogMarker, Id, ListingMarker as ListingIdMarker, OutboxEventMarker, UserMarker,
};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `Listing` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingRepository {
    pool: PgPool,
}

impl PgListingRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// 모든 `listing` 컬럼 + `PostGIS` 좌표 분해 (`ST_X`/`ST_Y`).
const LISTING_FULL_COLUMNS: &str = "id, owner_id, parcel_pnu, listing_type, transaction_type, \
    price_krw, deposit_krw, monthly_rent_krw, area_m2, \
    title, description, status, contact_visibility, \
    view_count, bookmark_count, \
    ST_X(geom_point) as geom_lng, ST_Y(geom_point) as geom_lat, \
    geom_point IS NOT NULL as has_geom, \
    created_at, updated_at, expires_at, version";

/// 지도 마커 projection — 필요한 필드만.
const LISTING_MARKER_COLUMNS: &str = "id, listing_type, transaction_type, price_krw, \
    ST_X(geom_point) as geom_lng, ST_Y(geom_point) as geom_lat";

/// `PgRow` 를 `Listing` 으로 변환해요. 21 필드 모두 round-trip.
#[allow(clippy::too_many_lines)]
fn row_to_listing(row: &PgRow) -> Result<Listing, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let owner_id_str: String = row
        .try_get("owner_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let parcel_pnu_str: String = row
        .try_get("parcel_pnu")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_type_str: String = row
        .try_get("listing_type")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let transaction_type_str: String = row
        .try_get("transaction_type")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let price_krw: i64 = row
        .try_get("price_krw")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let deposit_krw: Option<i64> = row
        .try_get("deposit_krw")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let monthly_rent_krw: Option<i64> = row
        .try_get("monthly_rent_krw")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let area_decimal: BigDecimal = row
        .try_get("area_m2")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let title_str: String = row
        .try_get("title")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let description_str: String = row
        .try_get("description")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let contact_vis_str: String = row
        .try_get("contact_visibility")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let view_count_i64: i64 = row
        .try_get("view_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let bookmark_count_i64: i64 = row
        .try_get("bookmark_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let has_geom: bool = row
        .try_get("has_geom")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lng: Option<f64> = row
        .try_get("geom_lng")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lat: Option<f64> = row
        .try_get("geom_lat")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let expires_at: Option<DateTime<Utc>> = row
        .try_get("expires_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingIdMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing id in DB: {e}")))?;
    let owner_id = Id::<UserMarker>::try_from_str(&owner_id_str)
        .map_err(|e| RepoError::Database(format!("malformed owner_id in DB: {e}")))?;
    let parcel_pnu = Pnu::try_new(&parcel_pnu_str)
        .map_err(|e| RepoError::Database(format!("malformed pnu in DB: {e}")))?;
    let listing_type = parse_listing_type(&listing_type_str)?;
    let transaction_type = parse_transaction_type(&transaction_type_str)?;
    let price = MoneyKrw::try_new(price_krw)
        .map_err(|e| RepoError::Database(format!("invalid price_krw in DB: {e}")))?;
    let deposit = deposit_krw
        .map(|v| {
            MoneyKrw::try_new(v)
                .map_err(|e| RepoError::Database(format!("invalid deposit_krw in DB: {e}")))
        })
        .transpose()?;
    let monthly_rent = monthly_rent_krw
        .map(|v| {
            MoneyKrw::try_new(v)
                .map_err(|e| RepoError::Database(format!("invalid monthly_rent_krw in DB: {e}")))
        })
        .transpose()?;
    let area_f64 = area_decimal
        .to_f64()
        .ok_or_else(|| RepoError::Database("area_m2 BigDecimal -> f64 conversion failed".into()))?;
    let area = AreaM2::try_new(area_f64)
        .map_err(|e| RepoError::Database(format!("invalid area_m2 in DB: {e}")))?;
    let title = ListingTitle::try_new(&title_str)
        .map_err(|e| RepoError::Database(format!("invalid title in DB: {e}")))?;
    let description = Description::try_new(&description_str)
        .map_err(|e| RepoError::Database(format!("invalid description in DB: {e}")))?;
    let status = parse_listing_status(&status_str)?;
    let contact_visibility = parse_contact_visibility(&contact_vis_str)?;
    let geom_point = if has_geom {
        match (geom_lng, geom_lat) {
            (Some(x), Some(y)) => Some(
                PointSrid::try_new_wgs84(x, y)
                    .map_err(|e| RepoError::Database(format!("invalid geom in DB: {e}")))?,
            ),
            _ => None,
        }
    } else {
        None
    };

    let view_count = u64::try_from(view_count_i64).unwrap_or(0);
    let bookmark_count = u64::try_from(bookmark_count_i64).unwrap_or(0);

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
        view_count,
        bookmark_count,
        geom_point,
        created_at,
        updated_at,
        expires_at,
        version,
    })
}

/// `PgRow` 를 `ListingMarker` projection 으로 변환해요.
fn row_to_marker(row: &PgRow) -> Result<ListingMarker, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_type_str: String = row
        .try_get("listing_type")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let transaction_type_str: String = row
        .try_get("transaction_type")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let price_krw: i64 = row
        .try_get("price_krw")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lng: f64 = row
        .try_get("geom_lng")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let geom_lat: f64 = row
        .try_get("geom_lat")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingIdMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing id in DB: {e}")))?;
    let geom = PointSrid::try_new_wgs84(geom_lng, geom_lat)
        .map_err(|e| RepoError::Database(format!("invalid geom in DB: {e}")))?;
    let price = MoneyKrw::try_new(price_krw)
        .map_err(|e| RepoError::Database(format!("invalid price_krw in DB: {e}")))?;
    let listing_type = parse_listing_type(&listing_type_str)?;
    let transaction_type = parse_transaction_type(&transaction_type_str)?;

    Ok(ListingMarker {
        id,
        geom,
        price,
        listing_type,
        transaction_type,
    })
}

fn parse_listing_type(s: &str) -> Result<ListingType, RepoError> {
    match s {
        "factory" => Ok(ListingType::Factory),
        "warehouse" => Ok(ListingType::Warehouse),
        "office" => Ok(ListingType::Office),
        "knowledge_industry_center" => Ok(ListingType::KnowledgeIndustryCenter),
        "industrial_land" => Ok(ListingType::IndustrialLand),
        "logistics_center" => Ok(ListingType::LogisticsCenter),
        other => Err(RepoError::Database(format!(
            "unexpected listing_type in DB: {other}"
        ))),
    }
}

fn parse_transaction_type(s: &str) -> Result<TransactionType, RepoError> {
    match s {
        "sale" => Ok(TransactionType::Sale),
        "monthly_rent" => Ok(TransactionType::MonthlyRent),
        "jeonse" => Ok(TransactionType::Jeonse),
        other => Err(RepoError::Database(format!(
            "unexpected transaction_type in DB: {other}"
        ))),
    }
}

fn parse_listing_status(s: &str) -> Result<ListingStatus, RepoError> {
    match s {
        "draft" => Ok(ListingStatus::Draft),
        "pending_review" => Ok(ListingStatus::PendingReview),
        "active" => Ok(ListingStatus::Active),
        "sold" => Ok(ListingStatus::Sold),
        "expired" => Ok(ListingStatus::Expired),
        "rejected" => Ok(ListingStatus::Rejected),
        other => Err(RepoError::Database(format!(
            "unexpected status in DB: {other}"
        ))),
    }
}

fn parse_contact_visibility(s: &str) -> Result<ContactVisibility, RepoError> {
    match s {
        "public" => Ok(ContactVisibility::Public),
        "login_required" => Ok(ContactVisibility::LoginRequired),
        "verified_only" => Ok(ContactVisibility::VerifiedOnly),
        other => Err(RepoError::Database(format!(
            "unexpected contact_visibility in DB: {other}"
        ))),
    }
}

#[async_trait]
impl ListingRepository for PgListingRepository {
    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn find(&self, id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError> {
        let sql = format!("select {LISTING_FULL_COLUMNS} from listing where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_listing).transpose()
    }

    #[instrument(skip(self, bbox), fields(min_lng = bbox.min_lng, min_lat = bbox.min_lat, max_lng = bbox.max_lng, max_lat = bbox.max_lat))]
    async fn find_markers_in_bbox(
        &self,
        bbox: BoundingBox,
    ) -> Result<Vec<ListingMarker>, RepoError> {
        let sql = format!(
            "select {LISTING_MARKER_COLUMNS} from listing \
             where status = 'active' \
               and geom_point is not null \
               and ST_Within(geom_point, ST_MakeEnvelope($1, $2, $3, $4, 4326))"
        );
        let rows = sqlx::query(&sql)
            .bind(bbox.min_lng)
            .bind(bbox.min_lat)
            .bind(bbox.max_lng)
            .bind(bbox.max_lat)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_marker).collect()
    }

    #[instrument(skip(self), fields(owner_id = %owner_id.as_str()))]
    async fn find_by_owner(
        &self,
        owner_id: &Id<UserMarker>,
        status: Option<ListingStatus>,
    ) -> Result<Vec<Listing>, RepoError> {
        let rows = if let Some(s) = status {
            let sql = format!(
                "select {LISTING_FULL_COLUMNS} from listing \
                 where owner_id = $1 and status = $2 order by created_at desc"
            );
            sqlx::query(&sql)
                .bind(owner_id.as_str())
                .bind(s.as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_err)?
        } else {
            let sql = format!(
                "select {LISTING_FULL_COLUMNS} from listing \
                 where owner_id = $1 order by created_at desc"
            );
            sqlx::query(&sql)
                .bind(owner_id.as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_err)?
        };
        rows.iter().map(row_to_listing).collect()
    }

    /// 트랜잭션 안에서 `listing` UPSERT + `audit_log` + `outbox_event` 를 함께 기록.
    ///
    /// SP5-iv 패턴: 1) tx begin → 2) listing UPSERT (OCC) → 3) audit_log INSERT
    /// (`resource_kind = 'listing'`) → 4) `ctx.events` 마다 outbox INSERT
    /// (`aggregate_kind = 'listing'`) → 5) commit. 어느 단계 실패든 자동 rollback.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, listing, ctx), fields(
        listing_id = %listing.id.as_str(),
        version = listing.version,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError> {
        // numeric(12, 2) — `f64` 를 2 decimal 문자열 → `BigDecimal` 변환.
        let area_str = format!("{:.2}", listing.area.as_f64());
        let area_decimal = BigDecimal::from_str(&area_str)
            .map_err(|e| RepoError::Database(format!("invalid area_m2 conversion: {e}")))?;

        let geom_lng_opt = listing.geom_point.as_ref().map(|p| p.lng);
        let geom_lat_opt = listing.geom_point.as_ref().map(|p| p.lat);

        let view_count_i64 = i64::try_from(listing.view_count).unwrap_or(i64::MAX);
        let bookmark_count_i64 = i64::try_from(listing.bookmark_count).unwrap_or(i64::MAX);

        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. listing UPSERT with OCC.
        let result = sqlx::query(
            r"
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
            ",
        )
        .bind(listing.id.as_str())
        .bind(listing.owner_id.as_str())
        .bind(listing.parcel_pnu.as_str())
        .bind(listing.listing_type.as_str())
        .bind(listing.transaction_type.as_str())
        .bind(listing.price.as_i64())
        .bind(listing.deposit.map(MoneyKrw::as_i64))
        .bind(listing.monthly_rent.map(MoneyKrw::as_i64))
        .bind(&area_decimal)
        .bind(listing.title.as_str())
        .bind(listing.description.as_str())
        .bind(listing.status.as_str())
        .bind(listing.contact_visibility.as_str())
        .bind(view_count_i64)
        .bind(bookmark_count_i64)
        .bind(geom_lng_opt)
        .bind(geom_lat_opt)
        .bind(listing.created_at)
        .bind(listing.updated_at)
        .bind(listing.expires_at)
        .bind(listing.version)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            // ON CONFLICT DO UPDATE WHERE version 미일치 → 갱신 0건 → Conflict (tx Drop → rollback).
            return Err(RepoError::Conflict);
        }

        // 2. audit_log INSERT — same tx.
        let audit_id = Id::<AuditLogMarker>::new();
        let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state,
                ip_address, user_agent,
                correlation_id, created_at
            )
            values ($1, $2, $3, 'listing', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(listing.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. outbox_event INSERT for each ctx.events — same tx.
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'listing', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(listing.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // 4. commit.
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}
