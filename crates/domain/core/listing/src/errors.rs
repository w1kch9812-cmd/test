//! `Listing` 도메인 에러.

use thiserror::Error;

use shared_kernel::transaction_type::TransactionType;

/// `Listing` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingError {
    /// `transaction_type` 과 `deposit`/`monthly_rent` 일관성 위반 (V003_01).
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
}
