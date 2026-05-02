//! `CourtAuctionKind` — `CourtAuction` 경매 유형.
//!
//! Market BC 내부 enum. 한국 법원 경매 공개 데이터의 경매 종류 분류예요.
//!
//! 3 변형:
//! - `Forced` (강제경매)
//! - `Voluntary` (임의경매)
//! - `Other` (기타)

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `CourtAuction` 경매 유형 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourtAuctionKind {
    /// 강제경매.
    Forced,
    /// 임의경매.
    Voluntary,
    /// 기타.
    Other,
}

/// `CourtAuctionKind` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CourtAuctionKindError {
    /// 미지원 값.
    #[error("unknown court_auction_kind: '{0}' (expected: forced, voluntary, other)")]
    Unknown(String),
}

impl CourtAuctionKind {
    /// 정규화된 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Forced => "forced",
            Self::Voluntary => "voluntary",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for CourtAuctionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CourtAuctionKind {
    type Err = CourtAuctionKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "forced" => Ok(Self::Forced),
            "voluntary" => Ok(Self::Voluntary),
            "other" => Ok(Self::Other),
            other => Err(CourtAuctionKindError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_each_variant() {
        assert_eq!(CourtAuctionKind::Forced.as_str(), "forced");
        assert_eq!(CourtAuctionKind::Voluntary.as_str(), "voluntary");
        assert_eq!(CourtAuctionKind::Other.as_str(), "other");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(
            CourtAuctionKind::from_str("forced"),
            Ok(CourtAuctionKind::Forced)
        );
        assert_eq!(
            CourtAuctionKind::from_str("voluntary"),
            Ok(CourtAuctionKind::Voluntary)
        );
        assert_eq!(
            CourtAuctionKind::from_str("other"),
            Ok(CourtAuctionKind::Other)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = CourtAuctionKind::from_str("auction").unwrap_err();
        assert!(matches!(err, CourtAuctionKindError::Unknown(s) if s == "auction"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = CourtAuctionKind::from_str("").unwrap_err();
        assert!(matches!(err, CourtAuctionKindError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", CourtAuctionKind::Forced), "forced");
        assert_eq!(format!("{}", CourtAuctionKind::Voluntary), "voluntary");
        assert_eq!(format!("{}", CourtAuctionKind::Other), "other");
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            CourtAuctionKind::Forced,
            CourtAuctionKind::Voluntary,
            CourtAuctionKind::Other,
        ] {
            assert_eq!(CourtAuctionKind::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = CourtAuctionKind::Voluntary;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""voluntary""#);
        let back: CourtAuctionKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = CourtAuctionKind::Forced;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(CourtAuctionKind::Forced);
        set.insert(CourtAuctionKind::Voluntary);
        set.insert(CourtAuctionKind::Other);
        assert_eq!(set.len(), 3);
    }
}
