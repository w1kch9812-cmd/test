//! `ContactVisibility` — 매물 연락처 공개 범위.
//!
//! Spec § 5.1 listing 테이블 `contact_visibility` CHECK enum 3값:
//! `public` (모두 공개), `login_required` (로그인 시 공개, `default`),
//! `verified_only` (검증 사용자만).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 연락처 공개 범위 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactVisibility {
    /// 모두 공개 (비로그인도 볼 수 있음).
    Public,
    /// 로그인 시 공개. `default`.
    LoginRequired,
    /// 검증 사용자만.
    VerifiedOnly,
}

/// `ContactVisibility` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContactVisibilityError {
    /// 미지원 값.
    #[error("unknown contact_visibility: '{0}' (expected: public, login_required, verified_only)")]
    Unknown(String),
}

impl ContactVisibility {
    /// 정규화된 `snake_case` 문자열 (`DB varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::LoginRequired => "login_required",
            Self::VerifiedOnly => "verified_only",
        }
    }
}

impl Default for ContactVisibility {
    /// Spec § 5.1 default — `login_required`.
    fn default() -> Self {
        Self::LoginRequired
    }
}

impl fmt::Display for ContactVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ContactVisibility {
    type Err = ContactVisibilityError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "public" => Ok(Self::Public),
            "login_required" => Ok(Self::LoginRequired),
            "verified_only" => Ok(Self::VerifiedOnly),
            other => Err(ContactVisibilityError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(ContactVisibility::Public.as_str(), "public");
        assert_eq!(ContactVisibility::LoginRequired.as_str(), "login_required");
        assert_eq!(ContactVisibility::VerifiedOnly.as_str(), "verified_only");
    }

    #[test]
    fn from_str_each_variant() {
        assert_eq!(
            ContactVisibility::from_str("public"),
            Ok(ContactVisibility::Public)
        );
        assert_eq!(
            ContactVisibility::from_str("login_required"),
            Ok(ContactVisibility::LoginRequired)
        );
        assert_eq!(
            ContactVisibility::from_str("verified_only"),
            Ok(ContactVisibility::VerifiedOnly)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = ContactVisibility::from_str("private").unwrap_err();
        assert!(matches!(err, ContactVisibilityError::Unknown(s) if s == "private"));
    }

    #[test]
    fn default_is_login_required() {
        assert_eq!(
            ContactVisibility::default(),
            ContactVisibility::LoginRequired
        );
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(
            format!("{}", ContactVisibility::VerifiedOnly),
            "verified_only"
        );
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            ContactVisibility::Public,
            ContactVisibility::LoginRequired,
            ContactVisibility::VerifiedOnly,
        ] {
            assert_eq!(ContactVisibility::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip() {
        let v = ContactVisibility::VerifiedOnly;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""verified_only""#);
        let back: ContactVisibility = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }
}
