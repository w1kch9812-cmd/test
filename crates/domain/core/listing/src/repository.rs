//! `ListingRepository` port (interface) + 지도 마커 projection.
//!
//! 구현체는 sub-project 5 (`crates/db`)에서 추가.

// `ListingRepository`/`ListingMarker` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::geometry::PointSrid;
// `shared_kernel::id::ListingMarker`는 `Id<_>`용 phantom marker.
// 이 모듈의 `ListingMarker` projection과 이름이 겹치므로 `ListingIdMarker`로 별명 부여.
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::transaction_type::TransactionType;
use thiserror::Error;

use crate::entity::Listing;

/// `Listing` 저장/조회 포트.
#[async_trait]
pub trait ListingRepository: Send + Sync {
    /// `id`로 조회. 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find(&self, id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError>;

    /// 지도 마커용 lightweight projection.
    ///
    /// `bbox` 안에 `geom_point`이 있는 `Active` 매물만 반환해요.
    /// 전체 [`Listing`] 대신 필요한 필드만 가져와 지도 렌더 성능을 최적화.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_markers_in_bbox(
        &self,
        bbox: BoundingBox,
    ) -> Result<Vec<ListingMarker>, RepoError>;

    /// 카드 list 검색 — `status='active'` + `geom_point` not null + filter 적용.
    ///
    /// `query.size` 의 max 100 (caller 가 검증). 응답은 (cards, total\_count) 튜플.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_card_summaries_in_bbox(
        &self,
        query: CardSearchQuery,
    ) -> Result<(Vec<ListingCardSummary>, u64), RepoError>;

    /// 소유자별 매물 조회. `status`가 `Some`이면 해당 상태만.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_owner(
        &self,
        owner_id: &Id<UserMarker>,
        status: Option<ListingStatus>,
    ) -> Result<Vec<Listing>, RepoError>;

    /// 저장 (insert or update). Optimistic lock 충돌 시 [`RepoError::Conflict`].
    ///
    /// `ctx` 의 `actor_id` / `action` / `metadata` / `events` 가 같은 트랜잭션
    /// 안에서 `audit_log` 와 `outbox_event` 로 자동 기록돼요 (SP5-iv 의
    /// transactional 패턴).
    ///
    /// # Errors
    ///
    /// 버전 불일치 → [`RepoError::Conflict`]. DB 통신 실패 → [`RepoError::Database`].
    async fn save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError>;
}

/// 지도 마커용 lightweight `Listing` projection.
///
/// 지도 렌더에 필요한 필드만 (전체 `Listing` 가져오기 비용 회피).
#[derive(Debug, Clone, PartialEq)]
pub struct ListingMarker {
    /// 매물 ID.
    pub id: Id<ListingIdMarker>,
    /// 좌표 (`WGS84`).
    pub geom: PointSrid,
    /// 가격 (`KRW`).
    pub price: MoneyKrw,
    /// 매물 유형.
    pub listing_type: ListingType,
    /// 거래 유형.
    pub transaction_type: TransactionType,
}

/// 카드 list 용 풍부한 projection (지도 핀 + 우측 카드 양쪽 사용).
///
/// 전체 [`Listing`] 의 21 필드 중 list 페이지에 필요한 것만.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingCardSummary {
    /// 매물 ID (`lst_...`).
    pub id: Id<ListingIdMarker>,
    /// 제목.
    pub title: String,
    /// 좌표 (`WGS84`, `geom_point` 가 NULL 인 매물은 응답 제외).
    pub geom: PointSrid,
    /// 매물 유형.
    pub listing_type: ListingType,
    /// 거래 유형.
    pub transaction_type: TransactionType,
    /// 주가격 (sale=매매가, jeonse=보증금, monthly\_rent=월세).
    pub price: MoneyKrw,
    /// 보증금 (월세/전세 만; sale 은 None).
    pub deposit: Option<MoneyKrw>,
    /// 월세 (monthly\_rent 만; sale/jeonse 는 None).
    pub monthly_rent: Option<MoneyKrw>,
    /// 면적 (m²).
    pub area_m2: f64,
    /// 사진 thumbnail URL (없으면 None — placeholder UI).
    pub thumbnail_url: Option<String>,
    /// 조회수.
    pub view_count: i64,
    /// 즐겨찾기 수.
    pub bookmark_count: i64,
    /// 등록일.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 카드 list 검색 조건 (모두 optional, default 는 "전체").
#[derive(Debug, Clone, Default)]
pub struct CardSearchQuery {
    /// 지도 영역 (4326). None 이면 한국 전체.
    pub bbox: Option<BoundingBox>,
    /// `listing_type` 필터 (None or empty = 6 종 모두).
    pub types: Option<Vec<ListingType>>,
    /// `transaction_type` 필터 (None or empty = 3 종 모두).
    pub transactions: Option<Vec<TransactionType>>,
    /// `area_m2 >=` (None = 0).
    pub min_area_m2: Option<f64>,
    /// `area_m2 <=` (None = +inf).
    pub max_area_m2: Option<f64>,
    /// `price_krw >=` (None = 0).
    pub min_price_krw: Option<i64>,
    /// `price_krw <=` (None = +inf).
    pub max_price_krw: Option<i64>,
    /// page (0-indexed).
    pub page: u32,
    /// page 당 항목 수 (max 100).
    pub size: u32,
    /// 정렬.
    pub sort: CardSearchSort,
}

/// 정렬 방식.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CardSearchSort {
    /// 등록일 최신순 (default).
    #[default]
    CreatedAtDesc,
    /// 가격 오름차순.
    PriceAsc,
    /// 가격 내림차순.
    PriceDesc,
    /// 면적 오름차순.
    AreaAsc,
    /// 면적 내림차순.
    AreaDesc,
}

/// `Repository` 에러.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 대상 Aggregate 미존재.
    #[error("not found")]
    NotFound,
    /// Optimistic lock 버전 불일치.
    #[error("conflict (version mismatch)")]
    Conflict,
    /// DB 통신/SQL 에러 (정보 누설 방지로 메시지만).
    #[error("database error: {0}")]
    Database(String),
}
