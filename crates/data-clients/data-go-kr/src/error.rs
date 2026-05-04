//! data.go.kr 클라이언트 에러 타입.

use thiserror::Error;

/// `DataGoKrConfig::from_env` 실패.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    /// 필수 환경변수 미설정.
    #[error("required env var '{0}' not set")]
    MissingEnv(&'static str),
    /// 환경변수 값이 빈 문자열.
    #[error("env var '{0}' is empty")]
    EmptyEnv(&'static str),
}

/// data.go.kr JSON → 도메인 `Building` 변환 실패.
#[derive(Debug, Error)]
pub enum ParseError {
    /// 응답 envelope `resultCode != "00"` — API 가 정상 종료를 알렸으나 실패.
    ///
    /// 일반적으로 인증 실패 (`30`) / quota 초과 (`22`) / 잘못된 파라미터 (`10`)
    /// 등. data.go.kr 표준 코드. 본문은 `(code, msg)` 그대로 보존.
    #[error("data.go.kr API error: code='{code}' msg='{msg}'")]
    ApiError {
        /// data.go.kr `resultCode`.
        code: String,
        /// data.go.kr `resultMsg`.
        msg: String,
    },
    /// JSON 구조가 예상과 다름.
    #[error("malformed data.go.kr response: {0}")]
    Malformed(String),
    /// 도메인 invariant 위반 (PNU 19자리 / 면적 음수 / 한글 코드 매핑 실패 등).
    #[error("domain validation failed: {0}")]
    Domain(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_error_display_matches_format() {
        let e = ConfigError::MissingEnv("ODP_SERVICE_KEY");
        assert_eq!(e.to_string(), "required env var 'ODP_SERVICE_KEY' not set");
        let e = ConfigError::EmptyEnv("ODP_SERVICE_KEY");
        assert_eq!(e.to_string(), "env var 'ODP_SERVICE_KEY' is empty");
    }

    #[test]
    fn parse_error_api_error_carries_code_and_msg() {
        let e = ParseError::ApiError {
            code: "30".to_owned(),
            msg: "SERVICE KEY IS NOT REGISTERED ERROR".to_owned(),
        };
        assert!(e.to_string().contains("code='30'"));
        assert!(e.to_string().contains("msg='SERVICE KEY"));
    }

    #[test]
    fn parse_error_malformed_carries_message() {
        let e = ParseError::Malformed("missing /response/body".to_owned());
        assert_eq!(
            e.to_string(),
            "malformed data.go.kr response: missing /response/body"
        );
    }
}
