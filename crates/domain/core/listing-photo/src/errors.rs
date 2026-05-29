//! `ListingPhoto` 도메인 에러.

use thiserror::Error;

/// `ListingPhoto` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingPhotoError {
    /// `r2_key` 빈 문자열.
    #[error("r2_key cannot be empty")]
    EmptyR2Key,
    /// `display_order` 음수.
    #[error("display_order must be >= 0 (got {actual})")]
    NegativeDisplayOrder {
        /// 실제 값.
        actual: i32,
    },
    /// `caption` 200자 초과.
    #[error("caption exceeds 200 chars (got {actual})")]
    CaptionTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// Confirming a soft-deleted photo is not allowed.
    #[error("deleted photo cannot be confirmed")]
    DeletedCannotConfirm,
    /// `width_px` must be positive when present.
    #[error("width_px must be > 0 (got {actual})")]
    InvalidWidthPx {
        /// Actual value.
        actual: i32,
    },
    /// `height_px` must be positive when present.
    #[error("height_px must be > 0 (got {actual})")]
    InvalidHeightPx {
        /// Actual value.
        actual: i32,
    },
    /// `file_size_bytes` must be positive after storage confirmation.
    #[error("file_size_bytes must be > 0 (got {actual})")]
    InvalidFileSizeBytes {
        /// Actual value.
        actual: i64,
    },
}
