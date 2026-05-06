//! `ListingRepository` port (interface) + 지도 마커 projection.
//!
//! 구현체는 sub-project 5 (`crates/db`)에서 추가.

// `ListingRepository`/`ListingMarker` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::admin_division::EupmyeondongCode;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::geometry::PointSrid;
// `shared_kernel::id::ListingMarker`는 `Id<_>`용 phantom marker.
// 이 모듈의 `ListingMarker` projection과 이름이 겹치므로 `ListingIdMarker`로 별명 부여.
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

    /// `view_count` 1 증가. version bump X / audit_log X (빈도 높아 분리).
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
    /// 즐겨찾기 수 (SP6-iii: `bookmark_listing` JOIN COUNT — denormalized 사본
    /// 아님).
    pub bookmark_count: i64,
    /// 본 viewer (`viewer_user_id`) 가 즐겨찾기 한 매물인지 (SP6-iii).
    pub is_bookmarked: bool,
    /// 등록일.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 카드 list 검색 조건 (모두 optional, default 는 "전체").
///
/// SP6-iii: `viewer_user_id` 가 필수 — 모든 listings endpoint 가 인증 사용자
/// 전용이라 항상 존재. `is_bookmarked` JOIN 에 사용.
#[derive(Debug, Clone)]
pub struct CardSearchQuery {
    /// 지도 영역 (4326). None 이면 한국 전체. **Deprecated (ADR 0018)** —
    /// `pnu` / `admin_code` 기반 검색으로 대체 진행 중. `geom_point` 컬럼 제거
    /// 시 함께 제거.
    pub bbox: Option<BoundingBox>,
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
