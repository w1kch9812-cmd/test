//! `NotificationKind` — 알림 종류 (도메인 enum, type-safe).
//!
//! DB 컬럼은 `varchar(50)` 그대로. 도메인이 enum, DB 가 string. 미지원 라벨이
//! 나타나면 `Other` fallback (forward-compat — 새 kind 가 DB 에 들어와도 panic
//! X).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// 알림 종류.
///
/// SP6-v 1차 = 3 known + Other. 후속 (FU 76+) 에서 추가.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    /// 본인 매물이 admin 에 의해 승인됨 (broker 수신).
    ListingApproved,
    /// 본인 매물이 admin 에 의해 반려됨 (broker 수신, payload 에 reason).
    ListingRejected,
    /// 다른 사용자가 본인 매물 즐겨찾기 (broker 수신).
    ListingBookmarked,
    /// 알 수 없는 / 향후 정의될 종류 (forward-compat fallback).
    Other,
}

impl NotificationKind {
    /// `varchar(50)` DB 컬럼 매핑.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListingApproved => "listing_approved",
            Self::ListingRejected => "listing_rejected",
            Self::ListingBookmarked => "listing_bookmarked",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for NotificationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NotificationKind {
    type Err = std::convert::Infallible;

    /// 미지원 코드 = `Other` (forward-compat — DB 에 새 kind 가 들어와도 panic X).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "listing_approved" => Self::ListingApproved,
            "listing_rejected" => Self::ListingRejected,
            "listing_bookmarked" => Self::ListingBookmarked,
            _ => Self::Other,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(NotificationKind::ListingApproved.as_str(), "listing_approved");
        assert_eq!(NotificationKind::ListingRejected.as_str(), "listing_rejected");
        assert_eq!(
            NotificationKind::ListingBookmarked.as_str(),
            "listing_bookmarked"
        );
        assert_eq!(NotificationKind::Other.as_str(), "other");
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", NotificationKind::ListingApproved), "listing_approved");
    }

    #[test]
    fn from_str_round_trip_known() {
        for v in [
            NotificationKind::ListingApproved,
            NotificationKind::ListingRejected,
            NotificationKind::ListingBookmarked,
            NotificationKind::Other,
        ] {
            let s = v.as_str();
            let back = NotificationKind::from_str(s).unwrap();
            assert_eq!(v, back);
        }
    }

    #[test]
    fn from_str_unknown_falls_back_to_other() {
        // forward-compat — 미지원 코드도 Other 로 흡수 (panic X).
        assert_eq!(
            NotificationKind::from_str("future_kind_we_dont_know").unwrap(),
            NotificationKind::Other
        );
    }

    #[test]
    fn from_str_empty_falls_back_to_other() {
        assert_eq!(
            NotificationKind::from_str("").unwrap(),
            NotificationKind::Other
        );
    }

    #[test]
    fn serde_roundtrip_listing_approved() {
        let v = NotificationKind::ListingApproved;
        let json = serde_json::to_string(&v).expect("ser");
        assert_eq!(json, r#""listing_approved""#);
        let back: NotificationKind = serde_json::from_str(&json).expect("de");
        assert_eq!(v, back);
    }

    #[test]
    fn serde_snake_case_for_compound_variant() {
        let v = NotificationKind::ListingBookmarked;
        let json = serde_json::to_string(&v).expect("ser");
        assert_eq!(json, r#""listing_bookmarked""#);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = NotificationKind::ListingApproved;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(NotificationKind::ListingApproved);
        set.insert(NotificationKind::Other);
        set.insert(NotificationKind::ListingApproved);
        assert_eq!(set.len(), 2);
    }
}
