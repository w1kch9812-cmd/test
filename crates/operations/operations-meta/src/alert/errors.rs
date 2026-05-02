//! `SystemAlert` 도메인 에러.

use thiserror::Error;

/// `SystemAlert` Aggregate 검증/전이 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SystemAlertError {
    /// `source` 가 비어있음 (trim 후).
    #[error("source cannot be empty")]
    EmptySource,
    /// `source` 가 50자 초과 (DB `varchar(50)` 한계).
    #[error("source exceeds 50 chars (got {actual})")]
    SourceTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `title` 가 비어있음 (trim 후).
    #[error("title cannot be empty")]
    EmptyTitle,
    /// `title` 가 200자 초과 (DB `varchar(200)` 한계).
    #[error("title exceeds 200 chars (got {actual})")]
    TitleTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `detail` 가 4000자 초과 (도메인 sanity bound; DB 는 `text`).
    #[error("detail exceeds 4000 chars (got {actual})")]
    DetailTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// 이미 acknowledge 된 알림에 대해 `acknowledge` 재호출.
    #[error("alert already acknowledged")]
    AlreadyAcknowledged,
    /// 이미 resolve 된 알림에 대해 `resolve` 재호출.
    #[error("alert already resolved")]
    AlreadyResolved,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_message() {
        assert_eq!(
            SystemAlertError::EmptySource.to_string(),
            "source cannot be empty"
        );
    }

    #[test]
    fn source_too_long_message() {
        assert_eq!(
            SystemAlertError::SourceTooLong { actual: 60 }.to_string(),
            "source exceeds 50 chars (got 60)"
        );
    }

    #[test]
    fn empty_title_message() {
        assert_eq!(
            SystemAlertError::EmptyTitle.to_string(),
            "title cannot be empty"
        );
    }

    #[test]
    fn title_too_long_message() {
        assert_eq!(
            SystemAlertError::TitleTooLong { actual: 250 }.to_string(),
            "title exceeds 200 chars (got 250)"
        );
    }

    #[test]
    fn detail_too_long_message() {
        assert_eq!(
            SystemAlertError::DetailTooLong { actual: 4500 }.to_string(),
            "detail exceeds 4000 chars (got 4500)"
        );
    }

    #[test]
    fn already_acknowledged_message() {
        assert_eq!(
            SystemAlertError::AlreadyAcknowledged.to_string(),
            "alert already acknowledged"
        );
    }

    #[test]
    fn already_resolved_message() {
        assert_eq!(
            SystemAlertError::AlreadyResolved.to_string(),
            "alert already resolved"
        );
    }
}
