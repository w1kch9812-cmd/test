//! `ListingType` — 산업용 부동산 매물 유형.
//!
//! Spec § 5.1 `listing` 테이블 `listing_type` `CHECK` enum 6값:
//! `factory`, `warehouse`, `office`, `knowledge_industry_center`,
//! `industrial_land`, `logistics_center`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 매물 유형 (6값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingType {
    /// 공장.
    Factory,
    /// 창고.
    Warehouse,
    /// 사무실.
    Office,
    /// 지식산업센터.
    KnowledgeIndustryCenter,
    /// 산업용지.
    IndustrialLand,
    /// 물류센터.
    LogisticsCenter,
}

/// `ListingType` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingTypeError {
    /// 미지원 값.
    #[error(
        "unknown listing_type: '{0}' (expected: factory, warehouse, office, \
         knowledge_industry_center, industrial_land, logistics_center)"
    )]
    Unknown(String),
}

impl ListingType {
    /// 정규화된 `snake_case` 문자열 반환 (`DB` `varchar(30)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Factory => "factory",
            Self::Warehouse => "warehouse",
            Self::Office => "office",
            Self::KnowledgeIndustryCenter => "knowledge_industry_center",
            Self::IndustrialLand => "industrial_land",
            Self::LogisticsCenter => "logistics_center",
        }
    }
}

impl fmt::Display for ListingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ListingType {
    type Err = ListingTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "factory" => Ok(Self::Factory),
            "warehouse" => Ok(Self::Warehouse),
            "office" => Ok(Self::Office),
            "knowledge_industry_center" => Ok(Self::KnowledgeIndustryCenter),
            "industrial_land" => Ok(Self::IndustrialLand),
            "logistics_center" => Ok(Self::LogisticsCenter),
            other => Err(ListingTypeError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(ListingType::Factory.as_str(), "factory");
        assert_eq!(ListingType::Warehouse.as_str(), "warehouse");
        assert_eq!(ListingType::Office.as_str(), "office");
        assert_eq!(
            ListingType::KnowledgeIndustryCenter.as_str(),
            "knowledge_industry_center"
        );
        assert_eq!(ListingType::IndustrialLand.as_str(), "industrial_land");
        assert_eq!(ListingType::LogisticsCenter.as_str(), "logistics_center");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(ListingType::from_str("factory"), Ok(ListingType::Factory));
        assert_eq!(
            ListingType::from_str("warehouse"),
            Ok(ListingType::Warehouse)
        );
        assert_eq!(ListingType::from_str("office"), Ok(ListingType::Office));
        assert_eq!(
            ListingType::from_str("knowledge_industry_center"),
            Ok(ListingType::KnowledgeIndustryCenter)
        );
        assert_eq!(
            ListingType::from_str("industrial_land"),
            Ok(ListingType::IndustrialLand)
        );
        assert_eq!(
            ListingType::from_str("logistics_center"),
            Ok(ListingType::LogisticsCenter)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = ListingType::from_str("apartment").unwrap_err();
        assert!(matches!(err, ListingTypeError::Unknown(s) if s == "apartment"));
    }

    #[test]
    fn from_str_rejects_camel_case() {
        // `KnowledgeIndustryCenter` 변형은 거부 (`DB`는 `snake_case`).
        let err = ListingType::from_str("KnowledgeIndustryCenter").unwrap_err();
        assert!(matches!(err, ListingTypeError::Unknown(_)));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = ListingType::from_str("").unwrap_err();
        assert!(matches!(err, ListingTypeError::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", ListingType::Factory), "factory");
        assert_eq!(
            format!("{}", ListingType::KnowledgeIndustryCenter),
            "knowledge_industry_center"
        );
    }

    #[test]
    fn round_trip_for_each_variant() {
        for v in [
            ListingType::Factory,
            ListingType::Warehouse,
            ListingType::Office,
            ListingType::KnowledgeIndustryCenter,
            ListingType::IndustrialLand,
            ListingType::LogisticsCenter,
        ] {
            let s = v.as_str();
            let parsed = ListingType::from_str(s).expect("round-trip");
            assert_eq!(parsed, v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = ListingType::KnowledgeIndustryCenter;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""knowledge_industry_center""#);
        let back: ListingType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_semantics() {
        let a = ListingType::Factory;
        let b = a; // `Copy`
        assert_eq!(a, b);
    }

    #[test]
    fn equality_and_hash_distinct_variants() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ListingType::Factory);
        set.insert(ListingType::Warehouse);
        assert_eq!(set.len(), 2);
        // 같은 variant 다시 → 1개
        set.insert(ListingType::Factory);
        assert_eq!(set.len(), 2);
    }
}
