//! `FeaturedContentTargetKind` — 추천 콘텐츠 대상 종류 (3값).
//!
//! Spec § 5.5 `featured_content.target_kind` `CHECK` enum 3값:
//! `listing`, `industrial_complex`, `manufacturer`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 추천 콘텐츠 대상 종류 (3값, DB `varchar(30)` 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeaturedContentTargetKind {
    /// 매물 (`listing` 테이블).
    Listing,
    /// 산업단지 (`industrial_complex` 테이블).
    IndustrialComplex,
    /// 제조업체 (`manufacturer` 테이블).
    Manufacturer,
}

impl FeaturedContentTargetKind {
    /// DB CHECK 제약과 동일한 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Listing => "listing",
            Self::IndustrialComplex => "industrial_complex",
            Self::Manufacturer => "manufacturer",
        }
    }

    /// DB 문자열을 enum 으로 파싱. 미지원 값이면 `None`.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "listing" => Some(Self::Listing),
            "industrial_complex" => Some(Self::IndustrialComplex),
            "manufacturer" => Some(Self::Manufacturer),
            _ => None,
        }
    }
}

impl fmt::Display for FeaturedContentTargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_str())
    }
}

/// `FeaturedContentTargetKind` 파싱 실패 에러.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseFeaturedContentTargetKindError;

impl fmt::Display for ParseFeaturedContentTargetKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid featured_content.target_kind")
    }
}

impl std::error::Error for ParseFeaturedContentTargetKindError {}

impl FromStr for FeaturedContentTargetKind {
    type Err = ParseFeaturedContentTargetKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s).ok_or(ParseFeaturedContentTargetKindError)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_db_str_matches_spec_for_each_variant() {
        assert_eq!(FeaturedContentTargetKind::Listing.as_db_str(), "listing");
        assert_eq!(
            FeaturedContentTargetKind::IndustrialComplex.as_db_str(),
            "industrial_complex"
        );
        assert_eq!(
            FeaturedContentTargetKind::Manufacturer.as_db_str(),
            "manufacturer"
        );
    }

    #[test]
    fn round_trip_listing() {
        let v = FeaturedContentTargetKind::Listing;
        assert_eq!(
            FeaturedContentTargetKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn round_trip_industrial_complex() {
        let v = FeaturedContentTargetKind::IndustrialComplex;
        assert_eq!(
            FeaturedContentTargetKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn round_trip_manufacturer() {
        let v = FeaturedContentTargetKind::Manufacturer;
        assert_eq!(
            FeaturedContentTargetKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn from_db_str_rejects_unknown() {
        assert!(FeaturedContentTargetKind::from_db_str("LISTING").is_none());
        assert!(FeaturedContentTargetKind::from_db_str("").is_none());
        assert!(FeaturedContentTargetKind::from_db_str("user").is_none());
    }

    #[test]
    fn display_matches_db_str() {
        assert_eq!(
            format!("{}", FeaturedContentTargetKind::Manufacturer),
            "manufacturer"
        );
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = FeaturedContentTargetKind::IndustrialComplex;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""industrial_complex""#);
        let back: FeaturedContentTargetKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn from_str_parses_valid() {
        let v: FeaturedContentTargetKind = "listing".parse().expect("ok");
        assert_eq!(v, FeaturedContentTargetKind::Listing);
    }

    #[test]
    fn from_str_rejects_invalid() {
        let err = "invalid".parse::<FeaturedContentTargetKind>().unwrap_err();
        assert_eq!(err.to_string(), "invalid featured_content.target_kind");
    }
}
