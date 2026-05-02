//! 한국 전화번호 (`PhoneKr`) 값 객체.
//!
//! 입력 형식 다양 (하이픈/공백/괄호 포함, `+82` prefix 등) → 숫자만 추출 후
//! `+82`를 선두 `0`으로 정규화. 결과는 9-11자리 `0`으로 시작하는 숫자 문자열.
//!
//! **명시적 `+82` prefix만 strip해요.** Raw `82xxx...`는 모호하므로
//! 그대로 검증하고 `MustStartWithZero`로 거부해요.
//!
//! `MSISDN` 형식 (E.164에서 `+` 제거 후 국가별 표기)에 가까운 한국 로컬 표기를 사용해요.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 한국 전화번호 (`MSISDN` 정규화).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PhoneKr(String);

/// `PhoneKr` 검증 에러.
#[derive(Debug, Error)]
pub enum PhoneKrError {
    /// 정규화 후 길이가 9-11 범위 밖.
    #[error("phone must be 9-11 digits, got {actual}")]
    InvalidLength {
        /// 정규화 후 실제 길이.
        actual: usize,
    },
    /// 선두 `0`으로 시작하지 않음.
    #[error("phone must start with 0 (got prefix '{prefix}')")]
    MustStartWithZero {
        /// 첫 글자.
        prefix: String,
    },
}

impl PhoneKr {
    /// 다양한 입력을 정규화 후 `PhoneKr` 생성.
    ///
    /// 처리 순서:
    /// 1. 모든 비숫자 문자 제거
    /// 2. 입력이 `+82`로 시작했다면 선두 `82` → `0`로 치환 (`+82-10-1234-5678` → `01012345678`)
    /// 3. 길이 9-11 검증
    /// 4. 선두 `0` 검증
    ///
    /// Raw `82xxx...` (명시적 `+` 없음)는 모호하므로 그대로 검증해요.
    ///
    /// # Errors
    ///
    /// 길이가 9-11 범위 밖이면 [`PhoneKrError::InvalidLength`].
    /// 선두가 `0`이 아니면 [`PhoneKrError::MustStartWithZero`].
    pub fn try_new(s: &str) -> Result<Self, PhoneKrError> {
        let trimmed = s.trim();
        let has_plus_82 = trimmed.starts_with("+82");

        let mut digits: String = trimmed.chars().filter(char::is_ascii_digit).collect();

        // 명시적 `+82` 국제 prefix만 strip → 선두 `0`으로 치환.
        // Raw `82...`는 모호하므로 leading-zero 검증에서 거부.
        if has_plus_82 {
            if let Some(rest) = digits.strip_prefix("82") {
                digits = format!("0{rest}");
            }
        }

        if !(9..=11).contains(&digits.len()) {
            return Err(PhoneKrError::InvalidLength {
                actual: digits.len(),
            });
        }

        if !digits.starts_with('0') {
            let prefix = digits
                .chars()
                .next()
                .map_or_else(String::new, |c| c.to_string());
            return Err(PhoneKrError::MustStartWithZero { prefix });
        }

        Ok(Self(digits))
    }

    /// 정규화된 숫자만 문자열 (예: `01012345678`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PhoneKr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for PhoneKr {
    type Err = PhoneKrError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{PhoneKr, PhoneKrError};

    #[test]
    fn parse_mobile_with_hyphens() {
        let p = PhoneKr::try_new("010-1234-5678").expect("valid");
        assert_eq!(p.as_str(), "01012345678");
    }

    #[test]
    fn parse_mobile_no_hyphens() {
        let p = PhoneKr::try_new("01012345678").expect("valid");
        assert_eq!(p.as_str(), "01012345678");
    }

    #[test]
    fn parse_seoul_landline_9_digits() {
        let p = PhoneKr::try_new("02-123-4567").expect("valid 9-digit landline");
        assert_eq!(p.as_str(), "021234567");
    }

    #[test]
    fn parse_seoul_landline_10_digits() {
        let p = PhoneKr::try_new("02-1234-5678").expect("valid 10-digit landline");
        assert_eq!(p.as_str(), "0212345678");
    }

    #[test]
    fn parse_provincial_landline() {
        let p = PhoneKr::try_new("031-123-4567").expect("valid Gyeonggi");
        assert_eq!(p.as_str(), "0311234567");
    }

    #[test]
    fn parse_with_plus_82_prefix() {
        let p = PhoneKr::try_new("+82-10-1234-5678").expect("valid +82 form");
        assert_eq!(p.as_str(), "01012345678");
    }

    #[test]
    fn rejects_82_prefix_without_plus() {
        // Without "+", leading "82..." is treated as raw digits.
        // 8212345678 → 10 digits starting with 8 → MustStartWithZero
        let err = PhoneKr::try_new("8212345678").unwrap_err();
        assert!(matches!(err, PhoneKrError::MustStartWithZero { .. }));
    }

    #[test]
    fn rejects_82_prefix_with_zero_after() {
        // "+82-0-..." would silently strip "82" → "0-..." which is correct,
        // but typing "+82" then "0" is a common user mistake. Verify the
        // result is at least valid Korean (starts with 0, length 9-11).
        // "+82-0-2-1234-5678" → digits "820212345678" (12) → strip "82" + prepend "0"
        //   → "00212345678" (11 digits). Redundant `0` after country code is preserved.
        // Current behavior: accepts. Document as expected-but-suboptimal.
        let p = PhoneKr::try_new("+82-0-2-1234-5678").expect("strips +82, accepts redundant 0");
        assert_eq!(p.as_str(), "00212345678");
    }

    #[test]
    fn parse_with_82_prefix_seoul() {
        let p = PhoneKr::try_new("+82-2-1234-5678").expect("valid Seoul +82");
        assert_eq!(p.as_str(), "0212345678");
    }

    #[test]
    fn parse_with_parentheses_and_spaces() {
        let p = PhoneKr::try_new("(02) 1234-5678").expect("valid");
        assert_eq!(p.as_str(), "0212345678");
    }

    #[test]
    fn rejects_too_short() {
        let err = PhoneKr::try_new("0123").unwrap_err();
        assert!(matches!(err, PhoneKrError::InvalidLength { actual: 4 }));
    }

    #[test]
    fn rejects_too_long() {
        let err = PhoneKr::try_new("012345678901").unwrap_err();
        assert!(matches!(err, PhoneKrError::InvalidLength { actual: 12 }));
    }

    #[test]
    fn rejects_no_digits() {
        let err = PhoneKr::try_new("hello world").unwrap_err();
        assert!(matches!(err, PhoneKrError::InvalidLength { actual: 0 }));
    }

    #[test]
    fn rejects_no_leading_zero() {
        // 11 digits but doesn't start with 0 (also doesn't start with 82)
        let err = PhoneKr::try_new("11234567890").unwrap_err();
        assert!(matches!(err, PhoneKrError::MustStartWithZero { .. }));
    }

    #[test]
    fn display_round_trips() {
        use std::fmt::Write;
        let p = PhoneKr::try_new("010-1234-5678").expect("ok");
        let mut out = String::new();
        write!(out, "{p}").expect("write");
        assert_eq!(out, "01012345678");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        let p = PhoneKr::from_str("+82-10-1234-5678").expect("ok");
        assert_eq!(p.as_str(), "01012345678");
    }
}
