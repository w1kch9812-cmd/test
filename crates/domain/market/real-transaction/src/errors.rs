//! `RealTransaction` Reader 에러.

use thiserror::Error;

/// `R2` Reader 에러.
#[derive(Debug, Error)]
pub enum ReaderError {
    /// 데이터 없음.
    #[error("real transaction not found")]
    NotFound,
    /// `R2` fetch 실패.
    #[error("R2 fetch failed: {0}")]
    Fetch(String),
    /// 파싱 실패.
    #[error("R2 data parse failed: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_displays() {
        assert_eq!(
            format!("{}", ReaderError::NotFound),
            "real transaction not found"
        );
    }

    #[test]
    fn fetch_displays() {
        let err = ReaderError::Fetch("timeout".to_owned());
        assert!(format!("{err}").contains("timeout"));
    }

    #[test]
    fn parse_displays() {
        let err = ReaderError::Parse("malformed JSON".to_owned());
        assert!(format!("{err}").contains("malformed"));
    }
}
