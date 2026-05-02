//! `SearchHistory` 도메인 에러.

use thiserror::Error;

/// `SearchHistory` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum SearchHistoryError {
    /// `query` 빈.
    #[error("query cannot be empty")]
    EmptyQuery,
    /// `query` 500자 초과.
    #[error("query exceeds 500 chars (got {actual})")]
    QueryTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `correlation_id` 빈.
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
    fn empty_query_message() {
        let err = SearchHistoryError::EmptyQuery;
        assert_eq!(err.to_string(), "query cannot be empty");
    }

    #[test]
    fn query_too_long_message() {
        let err = SearchHistoryError::QueryTooLong { actual: 501 };
        assert_eq!(err.to_string(), "query exceeds 500 chars (got 501)");
    }

    #[test]
    fn empty_correlation_id_message() {
        let err = SearchHistoryError::EmptyCorrelationId;
        assert_eq!(err.to_string(), "correlation_id cannot be empty");
    }

    #[test]
    fn correlation_id_too_long_message() {
        let err = SearchHistoryError::CorrelationIdTooLong { actual: 31 };
        assert_eq!(err.to_string(), "correlation_id exceeds 30 chars (got 31)");
    }
}
