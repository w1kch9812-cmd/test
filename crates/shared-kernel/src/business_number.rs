//! 한국 사업자등록번호 (`BusinessNumber`) 값 객체.
//!
//! 형식: 10자리 ASCII 숫자 (`123-45-67890` 또는 `1234567890`).
//! 국세청 (`NTS`) 체크섬 알고리즘으로 검증해요.
//!
//! ⚠️ 알고리즘은 학습 데이터 기반이라 운영 전 공식 명세 교차 확인 필요해요.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 한국 사업자등록번호.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusinessNumber(String);

/// `BusinessNumber` 검증 에러.
#[derive(Debug, Error)]
pub enum BusinessNumberError {
    /// 10자리가 아님 (하이픈 제거 후).
    #[error("business number must be 10 digits, got {actual}")]
    InvalidLength {
        /// 정규화 후 길이.
        actual: usize,
    },
    /// `ASCII` 숫자가 아닌 문자 포함 (하이픈 제외).
    #[error("business number must contain only ASCII digits (with optional hyphens)")]
    NonDigit,
    /// 국세청 체크섬 불일치.
    #[error("business number checksum invalid")]
    InvalidChecksum,
}

impl BusinessNumber {
    /// 검증 후 `BusinessNumber` 생성.
    ///
    /// 하이픈과 공백은 자동 제거됨. 길이 + 숫자 + 체크섬 모두 통과해야 해요.
    ///
    /// # Errors
    ///
    /// 길이 ≠ 10 → [`BusinessNumberError::InvalidLength`].
    /// 숫자 외 문자 → [`BusinessNumberError::NonDigit`].
    /// 체크섬 실패 → [`BusinessNumberError::InvalidChecksum`].
    pub fn try_new(s: &str) -> Result<Self, BusinessNumberError> {
        let cleaned: String = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect();
        if cleaned.len() != 10 {
            return Err(BusinessNumberError::InvalidLength {
                actual: cleaned.len(),
            });
        }
        if !cleaned.chars().all(|c| c.is_ascii_digit()) {
            return Err(BusinessNumberError::NonDigit);
        }
        if !verify_checksum(&cleaned) {
            return Err(BusinessNumberError::InvalidChecksum);
        }
        Ok(Self(cleaned))
    }

    /// 정규화된 10자리 문자열 (하이픈 없음).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 국세청 (`NTS`) 사업자등록번호 체크섬 검증.
///
/// **⚠️ 학습 데이터 기반 알고리즘.** 운영 전에 국세청 공식 명세
/// 또는 `data.go.kr` 사업자상태조회 검증 API와 비교 테스트 권고.
///
/// 가중치 `[1, 3, 7, 1, 3, 7, 1, 3, 5]`를 D₁..D₉에 적용하고, D₉ × 5의 십의 자리
/// `carry`를 더한 뒤 `(10 - sum mod 10) mod 10`이 D₁₀ (체크 디지트)와 일치하면 유효.
fn verify_checksum(digits: &str) -> bool {
    debug_assert_eq!(digits.len(), 10, "verify_checksum requires 10 digits");
    let weights = [1u32, 3, 7, 1, 3, 7, 1, 3, 5];
    let bytes = digits.as_bytes();
    let mut sum: u32 = 0;
    for (i, &w) in weights.iter().enumerate() {
        sum += u32::from(bytes[i] - b'0') * w;
    }
    // D₉ × 5의 십의 자리 carry 추가.
    sum += (u32::from(bytes[8] - b'0') * 5) / 10;
    let check = (10 - (sum % 10)) % 10;
    check == u32::from(bytes[9] - b'0')
}

impl std::fmt::Display for BusinessNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for BusinessNumber {
    type Err = BusinessNumberError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{BusinessNumber, BusinessNumberError};

    // ── 알고리즘 검증 (수동 계산 일치하는 입력) ───────────────
    //
    // "1234567891":
    //   weighted: 1*1 + 2*3 + 3*7 + 4*1 + 5*3 + 6*7 + 7*1 + 8*3 + 9*5
    //           = 1 + 6 + 21 + 4 + 15 + 42 + 7 + 24 + 45 = 165
    //   carry:    floor(9*5 / 10) = floor(45/10) = 4
    //   sum:      165 + 4 = 169
    //   check:    (10 - 169%10) % 10 = (10 - 9) % 10 = 1  → matches D₁₀ = 1
    // → VALID
    //
    // "1234567890" (D₁₀ -1):
    //   same sum 169, check digit 1 ≠ 0 → INVALID
    //
    // "1111111119":
    //   weighted: 1+3+7+1+3+7+1+3+5 = 31
    //   carry:    floor(5/10) = 0
    //   sum:      31
    //   check:    (10 - 1) % 10 = 9  → matches D₁₀ = 9
    // → VALID

    const VALID_NO_HYPHEN: &str = "1234567891";
    const VALID_WITH_HYPHEN: &str = "123-45-67891";
    const INVALID_CHECKSUM: &str = "1234567890";

    #[test]
    fn parse_valid_no_hyphen() {
        let bn = BusinessNumber::try_new(VALID_NO_HYPHEN).expect("valid checksum");
        assert_eq!(bn.as_str(), VALID_NO_HYPHEN);
    }

    #[test]
    fn parse_with_hyphens_normalizes() {
        let bn = BusinessNumber::try_new(VALID_WITH_HYPHEN).expect("valid + hyphens");
        assert_eq!(bn.as_str(), VALID_NO_HYPHEN); // hyphens stripped
    }

    #[test]
    fn parse_with_whitespace_normalizes() {
        let bn = BusinessNumber::try_new("  123-45-67891  ").expect("trim + normalize");
        assert_eq!(bn.as_str(), VALID_NO_HYPHEN);
    }

    #[test]
    fn rejects_invalid_checksum() {
        let err = BusinessNumber::try_new(INVALID_CHECKSUM).unwrap_err();
        assert!(matches!(err, BusinessNumberError::InvalidChecksum));
    }

    #[test]
    fn rejects_too_short() {
        let err = BusinessNumber::try_new("12345").unwrap_err();
        assert!(matches!(
            err,
            BusinessNumberError::InvalidLength { actual: 5 }
        ));
    }

    #[test]
    fn rejects_too_long() {
        let err = BusinessNumber::try_new("12345678901").unwrap_err();
        assert!(matches!(
            err,
            BusinessNumberError::InvalidLength { actual: 11 }
        ));
    }

    #[test]
    fn rejects_non_digit_letters() {
        let err = BusinessNumber::try_new("abcdefghij").unwrap_err();
        assert!(matches!(err, BusinessNumberError::NonDigit));
    }

    #[test]
    fn rejects_mixed_letters_digits() {
        let err = BusinessNumber::try_new("12345abcde").unwrap_err();
        assert!(matches!(err, BusinessNumberError::NonDigit));
    }

    #[test]
    fn algorithm_matches_manual_calc_alternate_valid() {
        // "1111111119" — second independent test of algorithm
        let bn = BusinessNumber::try_new("1111111119").expect("valid by manual calc");
        assert_eq!(bn.as_str(), "1111111119");
    }

    #[test]
    fn display_round_trips() {
        use std::fmt::Write;
        let bn = BusinessNumber::try_new(VALID_WITH_HYPHEN).expect("ok");
        let mut s = String::new();
        write!(s, "{bn}").expect("write ok");
        assert_eq!(s, VALID_NO_HYPHEN);
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        let bn = BusinessNumber::from_str(VALID_WITH_HYPHEN).expect("ok");
        assert_eq!(bn.as_str(), VALID_NO_HYPHEN);
    }
}
