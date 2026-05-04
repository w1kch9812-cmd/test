//! V-World 클라이언트 에러 타입.

use thiserror::Error;

/// `VWorldConfig::from_env` 실패.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// 필수 환경변수 미설정.
    #[error("required env var '{0}' not set")]
    MissingEnv(&'static str),
    /// 환경변수 값이 빈 문자열.
    #[error("env var '{0}' is empty")]
    EmptyEnv(&'static str),
}

/// V-World JSON → 도메인 `Parcel` 변환 실패.
#[derive(Debug, Error)]
pub enum ParseError {
    /// JSON 구조가 예상과 다름.
    #[error("malformed V-World response: {0}")]
    Malformed(String),
    /// 도메인 invariant 위반 (PNU 19자리 아님 등).
    #[error("domain validation failed: {0}")]
    Domain(String),
}

/// `RawCapture::capture` 실패. 정상 흐름엔 영향 없음 (warn 후 진행).
#[derive(Debug, Error)]
pub enum RawCaptureError {
    /// 저장소 통신 실패.
    #[error("raw capture sink failure: {0}")]
    Sink(String),
}
