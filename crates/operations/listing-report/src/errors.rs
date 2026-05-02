//! `ListingReport` 도메인 에러.

use thiserror::Error;

use crate::status::ListingReportStatus;

/// `ListingReport` Aggregate 검증/전이 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingReportError {
    /// `detail` 가 2000자 초과 (`try_new` 검증).
    #[error("detail exceeds 2000 chars (got {actual})")]
    DetailTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `handler_note` 가 비어있음 (`mark_confirmed` / `mark_dismissed` 는 메모 필수).
    #[error("handler_note cannot be empty")]
    EmptyHandlerNote,
    /// `handler_note` 가 2000자 초과.
    #[error("handler_note exceeds 2000 chars (got {actual})")]
    HandlerNoteTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// 현재 상태에서 시도한 전이가 허용되지 않음 (이미 terminal 이거나 자기 자신으로 전이).
    #[error("invalid transition from {from}")]
    InvalidTransition {
        /// 전이 시점의 상태.
        from: ListingReportStatus,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_too_long_message() {
        let err = ListingReportError::DetailTooLong { actual: 2500 };
        assert_eq!(err.to_string(), "detail exceeds 2000 chars (got 2500)");
    }

    #[test]
    fn empty_handler_note_message() {
        let err = ListingReportError::EmptyHandlerNote;
        assert_eq!(err.to_string(), "handler_note cannot be empty");
    }

    #[test]
    fn handler_note_too_long_message() {
        let err = ListingReportError::HandlerNoteTooLong { actual: 2001 };
        assert_eq!(
            err.to_string(),
            "handler_note exceeds 2000 chars (got 2001)"
        );
    }

    #[test]
    fn invalid_transition_message_from_confirmed() {
        let err = ListingReportError::InvalidTransition {
            from: ListingReportStatus::Confirmed,
        };
        assert_eq!(err.to_string(), "invalid transition from confirmed");
    }

    #[test]
    fn invalid_transition_message_from_investigating() {
        let err = ListingReportError::InvalidTransition {
            from: ListingReportStatus::Investigating,
        };
        assert_eq!(err.to_string(), "invalid transition from investigating");
    }

    #[test]
    fn equality_holds_per_variant() {
        assert_eq!(
            ListingReportError::EmptyHandlerNote,
            ListingReportError::EmptyHandlerNote
        );
        assert_eq!(
            ListingReportError::DetailTooLong { actual: 10 },
            ListingReportError::DetailTooLong { actual: 10 }
        );
        assert_ne!(
            ListingReportError::DetailTooLong { actual: 10 },
            ListingReportError::DetailTooLong { actual: 11 }
        );
    }
}
