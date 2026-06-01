# Sub-project 3 Auth Zitadel JWT - Part 01B: Claims

Parent index: [Sub-project 3 Auth Zitadel JWT - Part 01](./2026-05-03-sub-project-3-auth-zitadel-jwt.part-01.md).

### Task 2: `Claims` struct

**Files:**
- Modify: `crates/auth/src/claims.rs`

- [ ] **Step 1: 테스트 + 구현**

```rust
//! Zitadel `JWT` claims — sub / email / name / exp / iss / aud / nbf.

use serde::{Deserialize, Serialize};

/// Zitadel access_token claims (`OIDC` 표준 + 일부 옵션).
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

/// `aud` claim 은 OIDC 표준상 단일 문자열 또는 배열 모두 가능해요.
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
    fn deserialize_single_aud() {
        let json = r#"{"sub":"u1","exp":1000,"iss":"http://i","aud":"client-x"}"#;
        let c: Claims = serde_json::from_str(json).expect("parse");
        assert!(matches!(c.aud, Audience::Single(ref s) if s == "client-x"));
    }

    #[test]
    fn deserialize_multiple_aud() {
        let json = r#"{"sub":"u1","exp":1000,"iss":"http://i","aud":["a","b"]}"#;
        let c: Claims = serde_json::from_str(json).expect("parse");
        assert!(matches!(c.aud, Audience::Multiple(ref v) if v.len() == 2));
    }

    #[test]
    fn effective_email_fallback() {
        let c = Claims {
            sub: "s".into(),
            email: None,
            name: None,
            preferred_username: Some("alice@example.com".into()),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_email(), Some("alice@example.com"));
    }

    #[test]
    fn effective_display_name_fallback_to_sub_prefix() {
        let c = Claims {
            sub: "user-12345-abc".into(),
            email: None,
            name: None,
            preferred_username: None,
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        assert_eq!(c.effective_display_name(), "user-123");
    }
}
```

- [ ] **Step 2: commit + push + watch CI**

```bash
git add crates/auth/src/claims.rs
git commit -m "feat(auth): Claims struct with sub/email/name/exp/iss/aud + tests (SP3 T2)

- Audience::Single | Multiple (OIDC 표준 — 둘 다 허용)
- effective_email / effective_display_name fallback chain
- 6 tests"
git push
```

CI 그린 확인.

---
