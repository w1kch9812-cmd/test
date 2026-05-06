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
    /// JSON 구조가 예상과 다름 (envelope 또는 layer property).
    #[error("malformed V-World response: {0}")]
    Malformed(String),
    /// 도메인 invariant 위반 (PNU 19자리 아님 등).
    #[error("domain validation failed: {0}")]
    Domain(String),
    /// V-World 가 `status: "ERROR"` 로 응답 — API 측 에러.
    ///
    /// 흔한 코드: `INVALID_RANGE` (잘못된 attrFilter 속성), `INVALID_KEY`
    /// (키/도메인 검증 실패), `NO_PERMISSION` (해당 레이어 권한 없음).
    #[error("V-World API error: code={code}, text={text}")]
    VWorldApi {
        /// V-World 에러 코드 (예: `INVALID_RANGE`).
        code: String,
        /// 한국어 에러 메시지.
        text: String,
    },
}

// `RawCaptureError` 는 SP4-iii-d 에서 raw-capture-client crate 로 이동.
// 본 crate 의 lib.rs 가 re-export 해 호환성 유지.
