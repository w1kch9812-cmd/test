//! `LrqDecision` — 매물 검토 결정 (3값).
//!
//! Spec § 5.5 `listing_review_queue.decision` `CHECK` enum 3값:
//! `approve`, `reject`, `request_changes`.
//!
//! `decision` 컬럼은 `NULL` 가능 — `NULL` = pending (검토 전).
//! `Some(LrqDecision)` 으로 채워지면 terminal (이후 변경 불가).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 매물 검토 결정 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LrqDecision {
    /// 승인 — 매물 게시 허용.
    Approve,
    /// 거부 — 매물 게시 거부 (메모 필수).
    Reject,
    /// 변경 요청 — 매물 정보 수정 필요 (메모 필수).
    RequestChanges,
}

/// `LrqDecision` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum LrqDecisionError {
    /// 미지원 값.
    #[error("unknown lrq_decision: '{0}' (expected: approve, reject, request_changes)")]
    Unknown(String),
}

impl LrqDecision {
    /// 정규화된 `snake_case` 문자열 반환 (`DB varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Approve => "approve",
            Self::Reject => "reject",
            Self::RequestChanges => "request_changes",
        }
    }
}

impl fmt::Display for LrqDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LrqDecision {
    type Err = LrqDecisionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "approve" => Ok(Self::Approve),
            "reject" => Ok(Self::Reject),
            "request_changes" => Ok(Self::RequestChanges),
            other => Err(LrqDecisionError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(LrqDecision::Approve.as_str(), "approve");
        assert_eq!(LrqDecision::Reject.as_str(), "reject");
        assert_eq!(LrqDecision::RequestChanges.as_str(), "request_changes");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(LrqDecision::from_str("approve"), Ok(LrqDecision::Approve));
        assert_eq!(LrqDecision::from_str("reject"), Ok(LrqDecision::Reject));
        assert_eq!(
            LrqDecision::from_str("request_changes"),
            Ok(LrqDecision::RequestChanges)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = LrqDecision::from_str("approved").unwrap_err();
        assert!(matches!(err, LrqDecisionError::Unknown(s) if s == "approved"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = LrqDecision::from_str("").unwrap_err();
        assert!(matches!(err, LrqDecisionError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(
            format!("{}", LrqDecision::RequestChanges),
            "request_changes"
        );
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            LrqDecision::Approve,
            LrqDecision::Reject,
            LrqDecision::RequestChanges,
        ] {
            assert_eq!(LrqDecision::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = LrqDecision::RequestChanges;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""request_changes""#);
        let back: LrqDecision = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_roundtrip_all_3_variants() {
        for v in [
            LrqDecision::Approve,
            LrqDecision::Reject,
            LrqDecision::RequestChanges,
        ] {
            let json = serde_json::to_string(&v).expect("serialize");
            let back: LrqDecision = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, v);
        }
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = LrqDecision::Approve;
        let b = a; // Copy
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(LrqDecision::Approve);
        set.insert(LrqDecision::Reject);
        assert_eq!(set.len(), 2);
    }
}
