//! 이메일 주소 (`Email`) 값 객체.
//!
//! `RFC 5322` *간소화* 정규식으로 검증해요. 대문자는 자동 소문자 정규화해요.
//! 길이는 `RFC 5321` `SMTP` envelope 한도인 254자 이하로 제한해요.
//!
//! 일부 `RFC 5322` 엣지 케이스(따옴표 묶인 local part, IP literal 도메인)는
//! 의도적으로 거부해요 — 실무에서 거의 사용 안 되고 `ReDoS` 위험을 줄이기 위해서예요.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `RFC 5322` 간소화 정규식 컴파일 헬퍼.
///
/// 컴파일 타임 상수 패턴이라 `Regex::new`가 실패할 수 없어요.
/// 그래서 `expect`를 허용해요 — 실패하면 패턴 리터럴이 잘못된 거고
/// 그건 빌드 시점에 테스트로 잡혀요.
#[allow(clippy::expect_used)]
fn build_email_regex() -> Regex {
    // local part: [A-Za-z0-9._%+-]+
    // domain:     [A-Za-z0-9.-]+
    // tld:        [A-Za-z]{2,}
    Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$")
        .expect("static regex literal must compile")
}

/// `RFC 5322` 간소화 패턴 (lazy 컴파일, 1회만).
static EMAIL_RE: Lazy<Regex> = Lazy::new(build_email_regex);

/// 이메일 주소.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Email(String);

/// `Email` 검증 에러.
#[derive(Debug, Error)]
pub enum EmailError {
    /// 길이 254자 초과 (`RFC 5321` `SMTP` envelope 한도).
    #[error("email exceeds 254 chars (got {actual})")]
    TooLong {
        /// trim/소문자 후 실제 길이.
        actual: usize,
    },
    /// 정규식 불일치 (`local@domain.tld` 형식 위반).
    #[error("email format invalid (expected local@domain.tld)")]
    InvalidFormat,
}

impl Email {
    /// 검증 후 `Email` 생성. trim + 소문자 정규화.
    ///
    /// # Errors
    ///
    /// 254자 초과 → [`EmailError::TooLong`].
    /// 정규식 미일치 → [`EmailError::InvalidFormat`].
    pub fn try_new(s: &str) -> Result<Self, EmailError> {
        let normalized = s.trim().to_ascii_lowercase();
        if normalized.len() > 254 {
            return Err(EmailError::TooLong {
                actual: normalized.len(),
            });
        }
        if !EMAIL_RE.is_match(&normalized) {
            return Err(EmailError::InvalidFormat);
        }
        Ok(Self(normalized))
    }

    /// 정규화된 (소문자 + trim) 이메일 문자열.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Email {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Email {
    type Err = EmailError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_new(s)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn parse_simple_valid() {
        let e = Email::try_new("user@example.com").expect("valid");
        assert_eq!(e.as_str(), "user@example.com");
    }

    #[test]
    fn parse_with_dots_and_plus() {
        let e = Email::try_new("first.last+tag@example.co.kr").expect("valid");
        assert_eq!(e.as_str(), "first.last+tag@example.co.kr");
    }

    #[test]
    fn parse_with_hyphen_in_domain() {
        let e = Email::try_new("user@my-company.com").expect("valid");
        assert_eq!(e.as_str(), "user@my-company.com");
    }

    #[test]
    fn normalizes_uppercase_to_lowercase() {
        let e = Email::try_new("USER@Example.COM").expect("valid");
        assert_eq!(e.as_str(), "user@example.com");
    }

    #[test]
    fn trims_leading_trailing_whitespace() {
        let e = Email::try_new("  user@example.com  ").expect("valid");
        assert_eq!(e.as_str(), "user@example.com");
    }

    #[test]
    fn rejects_no_at_sign() {
        let err = Email::try_new("user.example.com").unwrap_err();
        assert!(matches!(err, EmailError::InvalidFormat));
    }

    #[test]
    fn rejects_no_domain() {
        let err = Email::try_new("user@").unwrap_err();
        assert!(matches!(err, EmailError::InvalidFormat));
    }

    #[test]
    fn rejects_no_tld() {
        let err = Email::try_new("user@domain").unwrap_err();
        assert!(matches!(err, EmailError::InvalidFormat));
    }

    #[test]
    fn rejects_single_char_tld() {
        let err = Email::try_new("user@domain.x").unwrap_err();
        assert!(matches!(err, EmailError::InvalidFormat));
    }

    #[test]
    fn rejects_empty() {
        let err = Email::try_new("").unwrap_err();
        assert!(matches!(err, EmailError::InvalidFormat));
    }

    #[test]
    fn rejects_too_long() {
        let local = "a".repeat(250);
        let too_long = format!("{local}@a.bc"); // 250 + 1 + 4 = 255
        assert_eq!(too_long.len(), 255);
        let err = Email::try_new(&too_long).unwrap_err();
        assert!(matches!(err, EmailError::TooLong { actual: 255 }));
    }

    #[test]
    fn accepts_exactly_254_chars() {
        let local = "a".repeat(249);
        let max = format!("{local}@a.bc"); // 249 + 1 + 4 = 254
        assert_eq!(max.len(), 254);
        let e = Email::try_new(&max).expect("254 is allowed");
        assert_eq!(e.as_str().len(), 254);
    }

    #[test]
    fn display_round_trips() {
        use std::fmt::Write;
        let e = Email::try_new("user@example.com").expect("ok");
        let mut out = String::new();
        write!(out, "{e}").expect("write");
        assert_eq!(out, "user@example.com");
    }

    #[test]
    fn from_str_round_trips() {
        use std::str::FromStr;
        let e = Email::from_str("USER@EXAMPLE.COM").expect("ok");
        assert_eq!(e.as_str(), "user@example.com");
    }
}
