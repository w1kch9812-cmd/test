//! 한국 표준 산업분류 (`KSIC`) 코드 값 객체.
//!
//! 형식: 5자리 = 첫 글자(대분류 영문 대문자, 21개 section) + 뒤 4자리(`ASCII` 숫자).
//!
//! 예시:
//! - `C2620` — C(제조업) > 2620(통신·방송장비 제조업)
//! - `A0111` — A(농업·임업·어업) > 0111(곡물 재배업)
//! - `K6420` — K(금융·보험업) > 6420(은행)
//!
//! 통계청 `KSIC` 11차 개정(2024) 기준이지만 *대분류 letter 자체*는 검증하지 않아요
//! (개정마다 letter set이 바뀔 수 있어 외부 사전 의존성 회피).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 한국 표준 산업분류 코드.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KsicCode(String);

/// `KsicCode` 검증 에러.
#[derive(Debug, Error)]
pub enum KsicCodeError {
    /// 길이가 5자리가 아님.
    #[error("KSIC code must be 5 chars, got {actual}")]
    InvalidLength {
        /// 실제 길이.
        actual: usize,
    },
    /// 첫 글자가 영문 대문자가 아님.
    #[error("KSIC section must be uppercase ASCII letter (got '{first}')")]
    FirstMustBeUppercase {
        /// 실제 첫 글자.
        first: char,
    },
    /// 뒤 4자리가 `ASCII` 숫자가 아님.
    #[error("KSIC subcategory (chars 2-5) must be ASCII digits (got '{tail}')")]
    TailMustBeDigits {
        /// 실제 뒤 4자리.
        tail: String,
    },
}

impl KsicCode {
    /// 검증 후 `KsicCode` 생성.
    ///
    /// # Errors
    ///
    /// 길이 ≠ 5 → `InvalidLength`. 첫 글자가 대문자 영문 아니면 `FirstMustBeUppercase`.
    /// 뒤 4자리가 숫자 아니면 `TailMustBeDigits`.
    pub fn try_new(s: &str) -> Result<Self, KsicCodeError> {
        if s.len() != 5 {
            return Err(KsicCodeError::InvalidLength { actual: s.len() });
        }
        let mut chars = s.chars();
        // SAFETY: length 5 guaranteed by check above; .next() yields Some.
        let first = chars.next().expect("length 5 guarantees first char");
        if !first.is_ascii_uppercase() {
            return Err(KsicCodeError::FirstMustBeUppercase { first });
        }
        let tail: String = chars.collect();
        if !tail.chars().all(|c| c.is_ascii_digit()) {
            return Err(KsicCodeError::TailMustBeDigits { tail });
        }
        Ok(Self(s.to_owned()))
    }

    /// 내부 5자리 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 대분류 section (첫 글자).
    ///
    /// # Panics
    ///
    /// 검증 통과한 `KsicCode`만 생성되므로 항상 첫 글자가 존재해요.
    /// 이 panic은 이론상 도달 불가능해요.
    #[must_use]
    #[allow(clippy::expect_used)] // see # Panics — try_new guarantees length 5
    pub fn section(&self) -> char {
        self.0.chars().next().expect("KsicCode is always 5 chars")
    }
}

impl std::fmt::Display for KsicCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for KsicCode {
    type Err = KsicCodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_valid_manufacturing() {
        let k = KsicCode::try_new("C2620").expect("valid");
        assert_eq!(k.as_str(), "C2620");
        assert_eq!(k.section(), 'C');
    }

    #[test]
    fn parse_valid_agriculture() {
        let k = KsicCode::try_new("A0111").expect("valid");
        assert_eq!(k.section(), 'A');
    }

    #[test]
    fn parse_valid_finance() {
        let k = KsicCode::try_new("K6420").expect("valid");
        assert_eq!(k.section(), 'K');
    }

    #[test]
    fn rejects_too_short() {
        let err = KsicCode::try_new("C262").unwrap_err();
        assert!(matches!(err, KsicCodeError::InvalidLength { actual: 4 }));
    }

    #[test]
    fn rejects_too_long() {
        let err = KsicCode::try_new("C26200").unwrap_err();
        assert!(matches!(err, KsicCodeError::InvalidLength { actual: 6 }));
    }

    #[test]
    fn rejects_lowercase_first_char() {
        let err = KsicCode::try_new("c2620").unwrap_err();
        assert!(matches!(
            err,
            KsicCodeError::FirstMustBeUppercase { first: 'c' }
        ));
    }

    #[test]
    fn rejects_digit_first_char() {
        let err = KsicCode::try_new("12620").unwrap_err();
        assert!(matches!(
            err,
            KsicCodeError::FirstMustBeUppercase { first: '1' }
        ));
    }

    #[test]
    fn rejects_non_digit_tail() {
        let err = KsicCode::try_new("C262X").unwrap_err();
        assert!(matches!(err, KsicCodeError::TailMustBeDigits { .. }));
    }

    #[test]
    fn rejects_unicode_first_char() {
        // 길이를 5로 맞춰야 InvalidLength가 아닌 FirstMustBeUppercase로 떨어지는데,
        // 한글은 UTF-8에서 3바이트라 "가1234"는 .len() = 7이 되어 InvalidLength로 처리됨.
        let err = KsicCode::try_new("가1234").unwrap_err();
        assert!(matches!(err, KsicCodeError::InvalidLength { .. }));
    }

    #[test]
    fn display_round_trips() {
        let k = KsicCode::try_new("C2620").expect("ok");
        assert_eq!(format!("{k}"), "C2620");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        assert_eq!(KsicCode::from_str("C2620").unwrap().as_str(), "C2620");
    }

    #[test]
    fn section_returns_first_char() {
        for (code, section) in [("C2620", 'C'), ("A0111", 'A'), ("Z9999", 'Z')] {
            let k = KsicCode::try_new(code).expect("valid");
            assert_eq!(k.section(), section);
        }
    }
}
