//! 한국 행정구역 코드 (행정안전부 표준).
//!
//! 3개 계층:
//! - `SidoCode` — 2자리 (예: `11` = 서울특별시)
//! - `SigunguCode` — 5자리 (예: `11110` = 서울 종로구). 시도 코드 포함.
//! - `EupmyeondongCode` — 8자리 (예: `11110101` = 서울 종로구 청운효자동). 시군구 + 읍면동.
//!
//! 모든 코드는 `ASCII` 숫자만 포함하고 길이가 정확해야 해요.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Sido ──────────────────────────────────────────────────────────────

/// 시도 코드 (2자리).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SidoCode(String);

impl SidoCode {
    /// 검증 후 `SidoCode` 생성.
    ///
    /// # Errors
    ///
    /// 길이 ≠ 2 → `InvalidLength`. 숫자 외 → `NonDigit`.
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 2)?;
        Ok(Self(s.to_owned()))
    }

    /// 내부 2자리 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── Sigungu ───────────────────────────────────────────────────────────

/// 시군구 코드 (5자리, 시도 + 시군구).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SigunguCode(String);

impl SigunguCode {
    /// 검증 후 `SigunguCode` 생성.
    ///
    /// # Errors
    ///
    /// 길이 ≠ 5 → `InvalidLength`. 숫자 외 → `NonDigit`.
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 5)?;
        Ok(Self(s.to_owned()))
    }

    /// 내부 5자리 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 시도 코드 추출 (앞 2자리).
    ///
    /// 검증 통과한 `SigunguCode`에서만 호출되므로 항상 성공해요.
    #[must_use]
    pub fn sido_code(&self) -> SidoCode {
        SidoCode(self.0[..2].to_owned())
    }
}

// ── Eupmyeondong ──────────────────────────────────────────────────────

/// 읍면동 코드 (8자리, 시도 + 시군구 + 읍면동).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EupmyeondongCode(String);

impl EupmyeondongCode {
    /// 검증 후 `EupmyeondongCode` 생성.
    ///
    /// # Errors
    ///
    /// 길이 ≠ 8 → `InvalidLength`. 숫자 외 → `NonDigit`.
    pub fn try_new(s: &str) -> Result<Self, AdminDivisionError> {
        validate_digits(s, 8)?;
        Ok(Self(s.to_owned()))
    }

    /// 내부 8자리 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 시도 코드 추출 (앞 2자리).
    #[must_use]
    pub fn sido_code(&self) -> SidoCode {
        SidoCode(self.0[..2].to_owned())
    }

    /// 시군구 코드 추출 (앞 5자리).
    #[must_use]
    pub fn sigungu_code(&self) -> SigunguCode {
        SigunguCode(self.0[..5].to_owned())
    }
}

// ── Error + validator ─────────────────────────────────────────────────

/// 행정구역 코드 검증 에러.
#[derive(Debug, Error)]
pub enum AdminDivisionError {
    /// 길이 불일치.
    #[error("expected {expected} digits, got {actual}")]
    InvalidLength {
        /// 기대 길이.
        expected: usize,
        /// 실제 길이.
        actual: usize,
    },
    /// `ASCII` 숫자 아닌 문자 포함.
    #[error("admin division code must be ASCII digits only")]
    NonDigit,
    /// `sigungu` 첫 2자리가 `sido`와 다름.
    #[error("sigungu prefix mismatch: sido={sido}, sigungu={sigungu}")]
    SidoSigunguMismatch {
        /// 입력 sido.
        sido: String,
        /// 입력 sigungu.
        sigungu: String,
    },
    /// `eupmyeondong` 첫 5자리가 `sigungu`와 다름.
    #[error("eupmyeondong prefix mismatch: sigungu={sigungu}, eupmyeondong={eupmyeondong}")]
    SigunguEupmyeondongMismatch {
        /// 입력 sigungu.
        sigungu: String,
        /// 입력 eupmyeondong.
        eupmyeondong: String,
    },
}

fn validate_digits(s: &str, expected: usize) -> Result<(), AdminDivisionError> {
    if s.len() != expected {
        return Err(AdminDivisionError::InvalidLength {
            expected,
            actual: s.len(),
        });
    }
    if !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(AdminDivisionError::NonDigit);
    }
    Ok(())
}

// ── Display + FromStr (3 types × 2 = 6 impls) ─────────────────────────

impl std::fmt::Display for SidoCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::str::FromStr for SidoCode {
    type Err = AdminDivisionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl std::fmt::Display for SigunguCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::str::FromStr for SigunguCode {
    type Err = AdminDivisionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl std::fmt::Display for EupmyeondongCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::str::FromStr for EupmyeondongCode {
    type Err = AdminDivisionError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

// ── AdminDivision composite ───────────────────────────────────────────

/// 행정구역 composite (시도 + 시군구 + 읍면동 일관성 강제).
///
/// 단일 newtype 아님 — 3 코드를 묶어서 cross-field invariant를 강제해요:
/// - `sigungu`의 첫 2자리 == `sido`
/// - `eupmyeondong`의 첫 5자리 == `sigungu`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdminDivision {
    /// 시도 코드 (2자리).
    pub sido: SidoCode,
    /// 시군구 코드 (5자리).
    pub sigungu: SigunguCode,
    /// 읍면동 코드 (8자리).
    pub eupmyeondong: EupmyeondongCode,
}

impl AdminDivision {
    /// 검증 후 `AdminDivision` 생성.
    ///
    /// # Errors
    ///
    /// `sigungu`의 첫 2자리가 `sido`와 다르면`SidoSigunguMismatch`.
    /// `eupmyeondong`의 첫 5자리가 `sigungu`와 다르면 `SigunguEupmyeondongMismatch`.
    pub fn try_new(
        sido: SidoCode,
        sigungu: SigunguCode,
        eupmyeondong: EupmyeondongCode,
    ) -> Result<Self, AdminDivisionError> {
        if sigungu.sido_code() != sido {
            return Err(AdminDivisionError::SidoSigunguMismatch {
                sido: sido.as_str().to_owned(),
                sigungu: sigungu.as_str().to_owned(),
            });
        }
        if eupmyeondong.sigungu_code() != sigungu {
            return Err(AdminDivisionError::SigunguEupmyeondongMismatch {
                sigungu: sigungu.as_str().to_owned(),
                eupmyeondong: eupmyeondong.as_str().to_owned(),
            });
        }
        Ok(Self {
            sido,
            sigungu,
            eupmyeondong,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    // ── SidoCode ────────────────────────────────────────────────────────

    #[test]
    fn sido_seoul() {
        let s = SidoCode::try_new("11").expect("Seoul");
        assert_eq!(s.as_str(), "11");
    }

    #[test]
    fn sido_busan() {
        let s = SidoCode::try_new("26").expect("Busan");
        assert_eq!(s.as_str(), "26");
    }

    #[test]
    fn sido_rejects_too_short() {
        let err = SidoCode::try_new("1").unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::InvalidLength {
                expected: 2,
                actual: 1
            }
        ));
    }

    #[test]
    fn sido_rejects_too_long() {
        let err = SidoCode::try_new("111").unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::InvalidLength {
                expected: 2,
                actual: 3
            }
        ));
    }

    #[test]
    fn sido_rejects_non_digit() {
        let err = SidoCode::try_new("1A").unwrap_err();
        assert!(matches!(err, AdminDivisionError::NonDigit));
    }

    // ── SigunguCode ─────────────────────────────────────────────────────

    #[test]
    fn sigungu_seoul_jongno() {
        let s = SigunguCode::try_new("11110").expect("Seoul Jongno");
        assert_eq!(s.as_str(), "11110");
    }

    #[test]
    fn sigungu_extracts_sido() {
        let s = SigunguCode::try_new("11110").expect("ok");
        assert_eq!(s.sido_code().as_str(), "11");
    }

    #[test]
    fn sigungu_rejects_invalid_length() {
        let err = SigunguCode::try_new("1111").unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::InvalidLength {
                expected: 5,
                actual: 4
            }
        ));
    }

    // ── EupmyeondongCode ────────────────────────────────────────────────

    #[test]
    fn dong_seoul_jongno_cheongun() {
        let d = EupmyeondongCode::try_new("11110101").expect("Cheongun-dong");
        assert_eq!(d.as_str(), "11110101");
    }

    #[test]
    fn dong_extracts_sigungu_and_sido() {
        let d = EupmyeondongCode::try_new("11110101").expect("ok");
        assert_eq!(d.sigungu_code().as_str(), "11110");
        assert_eq!(d.sido_code().as_str(), "11");
    }

    #[test]
    fn dong_rejects_invalid_length() {
        let err = EupmyeondongCode::try_new("1111010").unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::InvalidLength {
                expected: 8,
                actual: 7
            }
        ));
    }

    #[test]
    fn dong_rejects_non_digit() {
        let err = EupmyeondongCode::try_new("1111A101").unwrap_err();
        assert!(matches!(err, AdminDivisionError::NonDigit));
    }

    // ── Display + FromStr ───────────────────────────────────────────────

    #[test]
    fn display_round_trips() {
        let s = SidoCode::try_new("11").expect("ok");
        assert_eq!(format!("{s}"), "11");
        let g = SigunguCode::try_new("11110").expect("ok");
        assert_eq!(format!("{g}"), "11110");
        let d = EupmyeondongCode::try_new("11110101").expect("ok");
        assert_eq!(format!("{d}"), "11110101");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        assert_eq!(SidoCode::from_str("11").unwrap().as_str(), "11");
        assert_eq!(SigunguCode::from_str("11110").unwrap().as_str(), "11110");
        assert_eq!(
            EupmyeondongCode::from_str("11110101").unwrap().as_str(),
            "11110101"
        );
    }

    // ── AdminDivision composite ─────────────────────────────────────────

    #[test]
    fn admin_division_seoul_jongno_cheongun() {
        let sido = SidoCode::try_new("11").expect("ok");
        let sigungu = SigunguCode::try_new("11110").expect("ok");
        let dong = EupmyeondongCode::try_new("11110101").expect("ok");
        let d = AdminDivision::try_new(sido, sigungu, dong).expect("valid");
        assert_eq!(d.sido.as_str(), "11");
        assert_eq!(d.sigungu.as_str(), "11110");
        assert_eq!(d.eupmyeondong.as_str(), "11110101");
    }

    #[test]
    fn admin_division_rejects_sido_sigungu_mismatch() {
        // sido = "11" (Seoul), but sigungu prefix = "26" (Busan)
        let sido = SidoCode::try_new("11").expect("ok");
        let sigungu = SigunguCode::try_new("26110").expect("ok");
        let dong = EupmyeondongCode::try_new("26110101").expect("ok");
        let err = AdminDivision::try_new(sido, sigungu, dong).unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::SidoSigunguMismatch { .. }
        ));
    }

    #[test]
    fn admin_division_rejects_sigungu_eupmyeondong_mismatch() {
        // sigungu = "11110", but eupmyeondong prefix = "11140"
        let sido = SidoCode::try_new("11").expect("ok");
        let sigungu = SigunguCode::try_new("11110").expect("ok");
        let dong = EupmyeondongCode::try_new("11140101").expect("ok");
        let err = AdminDivision::try_new(sido, sigungu, dong).unwrap_err();
        assert!(matches!(
            err,
            AdminDivisionError::SigunguEupmyeondongMismatch { .. }
        ));
    }

    #[test]
    fn admin_division_serde_roundtrip() {
        let sido = SidoCode::try_new("11").expect("ok");
        let sigungu = SigunguCode::try_new("11110").expect("ok");
        let dong = EupmyeondongCode::try_new("11110101").expect("ok");
        let d = AdminDivision::try_new(sido, sigungu, dong).expect("valid");
        let json = serde_json::to_string(&d).expect("serialize");
        let back: AdminDivision = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(d, back);
    }

    #[test]
    fn admin_division_accessor_consistency() {
        let sido = SidoCode::try_new("11").expect("ok");
        let sigungu = SigunguCode::try_new("11110").expect("ok");
        let dong = EupmyeondongCode::try_new("11110101").expect("ok");
        let d = AdminDivision::try_new(sido, sigungu, dong).expect("valid");
        assert_eq!(d.sido, d.sigungu.sido_code());
        assert_eq!(d.sigungu, d.eupmyeondong.sigungu_code());
        assert_eq!(d.sido, d.eupmyeondong.sido_code());
    }

    #[test]
    fn admin_division_clone_and_eq() {
        let sido = SidoCode::try_new("26").expect("ok");
        let sigungu = SigunguCode::try_new("26110").expect("ok");
        let dong = EupmyeondongCode::try_new("26110101").expect("ok");
        let d = AdminDivision::try_new(sido, sigungu, dong).expect("valid");
        let cloned = d.clone();
        assert_eq!(d, cloned);
    }
}
