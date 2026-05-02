//! `AnalysisReport` 도메인 에러.

use thiserror::Error;

/// `AnalysisReport` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AnalysisReportError {
    /// `title` 빈 (trim 후).
    #[error("title cannot be empty")]
    EmptyTitle,
    /// `title` 200자 초과.
    #[error("title exceeds 200 chars (got {actual})")]
    TitleTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `target_pnus` 빈.
    #[error("target_pnus cannot be empty")]
    EmptyTargetPnus,
    /// `target_pnus` 50개 초과.
    #[error("target_pnus exceeds 50 entries (got {actual})")]
    TooManyTargetPnus {
        /// 실제 개수.
        actual: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_title_message() {
        let err = AnalysisReportError::EmptyTitle;
        assert_eq!(err.to_string(), "title cannot be empty");
    }

    #[test]
    fn title_too_long_message() {
        let err = AnalysisReportError::TitleTooLong { actual: 201 };
        assert_eq!(err.to_string(), "title exceeds 200 chars (got 201)");
    }

    #[test]
    fn empty_target_pnus_message() {
        let err = AnalysisReportError::EmptyTargetPnus;
        assert_eq!(err.to_string(), "target_pnus cannot be empty");
    }

    #[test]
    fn too_many_target_pnus_message() {
        let err = AnalysisReportError::TooManyTargetPnus { actual: 51 };
        assert_eq!(err.to_string(), "target_pnus exceeds 50 entries (got 51)");
    }
}
