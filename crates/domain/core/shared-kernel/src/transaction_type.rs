//! `TransactionType` — 매물 거래 유형.
//!
//! Spec § 5.1 listing 테이블 `transaction_type` CHECK enum 3값:
//! `sale` (매매), `monthly_rent` (월세), `jeonse` (전세).
//!
//! `V003_01` cross-field CHECK invariant:
//! - `sale`         → `deposit_krw` NULL,    `monthly_rent_krw` NULL
//! - `monthly_rent` → `deposit_krw` NOT NULL, `monthly_rent_krw` NOT NULL
//! - `jeonse`       → `deposit_krw` NOT NULL, `monthly_rent_krw` NULL

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 거래 유형 (3값).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    /// 매매.
    Sale,
    /// 월세.
    MonthlyRent,
    /// 전세.
    Jeonse,
}

/// `TransactionType` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TransactionTypeError {
    /// 미지원 값.
    #[error("unknown transaction_type: '{0}' (expected: sale, monthly_rent, jeonse)")]
    Unknown(String),
}

impl TransactionType {
    /// 정규화된 `snake_case` 문자열 반환 (`DB varchar(20)` 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Sale => "sale",
            Self::MonthlyRent => "monthly_rent",
            Self::Jeonse => "jeonse",
        }
    }

    /// 거래 유형이 `deposit_krw`를 필수로 요구하는지 (`V003_01` invariant).
    ///
    /// `monthly_rent` + `jeonse` → true. `sale` → false (deposit NULL).
    #[must_use]
    pub const fn requires_deposit(self) -> bool {
        matches!(self, Self::MonthlyRent | Self::Jeonse)
    }

    /// 거래 유형이 `monthly_rent_krw`를 필수로 요구하는지 (`V003_01` invariant).
    ///
    /// `monthly_rent` → true. `sale` + `jeonse` → false (`monthly_rent` NULL).
    #[must_use]
    pub const fn requires_monthly_rent(self) -> bool {
        matches!(self, Self::MonthlyRent)
    }
}

impl fmt::Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TransactionType {
    type Err = TransactionTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "sale" => Ok(Self::Sale),
            "monthly_rent" => Ok(Self::MonthlyRent),
            "jeonse" => Ok(Self::Jeonse),
            other => Err(TransactionTypeError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn as_str_matches_spec_for_each_variant() {
        assert_eq!(TransactionType::Sale.as_str(), "sale");
        assert_eq!(TransactionType::MonthlyRent.as_str(), "monthly_rent");
        assert_eq!(TransactionType::Jeonse.as_str(), "jeonse");
    }

    #[test]
    fn from_str_parses_each_variant() {
        assert_eq!(TransactionType::from_str("sale"), Ok(TransactionType::Sale));
        assert_eq!(
            TransactionType::from_str("monthly_rent"),
            Ok(TransactionType::MonthlyRent)
        );
        assert_eq!(
            TransactionType::from_str("jeonse"),
            Ok(TransactionType::Jeonse)
        );
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = TransactionType::from_str("rent_to_own").unwrap_err();
        assert!(matches!(err, TransactionTypeError::Unknown(s) if s == "rent_to_own"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = TransactionType::from_str("").unwrap_err();
        assert!(matches!(err, TransactionTypeError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", TransactionType::MonthlyRent), "monthly_rent");
    }

    #[test]
    fn round_trip_each_variant() {
        for v in [
            TransactionType::Sale,
            TransactionType::MonthlyRent,
            TransactionType::Jeonse,
        ] {
            assert_eq!(TransactionType::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn serde_roundtrip_via_json() {
        let v = TransactionType::MonthlyRent;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""monthly_rent""#);
        let back: TransactionType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = TransactionType::Sale;
        let b = a; // Copy
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(TransactionType::Sale);
        set.insert(TransactionType::Jeonse);
        assert_eq!(set.len(), 2);
    }

    // ── V003_01 invariant helpers ──────────────────────────────────

    #[test]
    fn requires_deposit_sale_false() {
        assert!(!TransactionType::Sale.requires_deposit());
    }

    #[test]
    fn requires_deposit_monthly_rent_true() {
        assert!(TransactionType::MonthlyRent.requires_deposit());
    }

    #[test]
    fn requires_deposit_jeonse_true() {
        assert!(TransactionType::Jeonse.requires_deposit());
    }

    #[test]
    fn requires_monthly_rent_only_monthly_rent() {
        assert!(!TransactionType::Sale.requires_monthly_rent());
        assert!(TransactionType::MonthlyRent.requires_monthly_rent());
        assert!(!TransactionType::Jeonse.requires_monthly_rent());
    }
}
