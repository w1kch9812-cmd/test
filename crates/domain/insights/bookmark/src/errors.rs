//! `Bookmark` 도메인 에러.

use thiserror::Error;

/// `Bookmark` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BookmarkError {
    /// `target_id` 빈.
    #[error("target_id cannot be empty")]
    EmptyTargetId,
    /// `target_id` 50자 초과.
    #[error("target_id exceeds 50 chars (got {actual})")]
    TargetIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `note` 500자 초과.
    #[error("note exceeds 500 chars (got {actual})")]
    NoteTooLong {
        /// 실제 길이.
        actual: usize,
    },
}
