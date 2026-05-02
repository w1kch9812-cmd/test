//! `ListingReviewQueue` 도메인 에러.

use thiserror::Error;

/// `ListingReviewQueue` Aggregate 검증/전이 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LrqError {
    /// 이미 결정이 내려진 큐에 다시 결정 시도.
    ///
    /// `decision` 이 `Some(_)` 인 큐는 모든 `decide_*` 호출이 거부돼요.
    #[error("lrq already decided")]
    AlreadyDecided,
    /// `reviewer_note` 가 비었음 (`reject` / `request_changes` 는 메모 필수).
    #[error("reviewer_note cannot be empty for {action}")]
    EmptyReviewerNote {
        /// 메모가 필수인 액션 이름 (예: `"reject"`, `"request_changes"`).
        action: &'static str,
    },
    /// `reviewer_note` 가 2000자 초과.
    #[error("reviewer_note exceeds 2000 chars (got {actual})")]
    ReviewerNoteTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `auto_check_score` 가 0-100 범위를 벗어남.
    #[error("auto_check_score must be 0-100 (got {actual})")]
    AutoCheckScoreOutOfRange {
        /// 실제 점수.
        actual: u32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_decided_message() {
        let err = LrqError::AlreadyDecided;
        assert_eq!(err.to_string(), "lrq already decided");
    }

    #[test]
    fn empty_reviewer_note_message_for_reject() {
        let err = LrqError::EmptyReviewerNote { action: "reject" };
        assert_eq!(err.to_string(), "reviewer_note cannot be empty for reject");
    }

    #[test]
    fn empty_reviewer_note_message_for_request_changes() {
        let err = LrqError::EmptyReviewerNote {
            action: "request_changes",
        };
        assert_eq!(
            err.to_string(),
            "reviewer_note cannot be empty for request_changes"
        );
    }

    #[test]
    fn reviewer_note_too_long_message() {
        let err = LrqError::ReviewerNoteTooLong { actual: 2001 };
        assert_eq!(
            err.to_string(),
            "reviewer_note exceeds 2000 chars (got 2001)"
        );
    }

    #[test]
    fn auto_check_score_out_of_range_message() {
        let err = LrqError::AutoCheckScoreOutOfRange { actual: 101 };
        assert_eq!(err.to_string(), "auto_check_score must be 0-100 (got 101)");
    }
}
