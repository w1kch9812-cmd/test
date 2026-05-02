//! `AdminAction` 도메인 에러.

use thiserror::Error;

/// `AdminAction` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdminActionError {
    /// `action_kind` 빈 (trim 후).
    #[error("action_kind cannot be empty")]
    EmptyActionKind,
    /// `action_kind` 50자 초과.
    #[error("action_kind exceeds 50 chars (got {actual})")]
    ActionKindTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `target_kind` 30자 초과.
    #[error("target_kind exceeds 30 chars (got {actual})")]
    TargetKindTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `target_id` 50자 초과.
    #[error("target_id exceeds 50 chars (got {actual})")]
    TargetIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `target_kind` 와 `target_id` 가 한쪽만 `Some` (도메인 invariant 위반).
    #[error("target_kind and target_id must both be Some or both be None")]
    MismatchedTarget,
    /// `correlation_id` 빈 (trim 후).
    #[error("correlation_id cannot be empty")]
    EmptyCorrelationId,
    /// `correlation_id` 30자 초과.
    #[error("correlation_id exceeds 30 chars (got {actual})")]
    CorrelationIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_action_kind_message() {
        assert_eq!(
            AdminActionError::EmptyActionKind.to_string(),
            "action_kind cannot be empty"
        );
    }

    #[test]
    fn action_kind_too_long_message() {
        let err = AdminActionError::ActionKindTooLong { actual: 51 };
        assert_eq!(err.to_string(), "action_kind exceeds 50 chars (got 51)");
    }

    #[test]
    fn target_kind_too_long_message() {
        let err = AdminActionError::TargetKindTooLong { actual: 31 };
        assert_eq!(err.to_string(), "target_kind exceeds 30 chars (got 31)");
    }

    #[test]
    fn target_id_too_long_message() {
        let err = AdminActionError::TargetIdTooLong { actual: 51 };
        assert_eq!(err.to_string(), "target_id exceeds 50 chars (got 51)");
    }

    #[test]
    fn mismatched_target_message() {
        assert_eq!(
            AdminActionError::MismatchedTarget.to_string(),
            "target_kind and target_id must both be Some or both be None"
        );
    }

    #[test]
    fn empty_correlation_id_message() {
        assert_eq!(
            AdminActionError::EmptyCorrelationId.to_string(),
            "correlation_id cannot be empty"
        );
    }

    #[test]
    fn correlation_id_too_long_message() {
        let err = AdminActionError::CorrelationIdTooLong { actual: 31 };
        assert_eq!(err.to_string(), "correlation_id exceeds 30 chars (got 31)");
    }
}
