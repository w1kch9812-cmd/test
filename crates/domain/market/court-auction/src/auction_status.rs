//! `CourtAuctionStatus` — `CourtAuction` 진행 상태.
//!
//! Market BC 내부 enum. 한국 법원 경매 공개 데이터의 진행 상태 분류예요.
//!
//! 5 변형:
//! - `Upcoming` (예정)
//! - `InProgress` (진행중, 입찰 가능)
//! - `Sold` (낙찰)
//! - `Cancelled` (취하)
//! - `Failed` (유찰)

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `CourtAuction` 진행 상태 (5값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourtAuctionStatus {
    /// 예정 (입찰 시작 전).
    Upcoming,
    /// 진행중 (입찰 가능).
    InProgress,
    /// 낙찰 완료.
    Sold,
    /// 취하 (집행 취소).
    Cancelled,
    /// 유찰 (응찰자 없음).
    Failed,
}

/// `CourtAuctionStatus` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CourtAuctionStatusError {
    /// 미지원 값.
    #[error(
        "unknown court_auction_status: '{0}' \
         (expected: upcoming, in_progress, sold, cancelled, failed)"
    )]
    Unknown(String),
}

impl CourtAuctionStatus {
    /// 정규화된 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Upcoming => "upcoming",
            Self::InProgress => "in_progress",
            Self::Sold => "sold",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    /// 활성 경매 (입찰 가능 또는 예정).
    ///
    /// `Upcoming` 또는 `InProgress`이면 `true`. 지도 필터링 헬퍼예요.
    #[must_use]
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Upcoming | Self::InProgress)
    }
}

impl fmt::Display for CourtAuctionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CourtAuctionStatus {
    type Err = CourtAuctionStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "upcoming" => Ok(Self::Upcoming),
            "in_progress" => Ok(Self::InProgress),
            "sold" => Ok(Self::Sold),
            "cancelled" => Ok(Self::Cancelled),
            "failed" => Ok(Self::Failed),
            other => Err(CourtAuctionStatusError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_each_variant() {
        assert_eq!(CourtAuctionStatus::Upcoming.as_str(), "upcoming");
        assert_eq!(CourtAuctionStatus::InProgress.as_str(), "in_progress");
        assert_eq!(CourtAuctionStatus::Sold.as_str(), "sold");
        assert_eq!(CourtAuctionStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(CourtAuctionStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(
            CourtAuctionStatus::from_str("upcoming"),
            Ok(CourtAuctionStatus::Upcoming)
        );
        assert_eq!(
            CourtAuctionStatus::from_str("in_progress"),
            Ok(CourtAuctionStatus::InProgress)
        );
        assert_eq!(
            CourtAuctionStatus::from_str("sold"),
            Ok(CourtAuctionStatus::Sold)
        );
        assert_eq!(
            CourtAuctionStatus::from_str("cancelled"),
            Ok(CourtAuctionStatus::Cancelled)
        );
        assert_eq!(
            CourtAuctionStatus::from_str("failed"),
            Ok(CourtAuctionStatus::Failed)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = CourtAuctionStatus::from_str("withdrawn").unwrap_err();
        assert!(matches!(err, CourtAuctionStatusError::Unknown(s) if s == "withdrawn"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = CourtAuctionStatus::from_str("").unwrap_err();
        assert!(matches!(err, CourtAuctionStatusError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", CourtAuctionStatus::Upcoming), "upcoming");
        assert_eq!(format!("{}", CourtAuctionStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", CourtAuctionStatus::Sold), "sold");
        assert_eq!(format!("{}", CourtAuctionStatus::Cancelled), "cancelled");
        assert_eq!(format!("{}", CourtAuctionStatus::Failed), "failed");
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            CourtAuctionStatus::Upcoming,
            CourtAuctionStatus::InProgress,
            CourtAuctionStatus::Sold,
            CourtAuctionStatus::Cancelled,
            CourtAuctionStatus::Failed,
        ] {
            assert_eq!(CourtAuctionStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = CourtAuctionStatus::InProgress;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""in_progress""#);
        let back: CourtAuctionStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = CourtAuctionStatus::Sold;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(CourtAuctionStatus::Upcoming);
        set.insert(CourtAuctionStatus::InProgress);
        set.insert(CourtAuctionStatus::Sold);
        set.insert(CourtAuctionStatus::Cancelled);
        set.insert(CourtAuctionStatus::Failed);
        assert_eq!(set.len(), 5);
    }

    #[test]
    fn is_active_returns_true_for_upcoming_and_in_progress() {
        assert!(CourtAuctionStatus::Upcoming.is_active());
        assert!(CourtAuctionStatus::InProgress.is_active());
    }

    #[test]
    fn is_active_returns_false_for_terminal_states() {
        assert!(!CourtAuctionStatus::Sold.is_active());
        assert!(!CourtAuctionStatus::Cancelled.is_active());
        assert!(!CourtAuctionStatus::Failed.is_active());
    }
}
