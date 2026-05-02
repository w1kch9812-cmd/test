//! `SystemAlertSeverity` — 시스템 알림 심각도 (4값).
//!
//! Spec § 5.5 `system_alert.severity` `CHECK` enum 4값:
//! `info`, `warning`, `error`, `critical`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 시스템 알림 심각도 (4값, DB `varchar(10)` 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemAlertSeverity {
    /// 단순 정보 (advisory).
    Info,
    /// 경고 — 즉각 조치 불필요하지만 모니터링 필요.
    Warning,
    /// 에러 — 조치 필요.
    Error,
    /// 치명적 — 즉시 조치 필수.
    Critical,
}

impl SystemAlertSeverity {
    /// DB CHECK 제약과 동일한 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
        }
    }

    /// DB 문자열을 enum 으로 파싱. 미지원 값이면 `None`.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    /// 조치가 필요한 심각도인지 — `Error` / `Critical` 만 `true`.
    ///
    /// `Info` / `Warning` 은 모니터링 목적이라 `false`.
    #[must_use]
    pub const fn is_actionable(self) -> bool {
        matches!(self, Self::Error | Self::Critical)
    }
}

impl fmt::Display for SystemAlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_str())
    }
}

/// `SystemAlertSeverity` 파싱 실패 에러.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSystemAlertSeverityError;

impl fmt::Display for ParseSystemAlertSeverityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid system_alert.severity")
    }
}

impl std::error::Error for ParseSystemAlertSeverityError {}

impl FromStr for SystemAlertSeverity {
    type Err = ParseSystemAlertSeverityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s).ok_or(ParseSystemAlertSeverityError)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn round_trip_info() {
        let v = SystemAlertSeverity::Info;
        assert_eq!(v.as_db_str(), "info");
        assert_eq!(SystemAlertSeverity::from_db_str("info"), Some(v));
    }

    #[test]
    fn round_trip_warning() {
        let v = SystemAlertSeverity::Warning;
        assert_eq!(v.as_db_str(), "warning");
        assert_eq!(SystemAlertSeverity::from_db_str("warning"), Some(v));
    }

    #[test]
    fn round_trip_error() {
        let v = SystemAlertSeverity::Error;
        assert_eq!(v.as_db_str(), "error");
        assert_eq!(SystemAlertSeverity::from_db_str("error"), Some(v));
    }

    #[test]
    fn round_trip_critical() {
        let v = SystemAlertSeverity::Critical;
        assert_eq!(v.as_db_str(), "critical");
        assert_eq!(SystemAlertSeverity::from_db_str("critical"), Some(v));
    }

    #[test]
    fn from_db_str_rejects_unknown() {
        assert!(SystemAlertSeverity::from_db_str("INFO").is_none());
        assert!(SystemAlertSeverity::from_db_str("").is_none());
        assert!(SystemAlertSeverity::from_db_str("debug").is_none());
    }

    #[test]
    fn is_actionable_info_false() {
        assert!(!SystemAlertSeverity::Info.is_actionable());
    }

    #[test]
    fn is_actionable_warning_false() {
        assert!(!SystemAlertSeverity::Warning.is_actionable());
    }

    #[test]
    fn is_actionable_error_true() {
        assert!(SystemAlertSeverity::Error.is_actionable());
    }

    #[test]
    fn is_actionable_critical_true() {
        assert!(SystemAlertSeverity::Critical.is_actionable());
    }

    #[test]
    fn display_matches_db_str() {
        assert_eq!(format!("{}", SystemAlertSeverity::Critical), "critical");
        assert_eq!(format!("{}", SystemAlertSeverity::Info), "info");
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = SystemAlertSeverity::Critical;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""critical""#);
        let back: SystemAlertSeverity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn from_str_parses_valid() {
        let v: SystemAlertSeverity = "warning".parse().expect("ok");
        assert_eq!(v, SystemAlertSeverity::Warning);
    }
}
