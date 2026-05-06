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
    const fn transition_to(
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
    pub const fn submit_for_review(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::PendingReview, at)
    }

    /// `PendingReview` → `Active`. 어드민 승인.
    ///
    /// `reviewed_by`/`reviewed_at` 추적은 별도 `listing_review_queue` 테이블.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `PendingReview`가 아니면 [`ListingError::InvalidTransition`].
    pub const fn approve(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Active, at)
    }

    /// `PendingReview` → `Rejected`. 어드민 거부.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `PendingReview`가 아니면 [`ListingError::InvalidTransition`].
    pub const fn reject(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Rejected, at)
    }

    /// `Rejected` → `Draft`. 사용자 수정 후 재제출 준비.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Rejected`가 아니면 [`ListingError::InvalidTransition`].
    pub const fn revise_after_rejection(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Draft, at)
    }

    /// `Active` → `Sold`. 판매 완료 (terminal).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Active`가 아니면 [`ListingError::InvalidTransition`].
    pub const fn mark_sold(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Sold, at)
    }

    /// `Active` → `Expired`. 만료 처리 (terminal).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Active`가 아니면 [`ListingError::InvalidTransition`].
    pub const fn expire(&mut self, at: DateTime<Utc>) -> Result<(), ListingError> {
        self.transition_to(ListingStatus::Expired, at)
    }

    /// 조회수 증가 (`saturating_add`). `version`은 bump하지 *않아요* — 빈번한
    /// 갱신이라 optimistic lock 충돌과 무관해요.
    pub const fn increment_view_count(&mut self, at: DateTime<Utc>) {
        self.view_count = self.view_count.saturating_add(1);
        self.updated_at = at;
    }

    /// 북마크 수 증가 (`saturating_add`). `version` bump *안* 함.
    ///
    /// **Deprecated (SP6-iii)**: `Listing.bookmark_count` denormalized 필드는
    /// 응답 시 JOIN COUNT 로 대체됨. 본 메서드는 호출자 0 (FU 70 schema 컬럼
    /// 제거 후 함께 제거).
    #[deprecated(
        since = "0.2.0",
        note = "FU 70: bookmark_count denormalization 제거. JOIN COUNT 응답으로 전환."
    )]
    pub const fn record_bookmark(&mut self, at: DateTime<Utc>) {
        self.bookmark_count = self.bookmark_count.saturating_add(1);
        self.updated_at = at;
    }

    /// 북마크 수 감소 (`saturating_sub`, `0` 이하면 `0` 유지). `version` bump *안* 함.
    ///
    /// **Deprecated (SP6-iii)**: `record_bookmark` 와 동일 사유.
    #[deprecated(
        since = "0.2.0",
        note = "FU 70: bookmark_count denormalization 제거. JOIN COUNT 응답으로 전환."
    )]
    pub const fn release_bookmark(&mut self, at: DateTime<Utc>) {
        self.bookmark_count = self.bookmark_count.saturating_sub(1);
        self.updated_at = at;
    }

    /// 편집 가능한 필드 일괄 갱신 — `Draft` / `Rejected` 상태에서만 허용.
    ///
    /// `transaction_type` / `parcel_pnu` / `listing_type` / `owner_id` 는 변경
    /// *불가* — 다른 매물로 봐야 함. 이 invariant 가 cross-field 검증의 base.
    ///
    /// `deposit` / `monthly_rent` 변경 시 `transaction_type` 의 cross-field
    /// invariant (`V003_01`) 재검증. 즉 `MonthlyRent` 매물의 `deposit` 을
    /// `None` 으로 바꾸려 하면 거부.
    ///
    /// # Errors
    ///
    /// - 현재 상태가 `Draft`/`Rejected` 가 아니면 [`ListingError::ImmutableState`]
    /// - `deposit`/`monthly_rent` 가 `transaction_type` 과 불일치하면
    ///   [`ListingError::TransactionFieldsMismatch`]
    pub fn update_editable_fields(
        &mut self,
        update: ListingUpdate,
        at: DateTime<Utc>,
    ) -> Result<(), ListingError> {
        if !matches!(
            self.status,
            ListingStatus::Draft | ListingStatus::Rejected
        ) {
            return Err(ListingError::ImmutableState {
                current: self.status,
            });
        }

        // deposit / monthly_rent 변경 의도가 있으면 그 값 사용, 없으면 현재 값.
        let new_deposit = match update.deposit {
            Some(v) => v,
            None => self.deposit,
        };
        let new_monthly_rent = match update.monthly_rent {
            Some(v) => v,
            None => self.monthly_rent,
        };
        let dep_required = self.transaction_type.requires_deposit();
        let rent_required = self.transaction_type.requires_monthly_rent();
        if new_deposit.is_some() != dep_required {
            return Err(ListingError::TransactionFieldsMismatch {
                transaction_type: self.transaction_type,
                deposit_required: dep_required,
                monthly_rent_required: rent_required,
            });
        }
        if new_monthly_rent.is_some() != rent_required {
            return Err(ListingError::TransactionFieldsMismatch {
                transaction_type: self.transaction_type,
                deposit_required: dep_required,
                monthly_rent_required: rent_required,
            });
        }

        if let Some(t) = update.title {
            self.title = t;
        }
        if let Some(d) = update.description {
            self.description = d;
        }
        if let Some(p) = update.price {
            self.price = p;
        }
        self.deposit = new_deposit;
        self.monthly_rent = new_monthly_rent;
        if let Some(a) = update.area {
            self.area = a;
        }
        if let Some(g) = update.geom_point {
            self.geom_point = g;
        }
        if let Some(c) = update.contact_visibility {
            self.contact_visibility = c;
        }

        self.version += 1;
        self.updated_at = at;
        Ok(())
    }
}

/// `Listing::update_editable_fields` 의 partial-update 페이로드.
///
/// 외부 `Option` = "변경 의도 있음" / 내부 `Option` (`deposit` / `monthly_rent` /
/// `geom_point`) = 실제 값 (`None` 으로 clear 가능). 이 두-단계 Option 패턴이
/// partial update 의 표준 — `null` 로 clear 와 "필드 미언급" 을 구분.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListingUpdate {
    /// 제목 (변경 의도 있음 = `Some`).
    pub title: Option<ListingTitle>,
    /// 설명.
    pub description: Option<Description>,
    /// 가격.
    pub price: Option<MoneyKrw>,
    /// 보증금 — 외부 `Some` = 변경, 내부 `None` = clear (Sale 매물).
    pub deposit: Option<Option<MoneyKrw>>,
    /// 월세 — 동일 패턴.
    pub monthly_rent: Option<Option<MoneyKrw>>,
    /// 면적.
    pub area: Option<AreaM2>,
    /// 좌표 — 외부 `Some` = 변경, 내부 `None` = 좌표 제거.
    pub geom_point: Option<Option<PointSrid>>,
    /// 연락처 공개 범위.
    pub contact_visibility: Option<ContactVisibility>,
}

// Tests in sibling files via #[path] (anticipate >500 lines combined).
#[cfg(test)]
#[path = "entity_tests.rs"]
mod entity_tests;

#[cfg(test)]
#[path = "methods_tests.rs"]
mod methods_tests;
