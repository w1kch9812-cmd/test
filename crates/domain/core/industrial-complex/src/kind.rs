//! `IndustrialComplexKind` — 한국 산업단지 4종.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 산업단지 종류 (4값).
///
/// 「산업입지 및 개발에 관한 법률」제2조의 분류를 따라요.
/// 국가 / 일반 / 도시첨단 / 농공 4종.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndustrialComplexKind {
    /// 국가산업단지.
    National,
    /// 일반산업단지.
    General,
    /// 도시첨단산업단지.
    UrbanHighTech,
    /// 농공단지.
    AgriculturalIndustrial,
}

/// `IndustrialComplexKind` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IndustrialComplexKindError {
    /// 정의되지 않은 코드 문자열.
    #[error("unknown industrial_complex_kind: '{0}'")]
    Unknown(String),
}

impl IndustrialComplexKind {
    /// 정규화된 `snake_case` 문자열 (`R2` 데이터 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::National => "national",
            Self::General => "general",
            Self::UrbanHighTech => "urban_high_tech",
            Self::AgriculturalIndustrial => "agricultural_industrial",
        }
    }
}

impl fmt::Display for IndustrialComplexKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for IndustrialComplexKind {
    type Err = IndustrialComplexKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "national" => Ok(Self::National),
            "general" => Ok(Self::General),
            "urban_high_tech" => Ok(Self::UrbanHighTech),
            "agricultural_industrial" => Ok(Self::AgriculturalIndustrial),
            other => Err(IndustrialComplexKindError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{IndustrialComplexKind, IndustrialComplexKindError};
    use std::str::FromStr;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(IndustrialComplexKind::National.as_str(), "national");
        assert_eq!(IndustrialComplexKind::General.as_str(), "general");
        assert_eq!(
            IndustrialComplexKind::UrbanHighTech.as_str(),
            "urban_high_tech"
        );
        assert_eq!(
            IndustrialComplexKind::AgriculturalIndustrial.as_str(),
            "agricultural_industrial"
        );
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            IndustrialComplexKind::National,
            IndustrialComplexKind::General,
            IndustrialComplexKind::UrbanHighTech,
            IndustrialComplexKind::AgriculturalIndustrial,
        ] {
            assert_eq!(IndustrialComplexKind::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = IndustrialComplexKind::from_str("foreign").unwrap_err();
        assert!(matches!(err, IndustrialComplexKindError::Unknown(s) if s == "foreign"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = IndustrialComplexKind::from_str("").unwrap_err();
        assert!(matches!(err, IndustrialComplexKindError::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn from_str_rejects_uppercase() {
        let err = IndustrialComplexKind::from_str("NATIONAL").unwrap_err();
        assert!(matches!(err, IndustrialComplexKindError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", IndustrialComplexKind::National), "national");
        assert_eq!(
            format!("{}", IndustrialComplexKind::UrbanHighTech),
            "urban_high_tech"
        );
    }

    #[test]
    fn serde_roundtrip() {
        let v = IndustrialComplexKind::National;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""national""#);
        let back: IndustrialComplexKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_snake_case_for_compound_variant() {
        let v = IndustrialComplexKind::UrbanHighTech;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""urban_high_tech""#);
        let v2 = IndustrialComplexKind::AgriculturalIndustrial;
        let json2 = serde_json::to_string(&v2).expect("serialize");
        assert_eq!(json2, r#""agricultural_industrial""#);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = IndustrialComplexKind::National;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(IndustrialComplexKind::National);
        set.insert(IndustrialComplexKind::General);
        set.insert(IndustrialComplexKind::National);
        assert_eq!(set.len(), 2);
    }
}
