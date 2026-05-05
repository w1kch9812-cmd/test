//! `HealthStatus` — drift 검출 결과 분류.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API drift 검출 결과 6 분류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// 정상 응답 + parser 통과.
    Success,
    /// HTTP 5xx — 정부 일시 장애 가능 (soft-fail).
    Http5xx,
    /// HTTP 4xx — 키 / quota / endpoint 죽음 (hard-fail).
    Http4xx,
    /// HTTP 200 + parser fail — schema drift (hard-fail, 즉시 escalation).
    ParseFail,
    /// timeout (soft-fail).
    Timeout,
    /// connection 실패 — DNS / SSL / TCP (soft-fail).
    ConnectionFail,
}

/// [`HealthStatus::from_str`] 실패 시 반환되는 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HealthStatusError {
    /// 알려지지 않은 status 문자열.
    #[error("unknown health_status: '{0}'")]
    Unknown(String),
}

impl HealthStatus {
    /// hard-fail 인가? (즉시 escalation 대상)
    #[must_use]
    pub const fn is_hard_fail(self) -> bool {
        matches!(self, Self::Http4xx | Self::ParseFail)
    }

    /// soft-fail 인가? (3일 연속이어야 escalation)
    #[must_use]
    pub const fn is_soft_fail(self) -> bool {
        matches!(self, Self::Http5xx | Self::Timeout | Self::ConnectionFail)
    }

    /// `true` 면 fail 종류, `false` 면 success.
    #[must_use]
    pub const fn is_failure(self) -> bool {
        !matches!(self, Self::Success)
    }

    /// DB / 로그 표기용 정적 문자열.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Http5xx => "http_5xx",
            Self::Http4xx => "http_4xx",
            Self::ParseFail => "parse_fail",
            Self::Timeout => "timeout",
            Self::ConnectionFail => "connection_fail",
        }
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for HealthStatus {
    type Err = HealthStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "http_5xx" => Ok(Self::Http5xx),
            "http_4xx" => Ok(Self::Http4xx),
            "parse_fail" => Ok(Self::ParseFail),
            "timeout" => Ok(Self::Timeout),
            "connection_fail" => Ok(Self::ConnectionFail),
            other => Err(HealthStatusError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn is_hard_fail_only_4xx_and_parse() {
        assert!(HealthStatus::Http4xx.is_hard_fail());
        assert!(HealthStatus::ParseFail.is_hard_fail());
        assert!(!HealthStatus::Http5xx.is_hard_fail());
        assert!(!HealthStatus::Timeout.is_hard_fail());
        assert!(!HealthStatus::ConnectionFail.is_hard_fail());
        assert!(!HealthStatus::Success.is_hard_fail());
    }

    #[test]
    fn is_soft_fail_only_5xx_timeout_connection() {
        assert!(HealthStatus::Http5xx.is_soft_fail());
        assert!(HealthStatus::Timeout.is_soft_fail());
        assert!(HealthStatus::ConnectionFail.is_soft_fail());
        assert!(!HealthStatus::Http4xx.is_soft_fail());
        assert!(!HealthStatus::ParseFail.is_soft_fail());
        assert!(!HealthStatus::Success.is_soft_fail());
    }

    #[test]
    fn is_failure_excludes_success() {
        for v in [
            HealthStatus::Http5xx,
            HealthStatus::Http4xx,
            HealthStatus::ParseFail,
            HealthStatus::Timeout,
            HealthStatus::ConnectionFail,
        ] {
            assert!(v.is_failure(), "{v} should be failure");
        }
        assert!(!HealthStatus::Success.is_failure());
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            HealthStatus::Success,
            HealthStatus::Http5xx,
            HealthStatus::Http4xx,
            HealthStatus::ParseFail,
            HealthStatus::Timeout,
            HealthStatus::ConnectionFail,
        ] {
            assert_eq!(HealthStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = HealthStatus::from_str("teapot").unwrap_err();
        assert!(matches!(err, HealthStatusError::Unknown(s) if s == "teapot"));
    }

    #[test]
    fn serde_roundtrip_snake_case() {
        let v = HealthStatus::ParseFail;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""parse_fail""#);
    }
}
