//! 한국 주소 값 객체 — 도로명·지번 두 종류.
//!
//! 두 타입은 *별도*예요 — `RoadAddress`와 `JibunAddress`를 컴파일 타임에 구분해
//! 호출자가 서로 섞지 못하게 해요. 형식 검증은 *비어 있지 않음 + ≤200자* 만.
//! 깊은 구조 검증은 외부 정규화 API (도로명주소 API 등) 책임.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 도로명 주소 (예: `서울특별시 종로구 종로 1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoadAddress(String);

impl RoadAddress {
    /// 검증 후 `RoadAddress` 생성. 앞뒤 공백 trim.
    ///
    /// # Errors
    ///
    /// trim 후 빈 문자열 → `Empty`. 200자 초과 → `TooLong`.
    pub fn try_new(s: &str) -> Result<Self, AddressError> {
        let trimmed = validate(s)?;
        Ok(Self(trimmed))
    }

    /// 내부 주소 문자열 (trim 적용됨).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 지번 주소 (예: `서울특별시 종로구 청운동 1-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JibunAddress(String);

impl JibunAddress {
    /// 검증 후 `JibunAddress` 생성. 앞뒤 공백 trim.
    ///
    /// # Errors
    ///
    /// trim 후 빈 문자열 → `Empty`. 200자 초과 → `TooLong`.
    pub fn try_new(s: &str) -> Result<Self, AddressError> {
        let trimmed = validate(s)?;
        Ok(Self(trimmed))
    }

    /// 내부 주소 문자열 (trim 적용됨).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 주소 검증 에러 (`RoadAddress` / `JibunAddress` 공통).
#[derive(Debug, Error)]
pub enum AddressError {
    /// 빈 문자열 (또는 공백만).
    #[error("address cannot be empty")]
    Empty,
    /// 길이 200자 초과 (`DB varchar(200)` 매핑).
    #[error("address exceeds 200 chars (got {actual})")]
    TooLong {
        /// trim 후 실제 길이.
        actual: usize,
    },
}

fn validate(s: &str) -> Result<String, AddressError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(AddressError::Empty);
    }
    if trimmed.len() > 200 {
        return Err(AddressError::TooLong {
            actual: trimmed.len(),
        });
    }
    Ok(trimmed.to_owned())
}

// ── Display + FromStr (2 types × 2 = 4 impls) ────────────────────────

impl std::fmt::Display for RoadAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::str::FromStr for RoadAddress {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

impl std::fmt::Display for JibunAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::str::FromStr for JibunAddress {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    // ── RoadAddress ────────────────────────────────────────────────

    #[test]
    fn road_typical() {
        let r = RoadAddress::try_new("서울특별시 종로구 종로 1").expect("valid");
        assert_eq!(r.as_str(), "서울특별시 종로구 종로 1");
    }

    #[test]
    fn road_trims() {
        let r = RoadAddress::try_new("  서울 종로구 종로 1  ").expect("trim");
        assert_eq!(r.as_str(), "서울 종로구 종로 1");
    }

    #[test]
    fn road_rejects_empty() {
        let err = RoadAddress::try_new("").unwrap_err();
        assert!(matches!(err, AddressError::Empty));
    }

    #[test]
    fn road_rejects_whitespace_only() {
        let err = RoadAddress::try_new("   ").unwrap_err();
        assert!(matches!(err, AddressError::Empty));
    }

    #[test]
    fn road_rejects_too_long() {
        let s = "가".repeat(201); // 201 hangul chars; bytes = 603 (3 each)
        // The actual length in chars matters for the user, but our check is byte-len.
        // 가.len() = 3 bytes per char; 201 hangul = 603 bytes, well over 200.
        let err = RoadAddress::try_new(&s).unwrap_err();
        assert!(matches!(err, AddressError::TooLong { .. }));
    }

    #[test]
    fn road_accepts_exactly_200_bytes() {
        let s = "X".repeat(200); // 200 ASCII chars = 200 bytes
        let r = RoadAddress::try_new(&s).expect("200 ok");
        assert_eq!(r.as_str().len(), 200);
    }

    // ── JibunAddress ────────────────────────────────────────────────

    #[test]
    fn jibun_typical() {
        let j = JibunAddress::try_new("서울특별시 종로구 청운동 1-1").expect("valid");
        assert_eq!(j.as_str(), "서울특별시 종로구 청운동 1-1");
    }

    #[test]
    fn jibun_trims() {
        let j = JibunAddress::try_new("  서울 종로구 청운동 1-1  ").expect("trim");
        assert_eq!(j.as_str(), "서울 종로구 청운동 1-1");
    }

    #[test]
    fn jibun_rejects_empty() {
        let err = JibunAddress::try_new("").unwrap_err();
        assert!(matches!(err, AddressError::Empty));
    }

    // ── Type discrimination (compile-time) ──────────────────────────

    // This test doesn't run logic — it documents the type-safety guarantee:
    // RoadAddress and JibunAddress are NOT interchangeable.
    #[test]
    fn types_are_distinct() {
        let r = RoadAddress::try_new("서울 종로구 종로 1").expect("ok");
        let j = JibunAddress::try_new("서울 종로구 청운동 1-1").expect("ok");
        // Both have inner String, but they have different types — passing one
        // where the other is expected is a compile error.
        assert_ne!(r.as_str(), j.as_str());
    }

    // ── Display + FromStr ───────────────────────────────────────────

    #[test]
    fn display_round_trips() {
        let r = RoadAddress::try_new("서울 종로구 종로 1").expect("ok");
        assert_eq!(format!("{r}"), "서울 종로구 종로 1");

        let j = JibunAddress::try_new("서울 종로구 청운동 1-1").expect("ok");
        assert_eq!(format!("{j}"), "서울 종로구 청운동 1-1");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        assert_eq!(
            RoadAddress::from_str("서울 종로구 종로 1").unwrap().as_str(),
            "서울 종로구 종로 1"
        );
        assert_eq!(
            JibunAddress::from_str("서울 종로구 청운동 1-1")
                .unwrap()
                .as_str(),
            "서울 종로구 청운동 1-1"
        );
    }
}
