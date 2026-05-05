//! Zitadel `JWT` claims — `sub` / `email` / `name` / `exp` / `iss` / `aud` / `nbf`.

use serde::{Deserialize, Serialize};

/// Zitadel `access_token` claims (`OIDC` 표준 + 일부 옵션).
///
/// `aud` 는 단일 문자열 또는 배열 모두 허용 (Zitadel 은 배열로 발급).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    /// 사용자 식별자 (Zitadel `sub` claim, `UUID`).
    pub sub: String,
    /// 이메일.
    #[serde(default)]
    pub email: Option<String>,
    /// 표시 이름.
    #[serde(default)]
    pub name: Option<String>,
    /// `preferred_username` (`email` 대체용).
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// `JWT` ID — `JTI` denylist key. Zitadel 가 항상 발급.
    pub jti: String,
    /// 만료 (`epoch seconds`).
    pub exp: i64,
    /// 미발효 (`epoch seconds`, 옵션).
    #[serde(default)]
    pub nbf: Option<i64>,
    /// 발급자.
    pub iss: String,
    /// 대상 (단일 또는 배열).
    pub aud: Audience,
}

/// `aud` claim 은 `OIDC` 표준상 단일 문자열 또는 배열 모두 가능해요.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Audience {
    /// 단일 audience.
    Single(String),
    /// 다수 audience.
    Multiple(Vec<String>),
}

impl Audience {
    /// `expected` 가 audience 목록에 포함되는지 확인.
    #[must_use]
    pub fn contains(&self, expected: &str) -> bool {
        match self {
            Self::Single(s) => s == expected,
            Self::Multiple(v) => v.iter().any(|s| s == expected),
        }
    }
}

impl Claims {
    /// `email` 또는 `preferred_username` 중 사용 가능한 값.
    ///
    /// 둘 다 없으면 `None`.
    #[must_use]
    pub fn effective_email(&self) -> Option<&str> {
        self.email.as_deref().or(self.preferred_username.as_deref())
    }

    /// `name` → `preferred_username` → `sub` (앞 8 char) 순서로 fallback.
    #[must_use]
    pub fn effective_display_name(&self) -> String {
        if let Some(n) = &self.name {
            return n.clone();
        }
        if let Some(u) = &self.preferred_username {
            return u.clone();
        }
        self.sub.chars().take(8).collect()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn audience_single_contains() {
        let a = Audience::Single("client-123".into());
        assert!(a.contains("client-123"));
        assert!(!a.contains("other"));
    }

    #[test]
    fn audience_multiple_contains() {
        let a = Audience::Multiple(vec!["a".into(), "b".into()]);
        assert!(a.contains("a"));
        assert!(a.contains("b"));
        assert!(!a.contains("c"));
    }

    #[test]
    fn audience_empty_multiple_contains_nothing() {
        let a = Audience::Multiple(vec![]);
        assert!(!a.contains("x"));
    }

    #[test]
    fn deserialize_single_aud() {
        let json = r#"{"sub":"u1","jti":"j1","exp":1000,"iss":"http://i","aud":"client-x"}"#;
        let c: Claims = serde_json::from_str(json).expect("parse");
        assert!(matches!(c.aud, Audience::Single(ref s) if s == "client-x"));
        assert_eq!(c.sub, "u1");
        assert_eq!(c.jti, "j1");
        assert_eq!(c.exp, 1000);
        assert_eq!(c.iss, "http://i");
        assert_eq!(c.email, None);
        assert_eq!(c.nbf, None);
    }

    #[test]
    fn deserialize_multiple_aud() {
        let json = r#"{"sub":"u1","jti":"j1","exp":1000,"iss":"http://i","aud":["a","b"]}"#;
        let c: Claims = serde_json::from_str(json).expect("parse");
        assert!(matches!(c.aud, Audience::Multiple(ref v) if v.len() == 2));
    }

    #[test]
    fn deserialize_with_optional_fields() {
        let json = r#"{
            "sub":"u1",
            "jti":"j1",
            "email":"a@b.com",
            "name":"Alice",
            "exp":1000,
            "nbf":900,
            "iss":"http://i",
            "aud":"x"
        }"#;
        let c: Claims = serde_json::from_str(json).expect("parse");
        assert_eq!(c.email.as_deref(), Some("a@b.com"));
        assert_eq!(c.name.as_deref(), Some("Alice"));
        assert_eq!(c.nbf, Some(900));
    }

    #[test]
    fn effective_email_prefers_email_over_preferred_username() {
        let c = Claims {
            sub: "s".into(),
            email: Some("primary@example.com".into()),
            name: None,
            preferred_username: Some("alt@example.com".into()),
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_email(), Some("primary@example.com"));
    }

    #[test]
    fn effective_email_falls_back_to_preferred_username() {
        let c = Claims {
            sub: "s".into(),
            email: None,
            name: None,
            preferred_username: Some("alice@example.com".into()),
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_email(), Some("alice@example.com"));
    }

    #[test]
    fn effective_email_none_when_both_absent() {
        let c = Claims {
            sub: "s".into(),
            email: None,
            name: None,
            preferred_username: None,
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_email(), None);
    }

    #[test]
    fn effective_display_name_prefers_name() {
        let c = Claims {
            sub: "s".into(),
            email: None,
            name: Some("Alice".into()),
            preferred_username: Some("alt".into()),
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_display_name(), "Alice");
    }

    #[test]
    fn effective_display_name_falls_back_to_preferred_username() {
        let c = Claims {
            sub: "s".into(),
            email: None,
            name: None,
            preferred_username: Some("alt".into()),
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_display_name(), "alt");
    }

    #[test]
    fn effective_display_name_falls_back_to_sub_prefix() {
        let c = Claims {
            sub: "user-12345-abc".into(),
            email: None,
            name: None,
            preferred_username: None,
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_display_name(), "user-123");
    }

    #[test]
    fn effective_display_name_short_sub() {
        let c = Claims {
            sub: "ab".into(),
            email: None,
            name: None,
            preferred_username: None,
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_display_name(), "ab");
    }

    #[test]
    fn round_trip_serde() {
        let c = Claims {
            sub: "u1".into(),
            email: Some("e@x".into()),
            name: Some("Alice".into()),
            preferred_username: None,
            jti: "j1".into(),
            exp: 1000,
            nbf: Some(900),
            iss: "http://i".into(),
            aud: Audience::Multiple(vec!["a".into(), "b".into()]),
        };
        let json = serde_json::to_string(&c).expect("serialize");
        let back: Claims = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }
}
