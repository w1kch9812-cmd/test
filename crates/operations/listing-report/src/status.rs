//! `ListingReportStatus` — 매물 신고 상태 (4값).
//!
//! Spec § 5.5 `listing_report.status` `CHECK` enum 4값:
//! `open`, `investigating`, `confirmed`, `dismissed`. 기본값 `'open'`.
//!
//! `Confirmed` / `Dismissed` 는 terminal — 이후 모든 전이 시도 거부.

use std::fmt;

use serde::{Deserialize, Serialize};

/// 매물 신고 상태 (4값, DB `varchar(20)` 매핑).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingReportStatus {
    /// 접수 직후 — handler 미배정.
    Open,
    /// 조사 중 — handler 가 배정되어 검토 진행.
    Investigating,
    /// 신고 확정 — terminal. 매물에 대한 후속 조치(블라인드/삭제 등) 별도 진행.
    Confirmed,
    /// 신고 기각 — terminal. 검토 결과 문제 없음.
    Dismissed,
}

impl ListingReportStatus {
    /// DB CHECK 제약과 동일한 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Investigating => "investigating",
            Self::Confirmed => "confirmed",
            Self::Dismissed => "dismissed",
        }
    }

    /// DB 문자열을 enum 으로 파싱. 미지원 값이면 `None`.
    #[must_use]
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(Self::Open),
            "investigating" => Some(Self::Investigating),
            "confirmed" => Some(Self::Confirmed),
            "dismissed" => Some(Self::Dismissed),
            _ => None,
        }
    }

    /// `Confirmed` / `Dismissed` 인지 — terminal 상태 검사.
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Confirmed | Self::Dismissed)
    }

    /// `is_terminal` 의 도메인 의미 별칭 — 신고가 *처리 완료* 상태인지.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        self.is_terminal()
    }
}

impl fmt::Display for ListingReportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_db_str())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_db_str_matches_spec_for_each_variant() {
        assert_eq!(ListingReportStatus::Open.as_db_str(), "open");
        assert_eq!(
            ListingReportStatus::Investigating.as_db_str(),
            "investigating"
        );
        assert_eq!(ListingReportStatus::Confirmed.as_db_str(), "confirmed");
        assert_eq!(ListingReportStatus::Dismissed.as_db_str(), "dismissed");
    }

    #[test]
    fn from_db_str_parses_each_variant() {
        assert_eq!(
            ListingReportStatus::from_db_str("open"),
            Some(ListingReportStatus::Open)
        );
        assert_eq!(
            ListingReportStatus::from_db_str("investigating"),
            Some(ListingReportStatus::Investigating)
        );
        assert_eq!(
            ListingReportStatus::from_db_str("confirmed"),
            Some(ListingReportStatus::Confirmed)
        );
        assert_eq!(
            ListingReportStatus::from_db_str("dismissed"),
            Some(ListingReportStatus::Dismissed)
        );
    }

    #[test]
    fn from_db_str_rejects_unknown() {
        assert_eq!(ListingReportStatus::from_db_str(""), None);
        assert_eq!(ListingReportStatus::from_db_str("OPEN"), None);
        assert_eq!(ListingReportStatus::from_db_str("closed"), None);
    }

    #[test]
    fn is_terminal_open_false() {
        assert!(!ListingReportStatus::Open.is_terminal());
    }

    #[test]
    fn is_terminal_investigating_false() {
        assert!(!ListingReportStatus::Investigating.is_terminal());
    }

    #[test]
    fn is_terminal_confirmed_true() {
        assert!(ListingReportStatus::Confirmed.is_terminal());
    }

    #[test]
    fn is_terminal_dismissed_true() {
        assert!(ListingReportStatus::Dismissed.is_terminal());
    }

    #[test]
    fn is_resolved_aliases_is_terminal() {
        for s in [
            ListingReportStatus::Open,
            ListingReportStatus::Investigating,
            ListingReportStatus::Confirmed,
            ListingReportStatus::Dismissed,
        ] {
            assert_eq!(s.is_resolved(), s.is_terminal());
        }
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            ListingReportStatus::Open,
            ListingReportStatus::Investigating,
            ListingReportStatus::Confirmed,
            ListingReportStatus::Dismissed,
        ] {
            assert_eq!(ListingReportStatus::from_db_str(v.as_db_str()), Some(v));
        }
    }

    #[test]
    fn display_matches_db_str() {
        assert_eq!(
            format!("{}", ListingReportStatus::Investigating),
            "investigating"
        );
        assert_eq!(format!("{}", ListingReportStatus::Confirmed), "confirmed");
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = ListingReportStatus::Investigating;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""investigating""#);
        let back: ListingReportStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }
}
