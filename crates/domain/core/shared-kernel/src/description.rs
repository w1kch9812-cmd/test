//! `Description` — 매물 설명 값 객체.
//!
//! Spec § 5.1 listing 테이블 `description text not null default ''`.
//! 빈 *허용* (`default` 빈 문자열). 길이 ≤5000자 (application-level cap).
//! `text` type은 `DB`-level 길이 제한 없지만 UI/응답 크기 보호 위해 cap.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `Description` 최대 길이 (UTF-8 byte).
const MAX_DESCRIPTION_BYTES: usize = 5000;

/// 매물 설명 (≤5000 bytes).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Description(String);

/// `Description` 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DescriptionError {
    /// 5000자 초과.
    #[error("description exceeds 5000 chars (got {actual})")]
    TooLong {
        /// 실제 byte 길이.
        actual: usize,
    },
}

impl Description {
    /// 검증 후 `Description` 생성. trim은 *적용 안 함* (사용자 의도 보존).
    /// 빈 문자열 *허용* (`DB` `default`).
    ///
    /// # Errors
    /// 5000자 초과 → `TooLong`.
    pub fn try_new(s: &str) -> Result<Self, DescriptionError> {
        if s.len() > MAX_DESCRIPTION_BYTES {
            return Err(DescriptionError::TooLong { actual: s.len() });
        }
        Ok(Self(s.to_owned()))
    }

    /// 빈 `Description` (`DB` `default`).
    #[must_use]
    pub const fn empty() -> Self {
        Self(String::new())
    }

    /// 내부 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 빈 문자열인지.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Default for Description {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for Description {
    type Err = DescriptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn typical_description() {
        let d = Description::try_new("서울 종로구 산업용 부지. 약 250㎡, 진입 차량 가능.")
            .expect("valid");
        assert!(!d.is_empty());
    }

    #[test]
    fn empty_string_allowed() {
        let d = Description::try_new("").expect("empty allowed (DB default)");
        assert!(d.is_empty());
        assert_eq!(d.as_str(), "");
    }

    #[test]
    fn empty_constructor() {
        let d = Description::empty();
        assert!(d.is_empty());
    }

    #[test]
    fn default_is_empty() {
        assert_eq!(Description::default(), Description::empty());
    }

    #[test]
    fn accepts_5000_bytes() {
        let s = "X".repeat(5000);
        let d = Description::try_new(&s).expect("5000 ok");
        assert_eq!(d.as_str().len(), 5000);
    }

    #[test]
    fn rejects_5001_bytes() {
        let s = "X".repeat(5001);
        let err = Description::try_new(&s).unwrap_err();
        assert!(matches!(err, DescriptionError::TooLong { actual: 5001 }));
    }

    #[test]
    fn multiline_preserved() {
        let multiline = "line 1\nline 2\nline 3";
        let d = Description::try_new(multiline).expect("ok");
        assert_eq!(d.as_str(), multiline);
    }

    #[test]
    fn no_trim_applied() {
        // 사용자 의도 보존 — leading/trailing whitespace는 그대로.
        let d = Description::try_new("  보존 됨  ").expect("ok");
        assert_eq!(d.as_str(), "  보존 됨  ");
    }

    #[test]
    fn display_round_trips() {
        let d = Description::try_new("hi").expect("ok");
        assert_eq!(format!("{d}"), "hi");
    }
}
