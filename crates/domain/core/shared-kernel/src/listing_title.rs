//! `ListingTitle` — 매물 제목 값 객체.
//!
//! Spec § 5.1 listing 테이블 `title varchar(200) not null`.
//! 빈 문자열 거부 + ≤200자 (UTF-8 byte 기준).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 매물 제목.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ListingTitle(String);

/// `ListingTitle` 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ListingTitleError {
    /// 빈 문자열 (또는 공백만).
    #[error("listing title cannot be empty")]
    Empty,
    /// 200자 초과 (`varchar(200)` 매핑 한도).
    #[error("listing title exceeds 200 chars (got {actual})")]
    TooLong {
        /// 실제 byte 길이.
        actual: usize,
    },
}

impl ListingTitle {
    /// trim + 빈/길이 검증 후 생성.
    ///
    /// # Errors
    /// trim 후 빈 → `Empty`. 200자 초과 → `TooLong`.
    pub fn try_new(s: &str) -> Result<Self, ListingTitleError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(ListingTitleError::Empty);
        }
        if trimmed.len() > 200 {
            return Err(ListingTitleError::TooLong {
                actual: trimmed.len(),
            });
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// 내부 문자열 (trim 적용됨).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ListingTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for ListingTitle {
    type Err = ListingTitleError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn typical_korean_title() {
        let t = ListingTitle::try_new("서울 종로구 산업용 부지 매매").expect("valid");
        assert_eq!(t.as_str(), "서울 종로구 산업용 부지 매매");
    }

    #[test]
    fn trims_whitespace() {
        let t = ListingTitle::try_new("  공장 임대  ").expect("trim");
        assert_eq!(t.as_str(), "공장 임대");
    }

    #[test]
    fn rejects_empty() {
        let err = ListingTitle::try_new("").unwrap_err();
        assert!(matches!(err, ListingTitleError::Empty));
    }

    #[test]
    fn rejects_whitespace_only() {
        let err = ListingTitle::try_new("   ").unwrap_err();
        assert!(matches!(err, ListingTitleError::Empty));
    }

    #[test]
    fn accepts_exactly_200_bytes() {
        let s = "X".repeat(200);
        let t = ListingTitle::try_new(&s).expect("200 ok");
        assert_eq!(t.as_str().len(), 200);
    }

    #[test]
    fn rejects_201_bytes() {
        let s = "X".repeat(201);
        let err = ListingTitle::try_new(&s).unwrap_err();
        assert!(matches!(err, ListingTitleError::TooLong { actual: 201 }));
    }

    #[test]
    fn rejects_201_byte_korean() {
        // 67 한글 chars = 201 UTF-8 bytes.
        let s = "가".repeat(67);
        assert_eq!(s.len(), 201);
        let err = ListingTitle::try_new(&s).unwrap_err();
        assert!(matches!(err, ListingTitleError::TooLong { actual: 201 }));
    }

    #[test]
    fn display_round_trips() {
        let t = ListingTitle::try_new("창고 임대").expect("ok");
        assert_eq!(format!("{t}"), "창고 임대");
    }

    #[test]
    fn from_str_round_trips() {
        let t = ListingTitle::from_str("산업용지 매매").expect("ok");
        assert_eq!(t.as_str(), "산업용지 매매");
    }
}
