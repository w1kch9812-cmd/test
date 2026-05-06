//! `Listing` 도메인 에러.

use thiserror::Error;

use shared_kernel::listing_status::ListingStatus;
use shared_kernel::transaction_type::TransactionType;

/// `Listing` Aggregate 검증/상태 전이 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingError {
    /// `transaction_type` 과 `deposit`/`monthly_rent` 일관성 위반 (`V003_01`).
    ///
    /// `Sale` → 둘 다 `None`. `MonthlyRent` → 둘 다 `Some`.
    /// `Jeonse` → `deposit` `Some` + `monthly_rent` `None`.
    #[error(
        "transaction_type {transaction_type:?} requires deposit={deposit_required}, monthly_rent={monthly_rent_required}"
    )]
    TransactionFieldsMismatch {
        /// 입력 `transaction_type`.
        transaction_type: TransactionType,
        /// `deposit` `Some` 필요 여부.
        deposit_required: bool,
        /// `monthly_rent` `Some` 필요 여부.
        monthly_rent_required: bool,
    },
    /// 상태 머신 위반 — `from` → `to` 전이가 허용되지 않음 (spec § 8.3).
    #[error("invalid status transition: {from:?} -> {to:?}")]
    InvalidTransition {
        /// 현재 상태.
        from: ListingStatus,
        /// 시도된 대상 상태.
        to: ListingStatus,
    },
    /// 현재 상태에서 필드 수정 불가 (`Draft` 또는 `Rejected` 만 허용).
    ///
    /// `Active` / `PendingReview` / `Sold` / `Expired` / `Archived` 모두 거부 —
    /// 매물 데이터 무결성 보호. 변경 필요 시 `revise_after_rejection` 등으로
    /// `Draft` 로 돌아온 후에만.
    #[error("listing in {current:?} state cannot be edited (only Draft/Rejected)")]
    ImmutableState {
        /// 현재 상태.
        current: ListingStatus,
    },
}
