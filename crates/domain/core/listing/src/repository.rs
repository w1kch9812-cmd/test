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
