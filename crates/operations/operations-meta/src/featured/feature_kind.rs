//! `FeaturedContentFeatureKind` — 노출 슬롯 종류 (4값).
//!
//! Spec § 5.5 `featured_content.feature_kind` `CHECK` enum 4값:
//! `homepage_featured`, `search_top`, `sponsored_marker`, `newsletter`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 노출 슬롯 종류 (4값, DB `varchar(30)` 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeaturedContentFeatureKind {
    /// 홈페이지 추천 슬롯.
    HomepageFeatured,
    /// 검색 결과 상단 노출.
    SearchTop,
    /// 지도 스폰서 마커.
    SponsoredMarker,
    /// 뉴스레터 노출.
    Newsletter,
}

impl FeaturedContentFeatureKind {
    /// DB CHECK 제약과 동일한 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::HomepageFeatured => "homepage_featured",
            Self::SearchTop => "search_top",
            Self::SponsoredMarker => "sponsored_marker",
            Self::Newsletter => "newsletter",
        }
    }

    /// DB 문자열을 enum 으로 파싱. 미지원 값이면 `None`.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "homepage_featured" => Some(Self::HomepageFeatured),
            "search_top" => Some(Self::SearchTop),
            "sponsored_marker" => Some(Self::SponsoredMarker),
            "newsletter" => Some(Self::Newsletter),
            _ => None,
        }
    }
}

impl fmt::Display for FeaturedContentFeatureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_str())
    }
}

/// `FeaturedContentFeatureKind` 파싱 실패 에러.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseFeaturedContentFeatureKindError;

impl fmt::Display for ParseFeaturedContentFeatureKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid featured_content.feature_kind")
    }
}

impl std::error::Error for ParseFeaturedContentFeatureKindError {}

impl FromStr for FeaturedContentFeatureKind {
    type Err = ParseFeaturedContentFeatureKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_db_str(s).ok_or(ParseFeaturedContentFeatureKindError)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_db_str_matches_spec_for_each_variant() {
        assert_eq!(
            FeaturedContentFeatureKind::HomepageFeatured.as_db_str(),
            "homepage_featured"
        );
        assert_eq!(
            FeaturedContentFeatureKind::SearchTop.as_db_str(),
            "search_top"
        );
        assert_eq!(
            FeaturedContentFeatureKind::SponsoredMarker.as_db_str(),
            "sponsored_marker"
        );
        assert_eq!(
            FeaturedContentFeatureKind::Newsletter.as_db_str(),
            "newsletter"
        );
    }

    #[test]
    fn round_trip_homepage_featured() {
        let v = FeaturedContentFeatureKind::HomepageFeatured;
        assert_eq!(
            FeaturedContentFeatureKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn round_trip_search_top() {
        let v = FeaturedContentFeatureKind::SearchTop;
        assert_eq!(
            FeaturedContentFeatureKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn round_trip_sponsored_marker() {
        let v = FeaturedContentFeatureKind::SponsoredMarker;
        assert_eq!(
            FeaturedContentFeatureKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn round_trip_newsletter() {
        let v = FeaturedContentFeatureKind::Newsletter;
        assert_eq!(
            FeaturedContentFeatureKind::from_db_str(v.as_db_str()),
            Some(v)
        );
    }

    #[test]
    fn from_db_str_rejects_unknown() {
        assert!(FeaturedContentFeatureKind::from_db_str("featured").is_none());
        assert!(FeaturedContentFeatureKind::from_db_str("").is_none());
    }

    #[test]
    fn display_matches_db_str() {
        assert_eq!(
            format!("{}", FeaturedContentFeatureKind::SearchTop),
            "search_top"
        );
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = FeaturedContentFeatureKind::Newsletter;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""newsletter""#);
        let back: FeaturedContentFeatureKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn from_str_parses_valid() {
        let v: FeaturedContentFeatureKind = "homepage_featured".parse().expect("ok");
        assert_eq!(v, FeaturedContentFeatureKind::HomepageFeatured);
    }
}
