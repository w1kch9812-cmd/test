//! `BookmarkExternalKind` — 외부 `R2` entity 북마크 대상 종류.
//!
//! Spec § 5.2 `bookmark_external.target_kind` `CHECK` enum 4값.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 외부 북마크 대상 종류 (4값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BookmarkExternalKind {
    /// 필지.
    Parcel,
    /// 경매.
    CourtAuction,
    /// 제조업체.
    Manufacturer,
    /// 산업단지.
    IndustrialComplex,
}

impl BookmarkExternalKind {
    /// 정규화된 `snake_case` 문자열.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Parcel => "parcel",
            Self::CourtAuction => "court_auction",
            Self::Manufacturer => "manufacturer",
            Self::IndustrialComplex => "industrial_complex",
        }
    }
}

impl fmt::Display for BookmarkExternalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for BookmarkExternalKind {
    type Err = BookmarkExternalKindError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "parcel" => Ok(Self::Parcel),
            "court_auction" => Ok(Self::CourtAuction),
            "manufacturer" => Ok(Self::Manufacturer),
            "industrial_complex" => Ok(Self::IndustrialComplex),
            other => Err(BookmarkExternalKindError::Unknown(other.to_owned())),
        }
    }
}

/// `BookmarkExternalKind` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BookmarkExternalKindError {
    /// 알 수 없는 `target_kind` 값.
    #[error("unknown bookmark_external target_kind: '{0}'")]
    Unknown(String),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(BookmarkExternalKind::Parcel.as_str(), "parcel");
        assert_eq!(BookmarkExternalKind::CourtAuction.as_str(), "court_auction");
        assert_eq!(BookmarkExternalKind::Manufacturer.as_str(), "manufacturer");
        assert_eq!(
            BookmarkExternalKind::IndustrialComplex.as_str(),
            "industrial_complex"
        );
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            BookmarkExternalKind::Parcel,
            BookmarkExternalKind::CourtAuction,
            BookmarkExternalKind::Manufacturer,
            BookmarkExternalKind::IndustrialComplex,
        ] {
            assert_eq!(BookmarkExternalKind::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = BookmarkExternalKind::from_str("listing").unwrap_err();
        assert!(matches!(err, BookmarkExternalKindError::Unknown(s) if s == "listing"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = BookmarkExternalKind::from_str("").unwrap_err();
        assert!(matches!(err, BookmarkExternalKindError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", BookmarkExternalKind::Parcel), "parcel");
    }

    #[test]
    fn serde_roundtrip() {
        let v = BookmarkExternalKind::CourtAuction;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""court_auction""#);
        let back: BookmarkExternalKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = BookmarkExternalKind::Parcel;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(BookmarkExternalKind::Parcel);
        set.insert(BookmarkExternalKind::Manufacturer);
        assert_eq!(set.len(), 2);
    }
}
