//! 공인중개사 자격번호 (`BrokerLicense`) 값 객체.
//!
//! 형식이 시도별로 다양해 (`11-2024-12345`, `26-2024-A12` 등) 표준 강제는 어렵게 해요.
//! 본 값 객체는 *빈 문자열 거부 + 길이 ≤ 50자* 만 검증해요.
//! 깊은 형식 검증은 aggregate(`User`) 또는 외부 검증 API(공인중개사협회)가 책임.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 한국 공인중개사 자격번호.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BrokerLicense(String);

/// `BrokerLicense` 검증 에러.
#[derive(Debug, Error)]
pub enum BrokerLicenseError {
    /// 빈 문자열 (또는 공백만).
    #[error("broker license cannot be empty")]
    Empty,
    /// 길이 50자 초과 (DB `varchar(50)` 매핑).
    #[error("broker license exceeds 50 chars (got {actual})")]
    TooLong {
        /// 실제 길이 (trim 후).
        actual: usize,
    },
}

impl BrokerLicense {
    /// 빈 문자열·길이 검증 후 `BrokerLicense` 생성.
    ///
    /// 앞뒤 공백은 자동 trim. 내부 공백·하이픈 등은 보존.
    ///
    /// # Errors
    ///
    /// trim 후 빈 문자열이면 [`BrokerLicenseError::Empty`].
    /// 50자 초과면 [`BrokerLicenseError::TooLong`].
    pub fn try_new(s: &str) -> Result<Self, BrokerLicenseError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(BrokerLicenseError::Empty);
        }
        if trimmed.len() > 50 {
            return Err(BrokerLicenseError::TooLong {
                actual: trimmed.len(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// 내부 자격번호 문자열 (trim 적용됨).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for BrokerLicense {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for BrokerLicense {
    type Err = BrokerLicenseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_typical_format() {
        let bl = BrokerLicense::try_new("11-2024-12345").expect("valid");
        assert_eq!(bl.as_str(), "11-2024-12345");
    }

    #[test]
    fn parse_alphanumeric_format() {
        // 시도별로 영숫자 혼용도 가능
        let bl = BrokerLicense::try_new("26-2024-A12").expect("valid");
        assert_eq!(bl.as_str(), "26-2024-A12");
    }

    #[test]
    fn trim_leading_trailing_whitespace() {
        let bl = BrokerLicense::try_new("  11-2024-12345  ").expect("trim");
        assert_eq!(bl.as_str(), "11-2024-12345");
    }

    #[test]
    fn rejects_empty() {
        let err = BrokerLicense::try_new("").unwrap_err();
        assert!(matches!(err, BrokerLicenseError::Empty));
    }

    #[test]
    fn rejects_whitespace_only() {
        let err = BrokerLicense::try_new("   ").unwrap_err();
        assert!(matches!(err, BrokerLicenseError::Empty));
    }

    #[test]
    fn rejects_too_long() {
        let s = "X".repeat(51);
        let err = BrokerLicense::try_new(&s).unwrap_err();
        assert!(matches!(err, BrokerLicenseError::TooLong { actual: 51 }));
    }

    #[test]
    fn accepts_exactly_50_chars() {
        let s = "X".repeat(50);
        let bl = BrokerLicense::try_new(&s).expect("50 is allowed");
        assert_eq!(bl.as_str().len(), 50);
    }

    #[test]
    fn display_round_trips() {
        use std::fmt::Write;
        let bl = BrokerLicense::try_new("11-2024-12345").expect("ok");
        let mut out = String::new();
        write!(out, "{bl}").expect("write");
        assert_eq!(out, "11-2024-12345");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        let bl = BrokerLicense::from_str("11-2024-12345").expect("ok");
        assert_eq!(bl.as_str(), "11-2024-12345");
    }
}
