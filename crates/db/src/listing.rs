//! `ListingRepository` `Postgres` implementation.
//!
//! `area_m2` `numeric(12, 2)` is read as `BigDecimal` and converted to `f64`
//! at the SQL boundary.
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
use listing_domain::repository::{
    ListingDetail, ListingMarkerTile, ListingMarkerTileQuery, ListingParcelDenormalize,
    ListingPhotoSummary, ListingRepository, RepoError, LISTING_MARKER_TILE_LAYER,
};
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
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

/// All `listing` aggregate columns.
const LISTING_FULL_COLUMNS: &str = "id, owner_id, parcel_pnu, listing_type, transaction_type, \
    price_krw, deposit_krw, monthly_rent_krw, area_m2, \
    title, description, status, contact_visibility, \
    view_count, bookmark_count, \
    created_at, updated_at, expires_at, version";

const LISTING_FULL_COLUMNS_WITH_L_ALIAS: &str = "l.id, l.owner_id, l.parcel_pnu, l.listing_type, \
    l.transaction_type, l.price_krw, l.deposit_krw, l.monthly_rent_krw, l.area_m2, \
    l.title, l.description, l.status, l.contact_visibility, \
    l.view_count, l.bookmark_count, \
    l.created_at, l.updated_at, l.expires_at, l.version";

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
        created_at,
        updated_at,
        expires_at,
        version,
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

    #[allow(clippy::too_many_lines)]
    #[instrument(skip(self, query))]
    async fn find_card_summaries(
        &self,
        query: listing_domain::repository::CardSearchQuery,
    ) -> Result<(Vec<listing_domain::repository::ListingCardSummary>, u64), RepoError> {
        use listing_domain::repository::{CardSearchSort, ListingCardSummary};

        // listing_type / transaction_type 필터 (None or empty = 전체).
        let types_array: Option<Vec<&str>> = query
            .types
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| v.iter().map(|t| t.as_str()).collect());
        let txns_array: Option<Vec<&str>> = query
            .transactions
            .as_ref()
            .filter(|v| !v.is_empty())
            .map(|v| v.iter().map(|t| t.as_str()).collect());

        let min_area = query.min_area_m2.unwrap_or(0.0_f64);
        // Korea 의 가장 큰 산업단지도 200M m² 미만. 1e9 m² 캡으로 numeric overflow 안전.
        let max_area = query.max_area_m2.unwrap_or(1e9_f64);
        let min_price = query.min_price_krw.unwrap_or(0_i64);
        let max_price = query.max_price_krw.unwrap_or(i64::MAX);

        let order_by = match query.sort {
            CardSearchSort::CreatedAtDesc => "created_at DESC",
            CardSearchSort::PriceAsc => "price_krw ASC",
            CardSearchSort::PriceDesc => "price_krw DESC",
            CardSearchSort::AreaAsc => "area_m2 ASC",
            CardSearchSort::AreaDesc => "area_m2 DESC",
        };

        // handler 가 1..=100 보장. DB layer 는 trust caller.
        let size = query.size;
        let offset = i64::from(query.page) * i64::from(size);

        // PERF: COUNT(*) over filtered set runs on every paginated request.
        // For large `listing` tables (millions of rows) this can be slow.
        // SP6-ii 후속 (또는 SP7-i 의 monitoring) 에서 cached total / approximate count 검토.
        //
        // SP6-iii: bookmark_count 와 is_bookmarked 는 bookmark_listing 테이블 JOIN
        // (denormalized listing.bookmark_count 컬럼 미사용 — FU 70 schema 제거 예정).
        // ADR 0018 SP9 T4: pnu / admin_code_prefix / land_use_type 필터 추가.
        // 지도 marker placement 는 platform-core PNU-anchor PBF 경로가 담당하고,
        // listing cards 는 PNU/admin-code 기반 목록 조회만 담당한다.
        let pnu_filter: Option<&str> = query.pnu.as_ref().map(Pnu::as_str);
        let admin_prefix_filter: Option<&str> = query.admin_code_prefix.as_deref();
        let land_use_filter: Option<&str> = query.land_use_type.map(|t| {
            use shared_kernel::land_use_type::LandUseType;
            LandUseType::as_str(t)
        });

        let sql = format!(
            r"
            WITH filtered AS (
                SELECT id, title, listing_type, transaction_type,
                       price_krw, deposit_krw, monthly_rent_krw, area_m2,
                       view_count, created_at, owner_id
                FROM listing
                WHERE status = 'active'
                  AND ($1::text[] IS NULL OR listing_type = ANY($1::text[]))
                  AND ($2::text[] IS NULL OR transaction_type = ANY($2::text[]))
                  AND area_m2::float8 BETWEEN $3 AND $4
                  AND price_krw BETWEEN $5 AND $6
                  AND ($10::text IS NULL OR parcel_pnu = $10)
                  AND ($11::text IS NULL OR admin_code LIKE $11 || '%')
                  AND ($12::text IS NULL OR parcel_land_use_type = $12)
            ),
            bm_count AS (
                SELECT listing_id, COUNT(*)::int8 AS cnt
                FROM bookmark_listing
                WHERE listing_id IN (SELECT id FROM filtered)
                GROUP BY listing_id
            )
            SELECT
                (SELECT COUNT(*) FROM filtered) AS total_count,
                f.id, f.title,
                f.listing_type, f.transaction_type,
                f.price_krw, f.deposit_krw, f.monthly_rent_krw,
                f.area_m2::float8 AS area_m2,
                f.view_count,
                COALESCE(bc.cnt, 0)::int8 AS bookmark_count,
                CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END AS is_bookmarked,
                f.created_at
            FROM filtered f
            LEFT JOIN bm_count bc ON bc.listing_id = f.id
            LEFT JOIN bookmark_listing ub
              ON ub.listing_id = f.id AND ub.user_id = $9
            ORDER BY f.{order_by}
            LIMIT $7 OFFSET $8
            "
        );

        let rows = sqlx::query(&sql)
            .bind(types_array.as_deref())
            .bind(txns_array.as_deref())
            .bind(min_area)
            .bind(max_area)
            .bind(min_price)
            .bind(max_price)
            .bind(i64::from(size))
            .bind(offset)
            .bind(query.viewer_user_id.as_str())
            .bind(pnu_filter)
            .bind(admin_prefix_filter)
            .bind(land_use_filter)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut total_count: u64 = 0;
        let mut cards: Vec<ListingCardSummary> = Vec::with_capacity(rows.len());
        for row in &rows {
            let tc: i64 = row.try_get("total_count").unwrap_or(0_i64);
            total_count = u64::try_from(tc.max(0)).unwrap_or(0);

            let id_str: String = row
                .try_get("id")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let id = Id::<ListingIdMarker>::try_from_str(&id_str)
                .map_err(|e| RepoError::Database(format!("invalid listing id: {e}")))?;

            let title: String = row
                .try_get("title")
                .map_err(|e| RepoError::Database(e.to_string()))?;

            let lt_str: String = row
                .try_get("listing_type")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let listing_type = parse_listing_type(&lt_str)?;

            let tt_str: String = row
                .try_get("transaction_type")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let transaction_type = parse_transaction_type(&tt_str)?;

            let price_i: i64 = row
                .try_get("price_krw")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let price = MoneyKrw::try_new(price_i)
                .map_err(|e| RepoError::Database(format!("invalid price_krw in DB: {e}")))?;

            let deposit_opt: Option<i64> = row
                .try_get("deposit_krw")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let deposit = deposit_opt
                .map(|d| {
                    MoneyKrw::try_new(d)
                        .map_err(|e| RepoError::Database(format!("invalid deposit_krw in DB: {e}")))
                })
                .transpose()?;

            let rent_opt: Option<i64> = row
                .try_get("monthly_rent_krw")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let monthly_rent = rent_opt
                .map(|d| {
                    MoneyKrw::try_new(d).map_err(|e| {
                        RepoError::Database(format!("invalid monthly_rent_krw in DB: {e}"))
                    })
                })
                .transpose()?;

            let area_m2: f64 = row
                .try_get("area_m2")
                .map_err(|e| RepoError::Database(e.to_string()))?;
            let view_count: i64 = row.try_get("view_count").unwrap_or(0_i64);
            let bookmark_count: i64 = row.try_get("bookmark_count").unwrap_or(0_i64);
            let is_bookmarked: bool = row.try_get("is_bookmarked").unwrap_or(false);
            let created_at: chrono::DateTime<chrono::Utc> = row
                .try_get("created_at")
                .map_err(|e| RepoError::Database(e.to_string()))?;

            cards.push(ListingCardSummary {
                id,
                title,
                listing_type,
                transaction_type,
                price,
                deposit,
                monthly_rent,
                area_m2,
                thumbnail_url: None, // SP6-iii 가 listing-photo 테이블 join 으로 채움 (FU 별도)
                view_count,
                bookmark_count,
                is_bookmarked,
                created_at,
            });
        }

        Ok((cards, total_count))
    }

    #[instrument(skip(self), fields(
        z = query.z,
        x = query.x,
        y = query.y,
        filter_hash = query.filter.hash(),
    ))]
    async fn find_listing_marker_tile(
        &self,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError> {
        let row = sqlx::query(
            r"
            WITH unanchored_active AS (
                SELECT COUNT(*)::int8 AS count
                FROM listing l
                LEFT JOIN parcel_marker_anchor a ON a.pnu = l.parcel_pnu
                WHERE l.status = 'active'
                  AND a.pnu IS NULL
            ),
            eligible AS (
                SELECT
                    l.id,
                    l.parcel_pnu AS pnu,
                    a.anchor_point,
                    a.anchor_snapshot_id,
                    row_number() OVER (ORDER BY l.created_at DESC, l.id ASC)::int4 AS rank
                FROM listing l
                INNER JOIN parcel_marker_anchor a ON a.pnu = l.parcel_pnu
                WHERE l.status = 'active'
                  AND ST_Intersects(
                      ST_Transform(a.anchor_point, 3857),
                      ST_TileEnvelope($1, $2, $3)
                  )
            ),
            features AS (
                SELECT
                    id,
                    pnu,
                    'listing'::text AS kind,
                    1::int4 AS count,
                    rank,
                    id::text AS detail_ref,
                    anchor_snapshot_id,
                    ST_AsMVTGeom(
                        ST_Transform(anchor_point, 3857),
                        ST_TileEnvelope($1, $2, $3),
                        4096,
                        256,
                        true
                    ) AS geom
                FROM eligible
            ),
            represented AS (
                SELECT *
                FROM features
                WHERE geom IS NOT NULL
            ),
            tile AS (
                SELECT ST_AsMVT(represented, $4, 4096, 'geom') AS bytes
                FROM represented
            )
            SELECT
                COALESCE((SELECT bytes FROM tile), '\x'::bytea) AS bytes,
                (SELECT count FROM unanchored_active) AS unanchored_active_count,
                (SELECT COUNT(*)::int8 FROM eligible) AS eligible_count,
                (SELECT COUNT(*)::int8 FROM represented) AS represented_count,
                (SELECT COUNT(*)::int8 FROM represented) AS feature_count,
                0::int8 AS aggregate_count,
                (SELECT max(anchor_snapshot_id) FROM represented) AS anchor_snapshot_id
            ",
        )
        .bind(i32::from(query.z))
        .bind(i32::try_from(query.x).map_err(|e| RepoError::Database(e.to_string()))?)
        .bind(i32::try_from(query.y).map_err(|e| RepoError::Database(e.to_string()))?)
        .bind(LISTING_MARKER_TILE_LAYER)
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx_err)?;

        let unanchored_active_count: i64 = row
            .try_get("unanchored_active_count")
            .map_err(map_sqlx_err)?;
        if unanchored_active_count > 0 {
            return Err(RepoError::Database(format!(
                "listing marker tile completeness violation: unanchored_active_count={unanchored_active_count}"
            )));
        }

        let eligible_count: i64 = row.try_get("eligible_count").map_err(map_sqlx_err)?;
        let represented_count: i64 = row.try_get("represented_count").map_err(map_sqlx_err)?;
        if eligible_count != represented_count {
            return Err(RepoError::Database(format!(
                "listing marker tile completeness violation: eligible_count={eligible_count}, represented_count={represented_count}"
            )));
        }

        Ok(ListingMarkerTile {
            bytes: row.try_get("bytes").map_err(map_sqlx_err)?,
            layer_name: LISTING_MARKER_TILE_LAYER,
            eligible_count,
            represented_count,
            feature_count: row.try_get("feature_count").map_err(map_sqlx_err)?,
            aggregate_count: row.try_get("aggregate_count").map_err(map_sqlx_err)?,
            anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
        })
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

        let view_count_i64 = i64::try_from(listing.view_count).unwrap_or(i64::MAX);
        let bookmark_count_i64 = i64::try_from(listing.bookmark_count).unwrap_or(i64::MAX);

        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state snapshot.
        let before_state = crate::audit_state::read_listing_json(&mut tx, &listing.id).await?;

        // 1. listing UPSERT with OCC.
        let result = sqlx::query(
            r"
            insert into listing (
                id, owner_id, parcel_pnu, listing_type, transaction_type,
                price_krw, deposit_krw, monthly_rent_krw, area_m2,
                title, description, status, contact_visibility,
                view_count, bookmark_count,
                created_at, updated_at, expires_at, version
            )
            values (
                $1, $2, $3, $4, $5,
                $6, $7, $8, $9,
                $10, $11, $12, $13,
                $14, $15,
                $16, $17, $18, $19
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
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at,
                version = excluded.version
            where listing.version = $19 - 1
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

        // 2a. SP-Obs T4: after_state snapshot + metadata merge.
        let after_state_raw = crate::audit_state::read_listing_json(&mut tx, &listing.id).await?;
        let after_state =
            crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

        // 2b. audit_log INSERT — same tx.
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
            values ($1, $2, $3, 'listing', $4, $5, $6, $7::inet, $8, $9, $10)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(listing.id.as_str())
        .bind(&before_state)
        .bind(&after_state)
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

    /// 상세 페이지 — `Listing` + photos + JOIN COUNT bookmark + viewer is_bookmarked.
    /// 단일 connection 으로 listing+bookmark 와 photos 두 query 순차 실행.
    #[instrument(skip(self), fields(
        listing_id = %id.as_str(),
        viewer = %viewer_user_id.as_str(),
    ))]
    async fn find_detail_by_id(
        &self,
        id: &Id<ListingIdMarker>,
        viewer_user_id: &Id<UserMarker>,
    ) -> Result<Option<ListingDetail>, RepoError> {
        // 1. Listing + bookmark JOIN (단일 row).
        let detail_sql = format!(
            r"
            SELECT {LISTING_FULL_COLUMNS_WITH_L_ALIAS},
                   COALESCE(bm.cnt, 0)::int8 AS jc_bookmark_count,
                   CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END AS jc_is_bookmarked
            FROM listing l
            LEFT JOIN (
                SELECT listing_id, COUNT(*)::int8 AS cnt
                FROM bookmark_listing
                WHERE listing_id = $1
                GROUP BY listing_id
            ) bm ON bm.listing_id = l.id
            LEFT JOIN bookmark_listing ub
              ON ub.listing_id = l.id AND ub.user_id = $2
            WHERE l.id = $1
            "
        );
        let row_opt = sqlx::query(&detail_sql)
            .bind(id.as_str())
            .bind(viewer_user_id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        let Some(row) = row_opt else {
            return Ok(None);
        };
        let listing = row_to_listing(&row)?;
        let bookmark_count: i64 = row.try_get("jc_bookmark_count").unwrap_or(0_i64);
        let is_bookmarked: bool = row.try_get("jc_is_bookmarked").unwrap_or(false);

        // 2. photos (active 만, display_order ASC).
        let photo_rows = sqlx::query(
            r"
            SELECT r2_key, thumbnail_r2_key, caption, display_order, content_type
            FROM listing_photo
            WHERE listing_id = $1 AND deleted_at IS NULL
            ORDER BY display_order ASC
            ",
        )
        .bind(id.as_str())
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        let photos: Vec<ListingPhotoSummary> = photo_rows
            .iter()
            .map(|r| {
                Ok::<_, RepoError>(ListingPhotoSummary {
                    r2_key: r.try_get("r2_key").map_err(map_sqlx_err)?,
                    thumbnail_r2_key: r.try_get("thumbnail_r2_key").map_err(map_sqlx_err)?,
                    caption: r.try_get("caption").map_err(map_sqlx_err)?,
                    display_order: r.try_get("display_order").map_err(map_sqlx_err)?,
                    content_type: r.try_get("content_type").map_err(map_sqlx_err)?,
                })
            })
            .collect::<Result<_, _>>()?;

        Ok(Some(ListingDetail {
            listing,
            photos,
            bookmark_count,
            is_bookmarked,
        }))
    }

    /// `view_count` += 1. version bump X / audit_log X (빈도 분리).
    /// 매물 미존재 시 `RepoError::NotFound`.
    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn increment_view_count(&self, id: &Id<ListingIdMarker>) -> Result<(), RepoError> {
        let result = sqlx::query(
            r"
            UPDATE listing
            SET view_count = view_count + 1, updated_at = now()
            WHERE id = $1
            ",
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    #[instrument(skip(self), fields(listing_id = %id))]
    async fn update_parcel_denormalize(
        &self,
        id: &Id<ListingIdMarker>,
        denormalize: &ListingParcelDenormalize,
    ) -> Result<(), RepoError> {
        // version bump 안 함 — 캐시 동기화. parcel_lookup_at = now() 가 stale 검출용.
        // SP-Obs T4 audit 도 스킵 — 비즈니스 변경 아님.
        let result = sqlx::query(
            r"
            UPDATE listing
            SET admin_code = $2,
                parcel_land_use_type = $3,
                parcel_zoning = $4,
                parcel_lookup_at = now()
            WHERE id = $1
            ",
        )
        .bind(id.as_str())
        .bind(denormalize.admin_code.as_str())
        .bind(denormalize.land_use_type.as_str())
        .bind(denormalize.zoning.as_ref().map(|z| z.as_str()))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
