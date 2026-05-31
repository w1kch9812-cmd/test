//! `ListingRepository` port (interface) + listing read projections.
//!
//! 구현체는 sub-project 5 (`crates/db`)에서 추가.

// `ListingRepository`처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::admin_division::EupmyeondongCode;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use shared_kernel::zoning::Zoning;
use thiserror::Error;

use crate::entity::Listing;
pub use crate::marker_filter::{
    ListingMarkerFilter, ListingMarkerFilterError, ListingMarkerFilterSpec,
    NormalizedListingMarkerFilterSpec, ALL_ACTIVE_LISTING_MARKER_FILTER_HASH,
};

/// `Listing` 저장/조회 포트.
#[async_trait]
pub trait ListingRepository: Send + Sync {
    /// `id`로 조회. 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find(&self, id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError>;

    /// 카드 list 검색 — `status='active'` + filter 적용.
    ///
    /// `query.size` 의 max 100 (caller 가 검증). 응답은 (cards, total\_count) 튜플.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_card_summaries(
        &self,
        query: CardSearchQuery,
    ) -> Result<(Vec<ListingCardSummary>, u64), RepoError>;

    /// Gongzzang-owned listing marker `MVT/PBF` tile.
    ///
    /// Marker positions must come from the platform-core `PNU` anchor projection, not from
    /// listing-owned coordinates. A successful tile must preserve `represented_count ==
    /// eligible_count`.
    ///
    /// # Errors
    ///
    /// DB failures, incomplete anchor coverage, or completeness invariant failures return
    /// [`RepoError::Database`].
    async fn find_listing_marker_tile(
        &self,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError>;

    /// Return marker ids matching a filter for a loaded listing marker tile.
    ///
    /// The mask is a serving optimization and must not expose canonical coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_mask(
        &self,
        query: ListingMarkerMaskQuery,
    ) -> Result<ListingMarkerMask, RepoError>;

    /// Return marker ids that must be hidden for a loaded listing marker tile.
    ///
    /// Tombstones prevent deleted, sold, rejected, expired, or private markers from remaining
    /// visible while cached base tiles age out.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError>;

    /// Return recent public marker changes for a loaded listing marker tile.
    ///
    /// Delta overlays improve write freshness before the base tile cache or artifact is refreshed.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_deltas(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerDeltas, RepoError>;

    /// Upsert the listing marker serving projection from listing semantics and PNU anchor data.
    ///
    /// The projection is not a coordinate source of truth. Marker position must be copied from
    /// the platform-core-owned `parcel_marker_anchor` read model.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::NotFound`] when the listing is absent, and [`RepoError::Database`]
    /// when the listing has no PNU anchor.
    async fn upsert_listing_marker_projection(
        &self,
        id: &Id<ListingIdMarker>,
    ) -> Result<(), RepoError>;

    /// Count public listing markers from the marker serving projection/index.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn count_listing_markers(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerCount, RepoError>;

    /// Register a normalized marker filter and return its stable hash.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the registry write fails.
    async fn register_listing_marker_filter(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerRegisteredFilter, RepoError>;

    /// Resolve a registered marker filter hash to its normalized payload.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the registry read fails.
    async fn resolve_listing_marker_filter(
        &self,
        filter_hash: &str,
    ) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError>;

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

    /// 상세 페이지용 — `Listing` + photos + bookmark 정보 (SP6-iii).
    ///
    /// `viewer_user_id` 는 `is_bookmarked` JOIN 에 사용. RBAC 는 호출 측 (handler)
    /// 가 `Listing.status` + `owner_id == viewer` 비교로 차단 — repo 는 데이터만
    /// 반환.
    ///
    /// # Errors
    ///
    /// 매물 미존재 → `Ok(None)`. DB 통신 실패 → [`RepoError::Database`].
    async fn find_detail_by_id(
        &self,
        id: &Id<ListingIdMarker>,
        viewer_user_id: &Id<UserMarker>,
    ) -> Result<Option<ListingDetail>, RepoError>;

    /// `view_count` 1 증가. version bump X / `audit_log` X (빈도 높아 분리).
    ///
    /// 본인 본인 매물 조회 시 skip — handler 책임.
    ///
    /// # Errors
    ///
    /// 매물 미존재 → [`RepoError::NotFound`]. DB 통신 실패 → [`RepoError::Database`].
    async fn increment_view_count(&self, id: &Id<ListingIdMarker>) -> Result<(), RepoError>;

    /// PNU 파생 denormalize 컬럼 갱신 (SP9 T4, ADR 0018).
    ///
    /// 매물 등록 직후 (V-World lookup 후) 또는 월간 재매핑 cron 에서 호출. version
    /// bump X / audit\_log X — *비즈니스 변경이 아닌 캐시 동기화* 라서 기록 가치
    /// 낮음. 추적은 `listing.parcel_lookup_at` timestamp 컬럼이 담당.
    ///
    /// `denormalize` 의 `zoning` 이 `None` 이면 DB 컬럼도 NULL 로 — V-World 가
    /// 용도지역 미제공 시 자연스러움.
    ///
    /// # Errors
    ///
    /// 매물 미존재 → [`RepoError::NotFound`]. DB 통신 실패 → [`RepoError::Database`].
    async fn update_parcel_denormalize(
        &self,
        id: &Id<ListingIdMarker>,
        denormalize: &ListingParcelDenormalize,
    ) -> Result<(), RepoError>;
}

/// PNU 파생 denormalize 입력 — `update_parcel_denormalize` 인자.
///
/// PNU 자체는 listing 의 정체성이라 본 struct 에 없음. 본 struct 는 PNU 가
/// *지시하는* 외부 사실 (행정구역, 지목, 용도지역) 을 담음.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingParcelDenormalize {
    /// 행정구역 8자리 (시도+시군구+읍면동).
    pub admin_code: EupmyeondongCode,
    /// 지목.
    pub land_use_type: LandUseType,
    /// 용도지역 (V-World `LP_PA_CBND_BUBUN` 미제공이면 `None`).
    pub zoning: Option<Zoning>,
}

/// 매물 상세 페이지 응답 — SP6-iii.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingDetail {
    /// 전체 `Listing` Aggregate (21 필드).
    pub listing: Listing,
    /// 활성 사진 (soft-delete 제외, `display_order ASC`).
    pub photos: Vec<ListingPhotoSummary>,
    /// 즐겨찾기 수 — `bookmark_listing` JOIN COUNT.
    pub bookmark_count: i64,
    /// 본 viewer 가 즐겨찾기 한 매물인지.
    pub is_bookmarked: bool,
}

/// 사진 요약 — frontend 표시용 5 필드 (audit/소유 메타 제외, SP6-iii).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingPhotoSummary {
    /// Photo ID (`lph_...`).
    pub photo_id: String,
    /// `R2` 객체 키.
    pub r2_key: String,
    /// 썸네일 `R2` 키 (선택).
    pub thumbnail_r2_key: Option<String>,
    /// 캡션 (선택).
    pub caption: Option<String>,
    /// 표시 순서.
    pub display_order: i32,
    /// `MIME` content-type 문자열 (`image/jpeg` 등).
    pub content_type: String,
}

/// 카드 list 용 풍부한 projection (지도 핀 + 우측 카드 양쪽 사용).
///
/// 전체 [`Listing`] 의 21 필드 중 list 페이지에 필요한 것만.
///
/// SP6-iii: `is_bookmarked` 와 `bookmark_count` 는 `bookmark_listing` 테이블
/// JOIN COUNT 결과 — `Listing.bookmark_count` denormalized 필드는 deprecated
/// (FU 70 schema 제거 예정).
#[derive(Debug, Clone, PartialEq)]
pub struct ListingCardSummary {
    /// 매물 ID (`lst_...`).
    pub id: Id<ListingIdMarker>,
    /// 제목.
    pub title: String,
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
    /// 즐겨찾기 수 (SP6-iii: `bookmark_listing` JOIN COUNT — denormalized 사본
    /// 아님).
    pub bookmark_count: i64,
    /// 본 viewer (`viewer_user_id`) 가 즐겨찾기 한 매물인지 (SP6-iii).
    pub is_bookmarked: bool,
    /// 등록일.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Gongzzang listing marker vector-tile layer name.
pub const LISTING_MARKER_TILE_LAYER: &str = "listing";

/// Gongzzang listing marker delta vector-tile layer name.
pub const LISTING_MARKER_DELTA_TILE_LAYER: &str = "listing_delta";

/// Marker tile response content type.
pub const LISTING_MARKER_TILE_CONTENT_TYPE: &str = "application/vnd.mapbox-vector-tile";

/// Minimum zoom accepted by the Gongzzang listing marker tile API.
pub const LISTING_MARKER_TILE_MIN_ZOOM: u8 = 0;

/// Lowest zoom where exact listing marker features are preferred.
pub const LISTING_MARKER_TILE_EXACT_MIN_ZOOM: u8 = 14;

/// Maximum zoom accepted by the listing marker tile API.
pub const LISTING_MARKER_TILE_MAX_ZOOM: u8 = 22;

/// Validated tile query for the listing marker PBF surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTileQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Typed marker filter.
    pub filter: ListingMarkerFilter,
}

impl ListingMarkerTileQuery {
    /// Build a query without validation. Use only when inputs are already trusted.
    #[must_use]
    pub const fn new(z: u8, x: u32, y: u32, filter: ListingMarkerFilter) -> Self {
        Self { z, x, y, filter }
    }

    /// Validate public tile-coordinate input.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerTileQueryError`] when zoom or axis values are outside the vector-tile
    /// coordinate range.
    pub fn try_new(
        z: u8,
        x: u32,
        y: u32,
        filter: ListingMarkerFilter,
    ) -> Result<Self, ListingMarkerTileQueryError> {
        if !(LISTING_MARKER_TILE_MIN_ZOOM..=LISTING_MARKER_TILE_MAX_ZOOM).contains(&z) {
            return Err(ListingMarkerTileQueryError::InvalidZoom { z });
        }
        let axis_limit = 1_u32 << u32::from(z);
        if x >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidX { z, x });
        }
        if y >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidY { z, y });
        }
        Ok(Self::new(z, x, y, filter))
    }
}

/// Listing marker tile coordinate validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ListingMarkerTileQueryError {
    /// Zoom is outside the accepted MVT range.
    #[error("listing marker tile zoom out of range: {z}")]
    InvalidZoom {
        /// Invalid zoom.
        z: u8,
    },
    /// X coordinate is outside the zoom-dependent axis range.
    #[error("listing marker tile x out of range for z={z}: {x}")]
    InvalidX {
        /// Zoom.
        z: u8,
        /// Invalid x coordinate.
        x: u32,
    },
    /// Y coordinate is outside the zoom-dependent axis range.
    #[error("listing marker tile y out of range for z={z}: {y}")]
    InvalidY {
        /// Zoom.
        z: u8,
        /// Invalid y coordinate.
        y: u32,
    },
}

/// Gongzzang listing marker PBF tile plus server-side completeness metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTile {
    /// MVT/PBF response bytes.
    pub bytes: Vec<u8>,
    /// MVT source-layer name.
    pub layer_name: &'static str,
    /// Active listings selected for this tile and filter.
    pub eligible_count: i64,
    /// Listings represented by returned features or truthful aggregates.
    pub represented_count: i64,
    /// Raw feature count in the tile.
    pub feature_count: i64,
    /// Aggregate feature count in the tile.
    pub aggregate_count: i64,
    /// Anchor snapshot identity used by represented features.
    pub anchor_snapshot_id: Option<String>,
}

/// Listing marker mask request for a loaded tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerMaskQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Typed marker filter.
    pub filter: ListingMarkerFilter,
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

/// Listing marker mask encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingMarkerMaskEncoding {
    /// `marker_ids` are the ids that should remain visible.
    Show,
    /// `marker_ids` are the ids that should be hidden.
    Hide,
}

impl ListingMarkerMaskEncoding {
    /// Stable JSON/API value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Hide => "hide",
        }
    }
}

/// Listing marker mask response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerMask {
    /// Compact mask encoding.
    pub encoding: ListingMarkerMaskEncoding,
    /// Marker ids selected by the mask. Coordinates are intentionally absent.
    pub marker_ids: Vec<String>,
    /// Highest projection version included in this mask.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in this mask.
    pub anchor_snapshot_id: Option<String>,
}

/// Query for listing marker overlay records addressed by tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerOverlayTileQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

impl ListingMarkerOverlayTileQuery {
    /// Validate public overlay tile-coordinate input.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerTileQueryError`] when zoom or axis values are outside the vector-tile
    /// coordinate range.
    pub fn try_new(
        z: u8,
        x: u32,
        y: u32,
        base_version: Option<i64>,
    ) -> Result<Self, ListingMarkerTileQueryError> {
        if z > LISTING_MARKER_TILE_MAX_ZOOM {
            return Err(ListingMarkerTileQueryError::InvalidZoom { z });
        }
        let axis_limit = 1_u32 << u32::from(z);
        if x >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidX { z, x });
        }
        if y >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidY { z, y });
        }
        Ok(Self {
            z,
            x,
            y,
            base_version,
        })
    }
}

/// Listing marker tombstone overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTombstones {
    /// Marker ids that must be hidden by the client.
    pub marker_ids: Vec<String>,
    /// Highest projection version represented by this tombstone response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this tombstone response.
    pub anchor_snapshot_id: Option<String>,
}

/// Listing marker delta overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerDeltas {
    /// MVT/PBF response bytes for recently changed public markers.
    pub bytes: Vec<u8>,
    /// MVT source-layer name.
    pub layer_name: &'static str,
    /// Number of changed marker features represented.
    pub feature_count: i64,
    /// Highest projection version represented by this delta response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this delta response.
    pub anchor_snapshot_id: Option<String>,
}

/// Exact listing marker count and projection metadata for a normalized filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerCount {
    /// Exact public marker count for the filter.
    pub total_count: i64,
    /// Highest projection version included in the count result.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in the count result.
    pub anchor_snapshot_id: Option<String>,
}

/// Registered listing marker filter identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerRegisteredFilter {
    /// Stable filter hash used by public tile/count/mask routes.
    pub filter_hash: String,
}

/// 카드 list 검색 조건 (모두 optional, default 는 "전체").
///
/// SP6-iii: `viewer_user_id` 가 필수 — 모든 listings endpoint 가 인증 사용자
/// 전용이라 항상 존재. `is_bookmarked` JOIN 에 사용.
#[derive(Debug, Clone)]
pub struct CardSearchQuery {
    /// 필지 PNU 19자리 정확 매칭 (ADR 0018 SP9 T4) — 폴리곤 클릭 시 사용.
    pub pnu: Option<Pnu>,
    /// 행정구역 코드 prefix 매칭 (2/5/8자리) — 시도/시군구/읍면동 어느 단계든.
    pub admin_code_prefix: Option<String>,
    /// 지목 필터 (`parcel_land_use_type` denormalize 컬럼 — V-World lookup 결과).
    pub land_use_type: Option<LandUseType>,
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
    /// 검색하는 사용자 ID — `is_bookmarked` JOIN 에 사용 (SP6-iii). 인증 사용자
    /// 전용 endpoint 라 항상 Some — anonymous 접근은 SP9 (B2C 확장) 영역.
    pub viewer_user_id: Id<UserMarker>,
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

#[cfg(test)]
mod tests {
    use super::ListingMarkerOverlayTileQuery;

    #[test]
    fn listing_marker_overlay_query_rejects_out_of_range_tiles() {
        assert!(ListingMarkerOverlayTileQuery::try_new(23, 0, 0, None).is_err());
        assert!(ListingMarkerOverlayTileQuery::try_new(4, 16, 0, None).is_err());
        assert!(ListingMarkerOverlayTileQuery::try_new(4, 0, 16, None).is_err());
    }
}
