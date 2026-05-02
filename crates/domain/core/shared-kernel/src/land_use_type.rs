//! `LandUseType` — 한국 토지대장 지목.
//!
//! 한국 표준 28종 中 산업용 부동산 도메인 핵심 9종 + `Other` (포괄).
//! Spec § 8.4 `Parcel.land_use_type` 매핑. R2 정적 데이터에서 fetch.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 지목 (9값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandUseType {
    /// 대 (일반 건축물 부지).
    Building,
    /// 전 (밭).
    Field,
    /// 답 (논).
    Paddy,
    /// 임야.
    Forest,
    /// 공장용지.
    FactorySite,
    /// 창고용지.
    WarehouseSite,
    /// 도로.
    Road,
    /// 공원.
    Park,
    /// 기타 (토지대장 28종 중 위 외).
    Other,
}

/// `LandUseType` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LandUseTypeError {
    /// 미지원 값.
    #[error("unknown land_use_type: '{0}' (expected: building, field, paddy, forest, factory_site, warehouse_site, road, park, other)")]
    Unknown(String),
}

impl LandUseType {
    /// 정규화된 `snake_case` 문자열 (`R2` 데이터 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Building => "building",
            Self::Field => "field",
            Self::Paddy => "paddy",
            Self::Forest => "forest",
            Self::FactorySite => "factory_site",
            Self::WarehouseSite => "warehouse_site",
            Self::Road => "road",
            Self::Park => "park",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for LandUseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LandUseType {
    type Err = LandUseTypeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "building" => Ok(Self::Building),
            "field" => Ok(Self::Field),
            "paddy" => Ok(Self::Paddy),
            "forest" => Ok(Self::Forest),
            "factory_site" => Ok(Self::FactorySite),
            "warehouse_site" => Ok(Self::WarehouseSite),
            "road" => Ok(Self::Road),
            "park" => Ok(Self::Park),
            "other" => Ok(Self::Other),
            other => Err(LandUseTypeError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(LandUseType::Building.as_str(), "building");
        assert_eq!(LandUseType::Field.as_str(), "field");
        assert_eq!(LandUseType::Paddy.as_str(), "paddy");
        assert_eq!(LandUseType::Forest.as_str(), "forest");
        assert_eq!(LandUseType::FactorySite.as_str(), "factory_site");
        assert_eq!(LandUseType::WarehouseSite.as_str(), "warehouse_site");
        assert_eq!(LandUseType::Road.as_str(), "road");
        assert_eq!(LandUseType::Park.as_str(), "park");
        assert_eq!(LandUseType::Other.as_str(), "other");
    }

    #[test]
    fn from_str_parses_each_variant() {
        for v in [
            LandUseType::Building,
            LandUseType::Field,
            LandUseType::Paddy,
            LandUseType::Forest,
            LandUseType::FactorySite,
            LandUseType::WarehouseSite,
            LandUseType::Road,
            LandUseType::Park,
            LandUseType::Other,
        ] {
            assert_eq!(LandUseType::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = LandUseType::from_str("residential").unwrap_err();
        assert!(matches!(err, LandUseTypeError::Unknown(s) if s == "residential"));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", LandUseType::FactorySite), "factory_site");
    }

    #[test]
    fn serde_roundtrip() {
        let v = LandUseType::WarehouseSite;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""warehouse_site""#);
        let back: LandUseType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = LandUseType::Building;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(LandUseType::Building);
        set.insert(LandUseType::Forest);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = LandUseType::from_str("").unwrap_err();
        assert!(matches!(err, LandUseTypeError::Unknown(_)));
    }
}
