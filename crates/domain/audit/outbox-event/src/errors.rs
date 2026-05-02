//! `OutboxEvent` 도메인 에러.

use thiserror::Error;

/// `OutboxEvent` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum OutboxEventError {
    /// `aggregate_kind` 빈 (trim 후).
    #[error("aggregate_kind cannot be empty")]
    EmptyAggregateKind,
    /// `aggregate_kind` 30자 초과.
    #[error("aggregate_kind exceeds 30 chars (got {actual})")]
    AggregateKindTooLong {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_aggregate_kind_message() {
        assert_eq!(
            OutboxEventError::EmptyAggregateKind.to_string(),
            "aggregate_kind cannot be empty"
        );
    }

    #[test]
    fn aggregate_kind_too_long_message() {
        let err = OutboxEventError::AggregateKindTooLong { actual: 31 };
        assert_eq!(err.to_string(), "aggregate_kind exceeds 30 chars (got 31)");
    }

    #[test]
    fn empty_correlation_id_message() {
        assert_eq!(
            OutboxEventError::EmptyCorrelationId.to_string(),
            "correlation_id cannot be empty"
        );
    }

    #[test]
    fn correlation_id_too_long_message() {
        let err = OutboxEventError::CorrelationIdTooLong { actual: 31 };
        assert_eq!(err.to_string(), "correlation_id exceeds 30 chars (got 31)");
    }
}
