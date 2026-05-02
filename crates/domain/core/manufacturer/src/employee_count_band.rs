//! `EmployeeCountBand` — `KOSIS` 종사자 수 구간 (6종).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 종사자 수 구간 (6값).
///
/// `KOSIS` (통계청) 사업체 통계 표준 구간을 따라요.
/// 개별 제조업체의 정확한 인원 대신 구간으로 공시 — PII 보호 목적.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmployeeCountBand {
    /// 1-4명.
    OneToFour,
    /// 5-9명.
    FiveToNine,
    /// 10-49명.
    TenToFortyNine,
    /// 50-99명.
    FiftyToNinetyNine,
    /// 100-299명.
    OneHundredToTwoNinetyNine,
    /// 300명 이상.
    ThreeHundredPlus,
}

/// `EmployeeCountBand` 파싱 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EmployeeCountBandError {
    /// 정의되지 않은 코드 문자열.
    #[error("unknown employee_count_band: '{0}'")]
    Unknown(String),
}

impl EmployeeCountBand {
    /// 정규화된 `snake_case` 문자열 (`R2` 데이터 매핑).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OneToFour => "one_to_four",
            Self::FiveToNine => "five_to_nine",
            Self::TenToFortyNine => "ten_to_forty_nine",
            Self::FiftyToNinetyNine => "fifty_to_ninety_nine",
            Self::OneHundredToTwoNinetyNine => "one_hundred_to_two_ninety_nine",
            Self::ThreeHundredPlus => "three_hundred_plus",
        }
    }
}

impl fmt::Display for EmployeeCountBand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EmployeeCountBand {
    type Err = EmployeeCountBandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "one_to_four" => Ok(Self::OneToFour),
            "five_to_nine" => Ok(Self::FiveToNine),
            "ten_to_forty_nine" => Ok(Self::TenToFortyNine),
            "fifty_to_ninety_nine" => Ok(Self::FiftyToNinetyNine),
            "one_hundred_to_two_ninety_nine" => Ok(Self::OneHundredToTwoNinetyNine),
            "three_hundred_plus" => Ok(Self::ThreeHundredPlus),
            other => Err(EmployeeCountBandError::Unknown(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{EmployeeCountBand, EmployeeCountBandError};
    use std::str::FromStr;

    #[test]
    fn as_str_each_variant() {
        assert_eq!(EmployeeCountBand::OneToFour.as_str(), "one_to_four");
        assert_eq!(EmployeeCountBand::FiveToNine.as_str(), "five_to_nine");
        assert_eq!(
            EmployeeCountBand::TenToFortyNine.as_str(),
            "ten_to_forty_nine"
        );
        assert_eq!(
            EmployeeCountBand::FiftyToNinetyNine.as_str(),
            "fifty_to_ninety_nine"
        );
        assert_eq!(
            EmployeeCountBand::OneHundredToTwoNinetyNine.as_str(),
            "one_hundred_to_two_ninety_nine"
        );
        assert_eq!(
            EmployeeCountBand::ThreeHundredPlus.as_str(),
            "three_hundred_plus"
        );
    }

    #[test]
    fn from_str_round_trip_all() {
        for v in [
            EmployeeCountBand::OneToFour,
            EmployeeCountBand::FiveToNine,
            EmployeeCountBand::TenToFortyNine,
            EmployeeCountBand::FiftyToNinetyNine,
            EmployeeCountBand::OneHundredToTwoNinetyNine,
            EmployeeCountBand::ThreeHundredPlus,
        ] {
            assert_eq!(EmployeeCountBand::from_str(v.as_str()).unwrap(), v);
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        let err = EmployeeCountBand::from_str("ten").unwrap_err();
        assert!(matches!(err, EmployeeCountBandError::Unknown(s) if s == "ten"));
    }

    #[test]
    fn from_str_rejects_empty() {
        let err = EmployeeCountBand::from_str("").unwrap_err();
        assert!(matches!(err, EmployeeCountBandError::Unknown(s) if s.is_empty()));
    }

    #[test]
    fn from_str_rejects_uppercase() {
        let err = EmployeeCountBand::from_str("ONE_TO_FOUR").unwrap_err();
        assert!(matches!(err, EmployeeCountBandError::Unknown(_)));
    }

    #[test]
    fn display_matches_as_str() {
        assert_eq!(format!("{}", EmployeeCountBand::OneToFour), "one_to_four");
        assert_eq!(
            format!("{}", EmployeeCountBand::OneHundredToTwoNinetyNine),
            "one_hundred_to_two_ninety_nine"
        );
    }

    #[test]
    fn serde_roundtrip() {
        let v = EmployeeCountBand::TenToFortyNine;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""ten_to_forty_nine""#);
        let back: EmployeeCountBand = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, v);
    }

    #[test]
    fn serde_snake_case_for_compound_variant() {
        let v = EmployeeCountBand::OneHundredToTwoNinetyNine;
        let json = serde_json::to_string(&v).expect("serialize");
        assert_eq!(json, r#""one_hundred_to_two_ninety_nine""#);
        let v2 = EmployeeCountBand::ThreeHundredPlus;
        let json2 = serde_json::to_string(&v2).expect("serialize");
        assert_eq!(json2, r#""three_hundred_plus""#);
    }

    #[test]
    fn copy_and_hash() {
        use std::collections::HashSet;
        let a = EmployeeCountBand::OneToFour;
        let b = a;
        assert_eq!(a, b);
        let mut set = HashSet::new();
        set.insert(EmployeeCountBand::OneToFour);
        set.insert(EmployeeCountBand::FiveToNine);
        set.insert(EmployeeCountBand::OneToFour);
        assert_eq!(set.len(), 2);
    }
}
