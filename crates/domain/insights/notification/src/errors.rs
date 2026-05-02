//! `Notification` 도메인 에러.

use thiserror::Error;

/// `Notification` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NotificationError {
    /// `kind` 빈.
    #[error("kind cannot be empty")]
    EmptyKind,
    /// `kind` 50자 초과.
    #[error("kind exceeds 50 chars (got {actual})")]
    KindTooLong {
        /// 실제 길이.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_kind_message() {
        let err = NotificationError::EmptyKind;
        assert_eq!(err.to_string(), "kind cannot be empty");
    }

    #[test]
    fn kind_too_long_message() {
        let err = NotificationError::KindTooLong { actual: 51 };
        assert_eq!(err.to_string(), "kind exceeds 50 chars (got 51)");
    }
}
