//! R2 client 에러 타입.

use thiserror::Error;

/// `R2Config::from_env` 실패.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// 필수 환경변수 미설정.
    #[error("required env var '{0}' not set")]
    MissingEnv(&'static str),
    /// 환경변수 값이 빈 문자열.
    #[error("env var '{0}' is empty")]
    EmptyEnv(&'static str),
}

/// PMTiles / JSON 인덱스 파싱 실패.
#[derive(Debug, Error)]
pub enum ParseError {
    /// PMTiles magic byte 불일치 (`PMTiles` ASCII 7 byte 기대).
    #[error("PMTiles magic mismatch: got {got:?} (expected b\"PMTiles\")")]
    MagicMismatch {
        /// 실제 magic 7 byte.
        got: [u8; 7],
    },
    /// PMTiles version 미지원 (v3 만 지원).
    #[error("unsupported PMTiles version: {0} (only v3 supported)")]
    UnsupportedVersion(u8),
    /// PMTiles header / directory 가 truncated 또는 손상.
    #[error("malformed PMTiles: {0}")]
    Malformed(String),
    /// JSON 인덱스 (예: `pnu_to_buildings.json`) 형식 오류.
    #[error("invalid JSON index: {0}")]
    InvalidIndex(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display_matches_format() {
        let e = ConfigError::MissingEnv("R2_PUBLIC_URL_BASE");
        assert_eq!(
            e.to_string(),
            "required env var 'R2_PUBLIC_URL_BASE' not set"
        );
        let e = ConfigError::EmptyEnv("R2_PUBLIC_URL_BASE");
        assert_eq!(e.to_string(), "env var 'R2_PUBLIC_URL_BASE' is empty");
    }

    #[test]
    fn parse_error_magic_mismatch_carries_got_bytes() {
        let e = ParseError::MagicMismatch {
            got: [b'X', b'Y', b'Z', 0, 0, 0, 0],
        };
        assert!(e.to_string().contains("PMTiles magic mismatch"));
    }

    #[test]
    fn parse_error_unsupported_version_carries_version() {
        let e = ParseError::UnsupportedVersion(2);
        assert!(e.to_string().contains("version: 2"));
    }
}
