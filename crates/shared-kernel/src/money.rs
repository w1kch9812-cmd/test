//! `KRW` 금액 값 객체.
//!
//! 음수 금지 (0은 허용 — `0` = 무료/미정), `i64` 기반 (KRW 최댓값 ≈ 9.2 quintillion).
//! 산술 연산은 `checked_add` / `checked_sub`로 overflow를 명시적으로 처리해요.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 한국 원화 금액 (`KRW`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct MoneyKrw(i64);

/// `MoneyKrw` 검증 / 산술 에러.
#[derive(Debug, Error)]
pub enum MoneyError {
    /// 음수 금액 (0 미만).
    #[error("money cannot be negative")]
    Negative,
    /// 덧셈 overflow.
    #[error("money addition overflowed")]
    Overflow,
    /// 뺄셈 underflow (결과가 0 미만이거나 `i64` 범위 이탈).
    #[error("money subtraction underflowed")]
    Underflow,
}

impl MoneyKrw {
    /// 음수 검증 후 `MoneyKrw` 생성.
    ///
    /// # Errors
    ///
    /// `krw < 0`이면 [`MoneyError::Negative`].
    pub fn try_new(krw: i64) -> Result<Self, MoneyError> {
        if krw < 0 {
            return Err(MoneyError::Negative);
        }
        Ok(Self(krw))
    }

    /// 내부 `i64` 값.
    #[must_use]
    pub fn as_i64(self) -> i64 {
        self.0
    }

    /// Overflow-safe 덧셈.
    ///
    /// # Errors
    ///
    /// `i64::MAX` 초과 시 [`MoneyError::Overflow`].
    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.0
            .checked_add(other.0)
            .ok_or(MoneyError::Overflow)
            .map(Self)
    }

    /// Underflow-safe 뺄셈.
    ///
    /// # Errors
    ///
    /// 결과가 0 미만이거나 `i64` 범위를 이탈하면 [`MoneyError::Underflow`].
    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        let raw = self.0.checked_sub(other.0).ok_or(MoneyError::Underflow)?;
        if raw < 0 {
            return Err(MoneyError::Underflow);
        }
        Ok(Self(raw))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{MoneyError, MoneyKrw};

    #[test]
    fn from_krw_positive() {
        let m = MoneyKrw::try_new(100_000_000).expect("100M is positive");
        assert_eq!(m.as_i64(), 100_000_000);
    }

    #[test]
    fn zero_is_allowed() {
        let m = MoneyKrw::try_new(0).expect("zero is allowed (free / TBD)");
        assert_eq!(m.as_i64(), 0);
    }

    #[test]
    fn rejects_negative() {
        let err = MoneyKrw::try_new(-1).unwrap_err();
        assert!(matches!(err, MoneyError::Negative));
    }

    #[test]
    fn rejects_min_i64() {
        let err = MoneyKrw::try_new(i64::MIN).unwrap_err();
        assert!(matches!(err, MoneyError::Negative));
    }

    #[test]
    fn add_within_bounds() {
        let a = MoneyKrw::try_new(1_000).expect("ok");
        let b = MoneyKrw::try_new(2_000).expect("ok");
        let sum = a.checked_add(b).expect("no overflow");
        assert_eq!(sum.as_i64(), 3_000);
    }

    #[test]
    fn add_max_plus_zero_is_max() {
        let max = MoneyKrw::try_new(i64::MAX).expect("ok");
        let zero = MoneyKrw::try_new(0).expect("ok");
        let sum = max.checked_add(zero).expect("no overflow");
        assert_eq!(sum.as_i64(), i64::MAX);
    }

    #[test]
    fn add_overflow_returns_err() {
        let max = MoneyKrw::try_new(i64::MAX).expect("ok");
        let one = MoneyKrw::try_new(1).expect("ok");
        let err = max.checked_add(one).unwrap_err();
        assert!(matches!(err, MoneyError::Overflow));
    }

    #[test]
    fn sub_within_bounds() {
        let a = MoneyKrw::try_new(5_000).expect("ok");
        let b = MoneyKrw::try_new(2_000).expect("ok");
        let diff = a.checked_sub(b).expect("no underflow");
        assert_eq!(diff.as_i64(), 3_000);
    }

    #[test]
    fn sub_to_zero_is_ok() {
        let a = MoneyKrw::try_new(1_000).expect("ok");
        let b = MoneyKrw::try_new(1_000).expect("ok");
        let diff = a.checked_sub(b).expect("0 is allowed");
        assert_eq!(diff.as_i64(), 0);
    }

    #[test]
    fn sub_underflow_returns_err() {
        let a = MoneyKrw::try_new(1_000).expect("ok");
        let b = MoneyKrw::try_new(2_000).expect("ok");
        let err = a.checked_sub(b).unwrap_err();
        assert!(matches!(err, MoneyError::Underflow));
    }

    #[test]
    fn ord_works() {
        let a = MoneyKrw::try_new(100).expect("ok");
        let b = MoneyKrw::try_new(200).expect("ok");
        assert!(a < b);
        assert!(b > a);
    }
}
