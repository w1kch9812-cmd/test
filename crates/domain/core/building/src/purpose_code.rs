//! `BuildingPurposeCode` — 한국 건축물대장 주용도 (산업용 핵심 10종).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 건물 주용도 (10값).
///
/// 한국 건축물대장 주용도 분류를 산업용 핵심 10종으로 추렸어요.
/// 그 외 분류 (의료/문화/숙박/노유자/...) 는 `Other` 로 매핑해요.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingPurposeCode {
    /// 단독주택.
    SingleHouse,
    /// 공동주택 (다세대 등).
    MultiHouse,
    /// 공장.
    Factory,
    /// 창고.
    Warehouse,
    /// 업무시설.
    Office,
    /// 판매시설.
    Retail,
    /// 지식산업센터.
    KnowledgeIndustryCenter,
    /// 물류시설.
    LogisticsCenter,
    /// 교육연구시설.
    Educational,
    /// 기타 (의료/문화/숙박/노유자/...).
    Other,
}

/// `BuildingPurposeCode` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BuildingPurposeCodeError {
    /// 정의되지 않은 코드 문자열.
    #[error("unknown building_purpose_code: '{0}'")]
    Unknown(String),
}

impl BuildingPurposeCode {
    /// 정규화된 `snake_case` 문자열 (`R2` 데이터 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SingleHouse => "single_house",
            Self::MultiHouse => "multi_house",
            Self::Factory => "factory",
            Self::Warehouse => "warehouse",
            Self::Office => "office",
            Self::Retail => "retail",
            Self::KnowledgeIndustryCenter => "knowledge_industry_center",
            Self::LogisticsCenter => "logistics_center",
            Self::Educational => "educational",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for BuildingPurposeCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BuildingPurposeCode {
    type Err = BuildingPurposeCodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "single_house" => Ok(Self::SingleHouse),
            "multi_house" => Ok(Self::MultiHouse),
            "factory" => Ok(Self::Factory),
            "warehouse" => Ok(Self::Warehouse),
            "office" => Ok(Self::Office),
            "retail" => Ok(Self::Retail),
            "knowledge_industry_center" => Ok(Self::KnowledgeIndustryCenter),
            "logistics_center" => Ok(Self::LogisticsCenter),
            "educational" => Ok(Self::Educational),
            "other" => Ok(Self::Other),
            other => Err(BuildingPurposeCodeError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{BuildingPurposeCode, BuildingPurposeCodeError};
    use std::str::FromStr;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(BuildingPurposeCode::SingleHouse.as_str(), "single_house");
        assert_eq!(BuildingPurposeCode::MultiHouse.as_str(), "multi_house");
        assert_eq!(BuildingPurposeCode::Factory.as_str(), "factory");
        assert_eq!(BuildingPurposeCode::Warehouse.as_str(), "warehouse");
        assert_eq!(BuildingPurposeCode::Office.as_str(), "office");
        assert_eq!(BuildingPurposeCode::Retail.as_str(), "retail");
        assert_eq!(
            BuildingPurposeCode::KnowledgeIndustryCenter.as_str(),
            "knowledge_industry_center"
        );
        assert_eq!(BuildingPurposeCode::LogisticsCenter.as_str(), "logistics_center");
        assert_eq!(BuildingPurposeCode::Educational.as_str(), "educational");
        assert_eq!(BuildingPurposeCode::Other.as_str(), "other");
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            BuildingPurposeCode::SingleHouse,
            BuildingPurposeCode::MultiHouse,
            BuildingPurposeCode::Factory,
            BuildingPurposeCode::Warehouse,
            BuildingPurposeCode::Office,
            BuildingPurposeCode::Retail,
            BuildingPurposeCode::KnowledgeIndustryCenter,
            BuildingPurposeCode::LogisticsCenter,
            BuildingPurposeCode::Educational,
            BuildingPurposeCode::Other,
        ] {
            assert_eq!(BuildingPurposeCode::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = BuildingPurposeCode::from_str("residential").unwrap_err();
        assert!(matches!(err, BuildingPurposeCodeError::Unknown(s) if s == "residential"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = BuildingPurposeCode::from_str("").unwrap_err();
        assert!(matches!(err, BuildingPurposeCodeError::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn from_str_rejects_uppercase() {
        let err = BuildingPurposeCode::from_str("FACTORY").unwrap_err();
        assert!(matches!(err, BuildingPurposeCodeError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", BuildingPurposeCode::Factory), "factory");
        assert_eq!(
            format!("{}", BuildingPurposeCode::KnowledgeIndustryCenter),
            "knowledge_industry_center"
        );
    }

    #[test]
    fn serde_roundtrip() {
        let v = BuildingPurposeCode::Factory;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""factory""#);
        let back: BuildingPurposeCode = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_snake_case_for_compound_variant() {
        let v = BuildingPurposeCode::KnowledgeIndustryCenter;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""knowledge_industry_center""#);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = BuildingPurposeCode::Factory;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(BuildingPurposeCode::Factory);
        set.insert(BuildingPurposeCode::Warehouse);
        set.insert(BuildingPurposeCode::Factory);
        assert_eq!(set.len(), 2);
    }
}
