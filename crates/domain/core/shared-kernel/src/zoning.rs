//! `Zoning` — 한국 국토계획법 용도지역.
//!
//! 4 대분류 + `Other` (관리지역/농림지역/자연환경보전지역 등 포괄).
//! Spec § 8.4 `Parcel.zoning` 매핑.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 용도지역 (5값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Zoning {
    /// 주거지역.
    Residential,
    /// 상업지역.
    Commercial,
    /// 공업지역.
    Industrial,
    /// 녹지지역.
    Green,
    /// 기타 (관리/농림/자연환경보전).
    Other,
}

/// `Zoning` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZoningError {
    /// 미지원 값.
    #[error("unknown zoning: '{0}' (expected: residential, commercial, industrial, green, other)")]
    Unknown(String),
}

impl Zoning {
    /// 정규화된 `snake_case` 문자열.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Residential => "residential",
            Self::Commercial => "commercial",
            Self::Industrial => "industrial",
            Self::Green => "green",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for Zoning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Zoning {
    type Err = ZoningError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "residential" => Ok(Self::Residential),
            "commercial" => Ok(Self::Commercial),
            "industrial" => Ok(Self::Industrial),
            "green" => Ok(Self::Green),
            "other" => Ok(Self::Other),
            other => Err(ZoningError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(Zoning::Residential.as_str(), "residential");
        assert_eq!(Zoning::Commercial.as_str(), "commercial");
        assert_eq!(Zoning::Industrial.as_str(), "industrial");
        assert_eq!(Zoning::Green.as_str(), "green");
        assert_eq!(Zoning::Other.as_str(), "other");
    }

    #[test]
    fn from_str_each_variant() {
        for v in [
            Zoning::Residential,
            Zoning::Commercial,
            Zoning::Industrial,
            Zoning::Green,
            Zoning::Other,
        ] {
            assert_eq!(Zoning::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = Zoning::from_str("agricultural").unwrap_err();
        assert!(matches!(err, ZoningError::Unknown(s) if s == "agricultural"));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", Zoning::Industrial), "industrial");
    }

    #[test]
    fn serde_roundtrip() {
        let v = Zoning::Industrial;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""industrial""#);
        let back: Zoning = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = Zoning::Industrial;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(Zoning::Residential);
        set.insert(Zoning::Industrial);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = Zoning::from_str("").unwrap_err();
        assert!(matches!(err, ZoningError::Unknown(_)));
    }
}
