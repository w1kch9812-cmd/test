//! `TransactionKind` — `RealTransaction` (과거 거래) 거래 유형.
//!
//! Market BC 내부 enum — `shared_kernel::TransactionType` (Listing/현재 매물)와
//! 의도적으로 분리되어 있어요. 이력 데이터 (`data.go.kr` 실거래가 공개)와 매물
//! 시스템은 진화 경로가 다르므로 BC-internal로 유지해요.
//!
//! 3 변형:
//! - `Sale` (매매)
//! - `Jeonse` (전세)
//! - `MonthlyRent` (월세)

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `RealTransaction` 거래 유형 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionKind {
    /// 매매.
    Sale,
    /// 전세.
    Jeonse,
    /// 월세.
    MonthlyRent,
}

/// `TransactionKind` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionKindError {
    /// 미지원 값.
    #[error("unknown transaction_kind: '{0}' (expected: sale, jeonse, monthly_rent)")]
    Unknown(String),
}

impl TransactionKind {
    /// 정규화된 `snake_case` 문자열 반환.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::Jeonse => "jeonse",
            Self::MonthlyRent => "monthly_rent",
        }
    }
}

impl fmt::Display for TransactionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TransactionKind {
    type Err = TransactionKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sale" => Ok(Self::Sale),
            "jeonse" => Ok(Self::Jeonse),
            "monthly_rent" => Ok(Self::MonthlyRent),
            other => Err(TransactionKindError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_each_variant() {
        assert_eq!(TransactionKind::Sale.as_str(), "sale");
        assert_eq!(TransactionKind::Jeonse.as_str(), "jeonse");
        assert_eq!(TransactionKind::MonthlyRent.as_str(), "monthly_rent");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(TransactionKind::from_str("sale"), Ok(TransactionKind::Sale));
        assert_eq!(
            TransactionKind::from_str("jeonse"),
            Ok(TransactionKind::Jeonse)
        );
        assert_eq!(
            TransactionKind::from_str("monthly_rent"),
            Ok(TransactionKind::MonthlyRent)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = TransactionKind::from_str("rent_to_own").unwrap_err();
        assert!(matches!(err, TransactionKindError::Unknown(s) if s == "rent_to_own"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = TransactionKind::from_str("").unwrap_err();
        assert!(matches!(err, TransactionKindError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", TransactionKind::MonthlyRent), "monthly_rent");
        assert_eq!(format!("{}", TransactionKind::Sale), "sale");
        assert_eq!(format!("{}", TransactionKind::Jeonse), "jeonse");
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            TransactionKind::Sale,
            TransactionKind::Jeonse,
            TransactionKind::MonthlyRent,
        ] {
            assert_eq!(TransactionKind::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = TransactionKind::Jeonse;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""jeonse""#);
        let back: TransactionKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = TransactionKind::Sale;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(TransactionKind::Sale);
        set.insert(TransactionKind::Jeonse);
        set.insert(TransactionKind::MonthlyRent);
        assert_eq!(set.len(), 3);
    }
}
