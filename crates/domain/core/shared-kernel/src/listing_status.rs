//! `ListingStatus` — 매물 상태 + 상태 전이 머신.
//!
//! Spec § 5.1 listing 테이블 `status` CHECK enum 6값:
//! `draft`, `pending_review`, `active`, `sold`, `expired`, `rejected`.
//!
//! Spec § 8.3 상태 전이 규칙은 `can_transition_to`에 인코딩.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 매물 상태 (6값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListingStatus {
    /// 작성 중. 소유자만 볼 수 있어요.
    Draft,
    /// 어드민 검토 대기.
    PendingReview,
    /// 공개 중.
    Active,
    /// 판매 완료 (terminal).
    Sold,
    /// 만료 (terminal).
    Expired,
    /// 검토 거부됨. 수정 후 `Draft`로 되돌릴 수 있어요.
    Rejected,
}

/// `ListingStatus` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingStatusError {
    /// 미지원 값.
    #[error(
        "unknown listing_status: '{0}' (expected: draft, pending_review, active, sold, expired, rejected)"
    )]
    Unknown(String),
}

impl ListingStatus {
    /// 정규화된 `snake_case` 문자열 반환 (`DB varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Active => "active",
            Self::Sold => "sold",
            Self::Expired => "expired",
            Self::Rejected => "rejected",
        }
    }

    /// 상태 전이가 허용되는지 검사 (spec § 8.3 상태 머신).
    ///
    /// 허용 전이 6개:
    /// - `Draft` → `PendingReview`
    /// - `PendingReview` → `Active`
    /// - `PendingReview` → `Rejected`
    /// - `Active` → `Sold`
    /// - `Active` → `Expired`
    /// - `Rejected` → `Draft` (사용자 수정 후 재제출)
    ///
    /// `Sold`/`Expired`는 terminal — 어떤 전이도 허용 안 해요.
    /// 같은 상태로의 전이는 항상 false.
    #[must_use]
    pub const fn can_transition_to(self, target: Self) -> bool {
        use ListingStatus::{Active, Draft, Expired, PendingReview, Rejected, Sold};
        matches!(
            (self, target),
            (Draft, PendingReview)
                | (PendingReview, Active)
                | (PendingReview, Rejected)
                | (Active, Sold)
                | (Active, Expired)
                | (Rejected, Draft)
        )
    }
}

impl fmt::Display for ListingStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ListingStatus {
    type Err = ListingStatusError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "pending_review" => Ok(Self::PendingReview),
            "active" => Ok(Self::Active),
            "sold" => Ok(Self::Sold),
            "expired" => Ok(Self::Expired),
            "rejected" => Ok(Self::Rejected),
            other => Err(ListingStatusError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(ListingStatus::Draft.as_str(), "draft");
        assert_eq!(ListingStatus::PendingReview.as_str(), "pending_review");
        assert_eq!(ListingStatus::Active.as_str(), "active");
        assert_eq!(ListingStatus::Sold.as_str(), "sold");
        assert_eq!(ListingStatus::Expired.as_str(), "expired");
        assert_eq!(ListingStatus::Rejected.as_str(), "rejected");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(ListingStatus::from_str("draft"), Ok(ListingStatus::Draft));
        assert_eq!(
            ListingStatus::from_str("pending_review"),
            Ok(ListingStatus::PendingReview)
        );
        assert_eq!(ListingStatus::from_str("active"), Ok(ListingStatus::Active));
        assert_eq!(ListingStatus::from_str("sold"), Ok(ListingStatus::Sold));
        assert_eq!(
            ListingStatus::from_str("expired"),
            Ok(ListingStatus::Expired)
        );
        assert_eq!(
            ListingStatus::from_str("rejected"),
            Ok(ListingStatus::Rejected)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = ListingStatus::from_str("archived").unwrap_err();
        assert!(matches!(err, ListingStatusError::Unknown(s) if s == "archived"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = ListingStatus::from_str("").unwrap_err();
        assert!(matches!(err, ListingStatusError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(
            format!("{}", ListingStatus::PendingReview),
            "pending_review"
        );
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            ListingStatus::Draft,
            ListingStatus::PendingReview,
            ListingStatus::Active,
            ListingStatus::Sold,
            ListingStatus::Expired,
            ListingStatus::Rejected,
        ] {
            assert_eq!(ListingStatus::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = ListingStatus::PendingReview;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""pending_review""#);
        let back: ListingStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = ListingStatus::Active;
        let b = a; // Copy
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(ListingStatus::Draft);
        set.insert(ListingStatus::Active);
        assert_eq!(set.len(), 2);
    }

    // ── State machine: ALLOWED transitions ─────────────────────────

    #[test]
    fn allowed_draft_to_pending_review() {
        assert!(ListingStatus::Draft.can_transition_to(ListingStatus::PendingReview));
    }

    #[test]
    fn allowed_pending_to_active() {
        assert!(ListingStatus::PendingReview.can_transition_to(ListingStatus::Active));
    }

    #[test]
    fn allowed_pending_to_rejected() {
        assert!(ListingStatus::PendingReview.can_transition_to(ListingStatus::Rejected));
    }

    #[test]
    fn allowed_active_to_sold() {
        assert!(ListingStatus::Active.can_transition_to(ListingStatus::Sold));
    }

    #[test]
    fn allowed_active_to_expired() {
        assert!(ListingStatus::Active.can_transition_to(ListingStatus::Expired));
    }

    #[test]
    fn allowed_rejected_to_draft() {
        assert!(ListingStatus::Rejected.can_transition_to(ListingStatus::Draft));
    }

    // ── State machine: DISALLOWED transitions ──────────────────────

    #[test]
    fn disallowed_sold_terminal() {
        for target in [
            ListingStatus::Draft,
            ListingStatus::PendingReview,
            ListingStatus::Active,
            ListingStatus::Sold,
            ListingStatus::Expired,
            ListingStatus::Rejected,
        ] {
            assert!(
                !ListingStatus::Sold.can_transition_to(target),
                "Sold should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn disallowed_expired_terminal() {
        for target in [
            ListingStatus::Draft,
            ListingStatus::PendingReview,
            ListingStatus::Active,
            ListingStatus::Sold,
            ListingStatus::Expired,
            ListingStatus::Rejected,
        ] {
            assert!(
                !ListingStatus::Expired.can_transition_to(target),
                "Expired should not transition to {target:?}"
            );
        }
    }

    #[test]
    fn disallowed_draft_skip_review() {
        assert!(!ListingStatus::Draft.can_transition_to(ListingStatus::Active));
        assert!(!ListingStatus::Draft.can_transition_to(ListingStatus::Rejected));
        assert!(!ListingStatus::Draft.can_transition_to(ListingStatus::Sold));
        assert!(!ListingStatus::Draft.can_transition_to(ListingStatus::Expired));
    }

    #[test]
    fn disallowed_active_rollback() {
        assert!(!ListingStatus::Active.can_transition_to(ListingStatus::Draft));
        assert!(!ListingStatus::Active.can_transition_to(ListingStatus::PendingReview));
        assert!(!ListingStatus::Active.can_transition_to(ListingStatus::Rejected));
    }

    #[test]
    fn disallowed_pending_skip_active() {
        assert!(!ListingStatus::PendingReview.can_transition_to(ListingStatus::Sold));
        assert!(!ListingStatus::PendingReview.can_transition_to(ListingStatus::Expired));
        assert!(!ListingStatus::PendingReview.can_transition_to(ListingStatus::Draft));
    }

    #[test]
    fn disallowed_self_transition_for_each() {
        for v in [
            ListingStatus::Draft,
            ListingStatus::PendingReview,
            ListingStatus::Active,
            ListingStatus::Sold,
            ListingStatus::Expired,
            ListingStatus::Rejected,
        ] {
            assert!(
                !v.can_transition_to(v),
                "{v:?} → {v:?} should not be allowed"
            );
        }
    }

    #[test]
    fn disallowed_rejected_to_anything_except_draft() {
        assert!(!ListingStatus::Rejected.can_transition_to(ListingStatus::PendingReview));
        assert!(!ListingStatus::Rejected.can_transition_to(ListingStatus::Active));
        assert!(!ListingStatus::Rejected.can_transition_to(ListingStatus::Sold));
        assert!(!ListingStatus::Rejected.can_transition_to(ListingStatus::Expired));
    }
}
