//! 면적 값 객체 (`m²`, SI 단위).
//!
//! 양수만 허용 (0 거부 — 면적 0인 필지/매물은 도메인 무효).
//! `NaN`, `±∞` 거부. 표시 단위 환산 (예: 평)은 UI 레이어 책임.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 면적 (`m²`).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AreaM2(f64);

/// `AreaM2` 검증 에러.
#[derive(Debug, Error)]
pub enum AreaError {
    /// `NaN` 또는 `±∞`.
    #[error("area must be finite (no NaN, no ±infinity)")]
    NotFinite,
    /// 양수 아님 (`<= 0`).
    #[error("area must be positive (got {actual})")]
    NonPositive {
        /// 입력 값.
        actual: f64,
    },
}

impl AreaM2 {
    /// 검증 후 `AreaM2` 생성.
    ///
    /// # Errors
    ///
    /// `NaN`/`±∞`이면 [`AreaError::NotFinite`]. `<= 0`이면 [`AreaError::NonPositive`].
    pub const fn try_new(m2: f64) -> Result<Self, AreaError> {
        if !m2.is_finite() {
            return Err(AreaError::NotFinite);
        }
        if m2 <= 0.0 {
            return Err(AreaError::NonPositive { actual: m2 });
        }
        Ok(Self(m2))
    }

    /// 내부 `f64` 값.
    #[must_use]
    pub const fn as_f64(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{AreaError, AreaM2};

    #[test]
    fn from_m2_positive() {
        let a = AreaM2::try_new(99.5).expect("99.5 m² is positive");
        assert!((a.as_f64() - 99.5).abs() < f64::EPSILON);
    }

    #[test]
    fn from_m2_very_small_positive() {
        // 1 m² 같은 작은 값도 허용 (1평 미만 토지 가능).
        let a = AreaM2::try_new(0.5).expect("0.5 is positive");
        assert!((a.as_f64() - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn from_m2_very_large_positive() {
        // 한국 산업단지 최대 면적 ≈ 60 km² = 60_000_000 m² (시화 산업단지).
        let a = AreaM2::try_new(60_000_000.0).expect("60M m² is positive");
        assert!((a.as_f64() - 60_000_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_zero() {
        let err = AreaM2::try_new(0.0).unwrap_err();
        assert!(matches!(err, AreaError::NonPositive { actual } if actual == 0.0));
    }

    #[test]
    fn rejects_negative() {
        let err = AreaM2::try_new(-1.0).unwrap_err();
        assert!(matches!(err, AreaError::NonPositive { .. }));
    }

    #[test]
    fn rejects_nan() {
        let err = AreaM2::try_new(f64::NAN).unwrap_err();
        assert!(matches!(err, AreaError::NotFinite));
    }

    #[test]
    fn rejects_positive_infinity() {
        let err = AreaM2::try_new(f64::INFINITY).unwrap_err();
        assert!(matches!(err, AreaError::NotFinite));
    }

    #[test]
    fn rejects_negative_infinity() {
        let err = AreaM2::try_new(f64::NEG_INFINITY).unwrap_err();
        assert!(matches!(err, AreaError::NotFinite));
    }

    #[test]
    fn partial_ord_works() {
        let a = AreaM2::try_new(100.0).expect("ok");
        let b = AreaM2::try_new(200.0).expect("ok");
        assert!(a < b);
    }
}
