//! `Building` Reader 에러.

use thiserror::Error;

/// `R2` Reader 에러.
#[derive(Debug, Error)]
pub enum ReaderError {
    /// 대상 건물 미존재 (`R2` 객체 없음).
    #[error("building not found")]
    NotFound,
    /// `R2` fetch 실패 (네트워크, 권한, 등).
    #[error("R2 fetch failed: {0}")]
    Fetch(String),
    /// `PMTiles` 또는 `JSON` 파싱 실패.
    #[error("R2 data parse failed: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::ReaderError;

    #[test]
    fn not_found_displays() {
        let err = ReaderError::NotFound;
        assert_eq!(format!("{err}"), "building not found");
    }

    #[test]
    fn fetch_displays_with_message() {
        let err = ReaderError::Fetch("connection timeout".to_owned());
        assert_eq!(format!("{err}"), "R2 fetch failed: connection timeout");
    }

    #[test]
    fn parse_displays_with_message() {
        let err = ReaderError::Parse("bad PMTiles header".to_owned());
        assert_eq!(format!("{err}"), "R2 data parse failed: bad PMTiles header");
    }
}
