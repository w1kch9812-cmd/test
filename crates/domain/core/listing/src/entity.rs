//! `Listing` Aggregate 구조체 + `try_new_draft` 생성자.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::geometry::PointSrid;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;

use crate::errors::ListingError;

/// `Listing` Aggregate (spec § 5.1, 20 필드).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Listing {
    /// `lst_<26 ULID>` 형식 ID.
    pub id: Id<ListingMarker>,
    /// 소유자 (`User` Aggregate FK).
    pub owner_id: Id<UserMarker>,
    /// 필지 `PNU` (R2의 `Parcel`과 매핑 — `FK` 아님).
    pub parcel_pnu: Pnu,
    /// 매물 유형 (`factory`/`warehouse`/...).
    pub listing_type: ListingType,
    /// 거래 유형 (`sale`/`monthly_rent`/`jeonse`).
    pub transaction_type: TransactionType,
    /// 가격 (`KRW`).
    pub price: MoneyKrw,
    /// 보증금 (`KRW`). `MonthlyRent`/`Jeonse`에서 `Some`.
    pub deposit: Option<MoneyKrw>,
    /// 월세 (`KRW`). `MonthlyRent`에서만 `Some`.
    pub monthly_rent: Option<MoneyKrw>,
    /// 면적 (`m²`).
    pub area: AreaM2,
    /// 제목 (≤200자).
    pub title: ListingTitle,
    /// 설명 (≤5000자, 빈 허용).
    pub description: Description,
    /// 상태 (`Draft` → ... → `Sold`/`Expired`/`Rejected`).
    pub status: ListingStatus,
    /// 연락처 공개 범위.
    pub contact_visibility: ContactVisibility,
    /// 조회수 (`u64`, monotonic).
    pub view_count: u64,
    /// 북마크 수 (`u64`).
    pub bookmark_count: u64,
    /// 매물 좌표 (`WGS84` `Point`). 선택 — 필지 좌표는 R2.
    pub geom_point: Option<PointSrid>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
    /// 마지막 갱신 시각.
    pub updated_at: DateTime<Utc>,
    /// 만료 시각 (선택).
    pub expires_at: Option<DateTime<Utc>>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl Listing {
    /// `Draft` 상태 새 매물 생성. `V003_01` cross-field invariant 강제.
    ///
    /// `created_at == updated_at == now`, `status = Draft`,
    /// `contact_visibility = LoginRequired`, `view_count = 0`,
    /// `bookmark_count = 0`, `expires_at = None`, `version = 1`.
    ///
    /// # Errors
    ///
    /// `transaction_type` 과 `deposit`/`monthly_rent` 조합이 `V003_01`과 다르면
    /// [`ListingError::TransactionFieldsMismatch`]. (예: `Sale`인데
    /// `deposit` `Some` 등)
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자
    pub fn try_new_draft(
        id: Id<ListingMarker>,
        owner_id: Id<UserMarker>,
        parcel_pnu: Pnu,
        listing_type: ListingType,
        transaction_type: TransactionType,
        price: MoneyKrw,
        deposit: Option<MoneyKrw>,
        monthly_rent: Option<MoneyKrw>,
        area: AreaM2,
        title: ListingTitle,
        description: Description,
        geom_point: Option<PointSrid>,
        now: DateTime<Utc>,
    ) -> Result<Self, ListingError> {
        let deposit_required = transaction_type.requires_deposit();
        let monthly_rent_required = transaction_type.requires_monthly_rent();

        if deposit.is_some() != deposit_required {
            return Err(ListingError::TransactionFieldsMismatch {
                transaction_type,
                deposit_required,
                monthly_rent_required,
            });
        }
        if monthly_rent.is_some() != monthly_rent_required {
            return Err(ListingError::TransactionFieldsMismatch {
                transaction_type,
                deposit_required,
                monthly_rent_required,
            });
        }

        Ok(Self {
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
            status: ListingStatus::Draft,
            contact_visibility: ContactVisibility::LoginRequired,
            view_count: 0,
            bookmark_count: 0,
            geom_point,
            created_at: now,
            updated_at: now,
            expires_at: None,
            version: 1,
        })
    }

    /// 내부 헬퍼 — 상태 전이 + `version` bump + `updated_at` 갱신.
    ///
    /// `ListingStatus::can_transition_to`로 spec § 8.3 머신 검사.
    fn transition_to(
        &mut self,
        target: ListingStatus,
        at: DateTime<Utc>,
    ) -> Result<(), ListingError> {
        if !self.status.can_transition_to(target) {
            return Err(ListingError::InvalidTransition {
                from: self.status,
                to: target,
            });
        }
        self.status = target;
        self.version += 1;
        self.updated_at = at;
        Ok(())
    }

    /// `Draft` → `PendingReview`. 사용자가 검토 요청.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Draft`가 아니면 [`ListingError::InvalidTransition`].
    pub fn submit_for_review(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::PendingReview, at)
    }

    /// `PendingReview` → `Active`. 어드민 승인.
    ///
    /// `reviewed_by`/`reviewed_at` 추적은 별도 `listing_review_queue` 테이블.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `PendingReview`가 아니면 [`ListingError::InvalidTransition`].
    pub fn approve(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Active, at)
    }

    /// `PendingReview` → `Rejected`. 어드민 거부.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `PendingReview`가 아니면 [`ListingError::InvalidTransition`].
    pub fn reject(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Rejected, at)
    }

    /// `Rejected` → `Draft`. 사용자 수정 후 재제출 준비.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Rejected`가 아니면 [`ListingError::InvalidTransition`].
    pub fn revise_after_rejection(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Draft, at)
    }

    /// `Active` → `Sold`. 판매 완료 (terminal).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Active`가 아니면 [`ListingError::InvalidTransition`].
    pub fn mark_sold(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Sold, at)
    }

    /// `Active` → `Expired`. 만료 처리 (terminal).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Active`가 아니면 [`ListingError::InvalidTransition`].
    pub fn expire(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Expired, at)
    }

    /// 조회수 증가 (`saturating_add`). `version`은 bump하지 *않아요* — 빈번한
    /// 갱신이라 optimistic lock 충돌과 무관해요.
    pub fn increment_view_count(&mut self, at: DateTime<Utc>) {
        self.view_count = self.view_count.saturating_add(1);
        self.updated_at = at;
    }

    /// 북마크 수 증가 (`saturating_add`). `version` bump *안* 함.
    pub fn record_bookmark(&mut self, at: DateTime<Utc>) {
        self.bookmark_count = self.bookmark_count.saturating_add(1);
        self.updated_at = at;
    }

    /// 북마크 수 감소 (`saturating_sub`, `0` 이하면 `0` 유지). `version` bump *안* 함.
    pub fn release_bookmark(&mut self, at: DateTime<Utc>) {
        self.bookmark_count = self.bookmark_count.saturating_sub(1);
        self.updated_at = at;
    }
}

// Tests in sibling files via #[path] (anticipate >500 lines combined).
#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;

#[cfg(test)]
#[path = "methods_tests.rs"]
mod methods_tests;
