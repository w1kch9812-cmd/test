//! `AuditLog` 도메인 에러.

use thiserror::Error;

/// `AuditLog` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuditLogError {
    /// `action` 빈 (trim 후).
    #[error("action cannot be empty")]
    EmptyAction,
    /// `action` 100자 초과.
    #[error("action exceeds 100 chars (got {actual})")]
    ActionTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `resource_kind` 빈 (trim 후).
    #[error("resource_kind cannot be empty")]
    EmptyResourceKind,
    /// `resource_kind` 50자 초과.
    #[error("resource_kind exceeds 50 chars (got {actual})")]
    ResourceKindTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `resource_id` 빈 (trim 후).
    #[error("resource_id cannot be empty")]
    EmptyResourceId,
    /// `resource_id` 50자 초과.
    #[error("resource_id exceeds 50 chars (got {actual})")]
    ResourceIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `correlation_id` 빈 (trim 후).
    #[error("correlation_id cannot be empty")]
    EmptyCorrelationId,
    /// `correlation_id` 30자 초과.
    #[error("correlation_id exceeds 30 chars (got {actual})")]
    CorrelationIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `user_agent` 500자 초과.
    #[error("user_agent exceeds 500 chars (got {actual})")]
    UserAgentTooLong {
        /// 실제 길이.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_action_message() {
        assert_eq!(
            AuditLogError::EmptyAction.to_string(),
            "action cannot be empty"
        );
    }

    #[test]
    fn action_too_long_message() {
        let err = AuditLogError::ActionTooLong { actual: 101 };
        assert_eq!(err.to_string(), "action exceeds 100 chars (got 101)");
    }

    #[test]
    fn empty_resource_kind_message() {
        assert_eq!(
            AuditLogError::EmptyResourceKind.to_string(),
            "resource_kind cannot be empty"
        );
    }

    #[test]
    fn resource_kind_too_long_message() {
        let err = AuditLogError::ResourceKindTooLong { actual: 51 };
        assert_eq!(err.to_string(), "resource_kind exceeds 50 chars (got 51)");
    }

    #[test]
    fn empty_resource_id_message() {
        assert_eq!(
            AuditLogError::EmptyResourceId.to_string(),
            "resource_id cannot be empty"
        );
    }

    #[test]
    fn resource_id_too_long_message() {
        let err = AuditLogError::ResourceIdTooLong { actual: 51 };
        assert_eq!(err.to_string(), "resource_id exceeds 50 chars (got 51)");
    }

    #[test]
    fn empty_correlation_id_message() {
        assert_eq!(
            AuditLogError::EmptyCorrelationId.to_string(),
            "correlation_id cannot be empty"
        );
    }

    #[test]
    fn correlation_id_too_long_message() {
        let err = AuditLogError::CorrelationIdTooLong { actual: 31 };
        assert_eq!(err.to_string(), "correlation_id exceeds 30 chars (got 31)");
    }

    #[test]
    fn user_agent_too_long_message() {
        let err = AuditLogError::UserAgentTooLong { actual: 501 };
        assert_eq!(err.to_string(), "user_agent exceeds 500 chars (got 501)");
    }
}
