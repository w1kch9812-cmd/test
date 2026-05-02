//! `User` 도메인 에러.

// `UserError` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use thiserror::Error;

/// `User` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UserError {
    /// `display_name` 빈 문자열 (또는 공백만).
    #[error("display_name cannot be empty")]
    EmptyDisplayName,
    /// `display_name` 100자 초과.
    #[error("display_name exceeds 100 chars (got {actual})")]
    DisplayNameTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `zitadel_sub` 빈 문자열.
    #[error("zitadel_sub cannot be empty")]
    EmptyZitadelSub,
    /// `zitadel_sub` 255자 초과.
    #[error("zitadel_sub exceeds 255 chars (got {actual})")]
    ZitadelSubTooLong {
        /// 실제 길이.
        actual: usize,
    },
}
