# Sub-project 3: Auth — Zitadel JWT 핵심 게이트 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **CRITICAL pre-read:** [memory/feedback_subproject_2a_lessons.md](../../../memory/feedback_subproject_2a_lessons.md) + [memory/project_progress.md](../../../memory/project_progress.md) + [docs/superpowers/specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md](../specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md)

**Goal:** Zitadel access_token JWT 검증 미들웨어 구축 + Walking Skeleton API 인증 게이트 적용 + CI 워크플로우에 진짜 Zitadel 컨테이너 통합.

**Architecture:** 신규 `crates/auth/` crate 가 검증·미들웨어·extractor 를 제공하고, `services/api` 가 그것을 tower layer 로 적용해요. `User` 도메인은 SP1/SP2 에서 이미 `roles`/`find_by_zitadel_sub` 를 갖고 있어 도메인 변경은 불필요.

**Tech Stack:** Rust 1.88, Axum 0.7, tower 0.5, tower-http 0.6, jsonwebtoken 9, reqwest 0.12, async-trait, Zitadel (셀프호스트, ghcr.io/zitadel/zitadel:latest).

---

## Spec → 현실 정정 (plan 작성 중 발견)

Spec § 4.2 / § 4.3 / § 4.4 는 다음을 *추가 작업*으로 적었으나 SP1/SP2 산출물에 *이미* 있어요. 본 plan 은 정정된 사실을 반영해 task 를 줄였어요:

1. `UserRole` enum — 코드는 7 variants (`Buyer`/`Seller`/`Broker`/`Developer`/`Enterprise`/`Operator`/`Admin`). Spec 의 5 는 로드맵 누락. 본 plan 은 7 로 진행.
2. `User.roles: Vec<UserRole>` 필드 — 이미 `crates/domain/core/user/src/entity.rs:94` 에 존재. `add_role`/`remove_role`/`has_role` 도 있음.
3. `UserRepository::find_by_zitadel_sub` — 이미 trait `crates/domain/core/user/src/repository.rs:28` + `PgUserRepository` 구현 존재.
4. `user.roles text[]` 컬럼 + GIN 인덱스 — 이미 `migrations/10001_core_tables.sql:15,26` 에 존재. 본 plan T8 은 *추가 CHECK 제약* 만 위한 신규 마이그레이션 `30005_user_roles_check.sql`.

Spec 자체는 수정하지 않고 본 plan 의 본 절이 진실 출처. 후속 sub-project 종료 시 spec 본문 동기화 (Spec FU 12).

---

## File Structure

신규 작성:
```
crates/auth/
├── Cargo.toml                    (name = "auth", deps: jsonwebtoken, reqwest, tower, axum, tokio, ...)
├── README.md
└── src/
    ├── lib.rs                    (pub mod 선언 + crate-level rustdoc)
    ├── errors.rs                 (AuthError enum + IntoResponse impl)
    ├── claims.rs                 (Claims struct + serde + tests)
    ├── jwks_cache.rs             (JwksCache + 1h TTL + lazy refetch)
    ├── verifier.rs               (JwtVerifier::new + verify)
    ├── middleware.rs             (axum::middleware::from_fn_with_state 함수)
    ├── extractor.rs              (AuthenticatedUser FromRequestParts)
    └── role_guard.rs             (require_role + RequireRole guard struct)

migrations/30005_user_roles_check.sql   (CHECK 제약 추가)

.github/workflows/walking-skeleton.yml  (수정 — Zitadel 서비스 추가)
tests/walking-skeleton/zitadel-setup.sh (신규 — Zitadel 초기 setup + JWT 발급)
```

수정:
```
Cargo.toml                                    (workspace member: crates/auth, workspace.deps: jsonwebtoken, reqwest)
services/api/Cargo.toml                       (auth dep 추가)
services/api/src/main.rs                      (verifier 초기화 + 라우터 분리 + middleware 적용 + POST /users 제거)
crates/domain/core/user/src/entity.rs         (Spec FU 12 — 코드 변경 없음, doc 만 갱신)
.env.example                                  (ZITADEL_ISSUER, ZITADEL_AUDIENCE 추가)
```

---

## Task 분해 (9 task)

- **Phase A (T1-T4):** auth crate 구축 — errors / claims / jwks_cache / verifier
- **Phase B (T5-T6):** middleware + extractor + role guard
- **Phase C (T7):** services/api 통합
- **Phase D (T8):** DB CHECK 제약 마이그레이션
- **Phase E (T9):** CI walking-skeleton에 Zitadel 컨테이너 통합 + e2e
- **Phase F (T10):** 통합 검증 + project_progress 갱신

> 9 task 로 시작했으나 T8 (마이그) 추가하며 10 으로 늘림. 모든 task 는 fresh subagent 로 dispatch.

**환경 한계:** Windows 로컬 빌드 불가 (MSVC 부재) — 모든 검증은 CI Linux 가 진실. TDD 스텝의 "테스트 실행" 은 commit + push + `gh run watch` 로 대체.

---

## Phase A: `crates/auth/` 기반

### Task 1: `auth` crate skeleton + `AuthError`

**Files:**
- Create: `crates/auth/Cargo.toml`
- Create: `crates/auth/README.md`
- Create: `crates/auth/src/lib.rs`
- Create: `crates/auth/src/errors.rs`
- Modify: `Cargo.toml` (workspace member + workspace.dependencies 항목)

- [ ] **Step 1: workspace 멤버 + workspace deps 추가**

`Cargo.toml`:
```toml
# [workspace] members 끝부분 (operations-meta 다음)
members = [
    # ... 기존 ...
    "crates/operations/operations-meta",
    "crates/auth",
    # ...
]

# [workspace.dependencies] 추가
[workspace.dependencies]
# ... 기존 ...
jsonwebtoken = "9"
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls", "json"] }
```

- [ ] **Step 2: `crates/auth/Cargo.toml` 작성**

```toml
[package]
name = "auth"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license = "Apache-2.0"
description = "공짱 Auth — Zitadel JWT 검증 미들웨어 (sub-project 3)"

[dependencies]
shared-kernel = { path = "../domain/core/shared-kernel", version = "0.1.0" }
user-domain = { path = "../domain/core/user", version = "0.1.0" }
async-trait = { workspace = true }
axum = { workspace = true }
chrono = { workspace = true }
jsonwebtoken = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tokio = { workspace = true, features = ["sync", "time"] }
tower = "0.5"
tracing = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }

[lints]
workspace = true
```

- [ ] **Step 3: `crates/auth/src/lib.rs` 작성**

```rust
//! 공짱 인증 핵심 게이트 — Zitadel access_token JWT 검증.
//!
//! - [`verifier::JwtVerifier`] — JWKS 캐시 + 서명·exp·iss·aud 검증
//! - [`middleware`] — Axum tower layer (Bearer → Extension<AuthenticatedUser>)
//! - [`extractor::AuthenticatedUser`] — 핸들러용 extractor
//! - [`role_guard::require_role`] — `Role` 가드 helper
//!
//! Spec: [`docs/superpowers/specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md`]

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod claims;
pub mod errors;
pub mod extractor;
pub mod jwks_cache;
pub mod middleware;
pub mod role_guard;
pub mod verifier;
```

- [ ] **Step 4: `crates/auth/src/errors.rs` — 실패 테스트 + 구현**

전체 코드 (1 파일에 테스트 포함):

```rust
//! `AuthError` — `401`/`403`/`500` 매핑.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

/// 인증/인가 실패 종류.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AuthError {
    /// `Authorization` 헤더가 없어요.
    #[error("missing Authorization header")]
    MissingToken,
    /// `Authorization` 헤더가 `Bearer ` 접두사로 시작하지 않거나 토큰 본문이 비어있어요.
    #[error("invalid Authorization format")]
    InvalidFormat,
    /// `JWT` 파싱 실패 (`base64`/`JSON` 깨짐).
    #[error("malformed token")]
    MalformedToken,
    /// `kid` 헤더에 매칭되는 공개키가 `JWKS` 에 없어요.
    #[error("unknown signing key (kid not found)")]
    UnknownKey,
    /// 서명 검증 실패.
    #[error("invalid signature")]
    InvalidSignature,
    /// `exp` 만료.
    #[error("token expired")]
    Expired,
    /// `nbf` 미도래.
    #[error("token not yet valid")]
    NotYetValid,
    /// `iss` 불일치.
    #[error("invalid issuer")]
    InvalidIssuer,
    /// `aud` 불일치.
    #[error("invalid audience")]
    InvalidAudience,
    /// `sub` claim 누락.
    #[error("missing subject claim")]
    MissingSubject,
    /// `User` 자동 생성 실패 (`DB` 또는 도메인 검증).
    #[error("user provisioning failed: {0}")]
    UserProvisioningFailed(String),
    /// 역할 부족.
    #[error("insufficient role")]
    InsufficientRole,
    /// `JWKS` 페치 실패.
    #[error("JWKS fetch failed: {0}")]
    JwksFetchFailed(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error_code: &'static str,
    message: &'static str,
}

impl AuthError {
    /// 응답 코드 + 메시지 매핑 (spec § 6.1).
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::MissingToken => "AUTH_MISSING_TOKEN",
            Self::InvalidFormat => "AUTH_INVALID_FORMAT",
            Self::MalformedToken => "AUTH_MALFORMED_TOKEN",
            Self::UnknownKey => "AUTH_UNKNOWN_KEY",
            Self::InvalidSignature => "AUTH_INVALID_SIGNATURE",
            Self::Expired => "AUTH_TOKEN_EXPIRED",
            Self::NotYetValid => "AUTH_TOKEN_NOT_YET_VALID",
            Self::InvalidIssuer => "AUTH_INVALID_ISSUER",
            Self::InvalidAudience => "AUTH_INVALID_AUDIENCE",
            Self::MissingSubject => "AUTH_MISSING_SUBJECT",
            Self::UserProvisioningFailed(_) => "AUTH_USER_PROVISION_FAILED",
            Self::InsufficientRole => "AUTH_INSUFFICIENT_ROLE",
            Self::JwksFetchFailed(_) => "AUTH_INVALID_SIGNATURE",
        }
    }

    /// 한국어 해요체 메시지 (spec § 6.1).
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match self {
            Self::MissingToken => "인증 토큰이 필요해요",
            Self::InvalidFormat => "토큰 형식이 잘못됐어요",
            Self::MalformedToken => "토큰을 해석할 수 없어요",
            Self::UnknownKey => "토큰 서명 키를 찾을 수 없어요",
            Self::InvalidSignature | Self::JwksFetchFailed(_) => "토큰이 유효하지 않아요",
            Self::Expired => "토큰이 만료됐어요. 다시 로그인해 주세요",
            Self::NotYetValid => "토큰이 아직 사용할 수 없어요",
            Self::InvalidIssuer => "토큰 발급자가 일치하지 않아요",
            Self::InvalidAudience => "토큰 대상이 일치하지 않아요",
            Self::MissingSubject => "토큰에 사용자 정보가 없어요",
            Self::UserProvisioningFailed(_) => "사용자 등록에 실패했어요. 잠시 후 다시 시도해 주세요",
            Self::InsufficientRole => "이 작업을 수행할 권한이 부족해요",
        }
    }

    /// `HTTP` 상태 코드.
    #[must_use]
    pub const fn status(&self) -> StatusCode {
        match self {
            Self::InsufficientRole => StatusCode::FORBIDDEN,
            Self::UserProvisioningFailed(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::UNAUTHORIZED,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = ErrorBody {
            error_code: self.code(),
            message: self.message(),
        };
        (self.status(), Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn code_maps_each_variant() {
        assert_eq!(AuthError::MissingToken.code(), "AUTH_MISSING_TOKEN");
        assert_eq!(AuthError::Expired.code(), "AUTH_TOKEN_EXPIRED");
        assert_eq!(AuthError::InsufficientRole.code(), "AUTH_INSUFFICIENT_ROLE");
        assert_eq!(
            AuthError::UserProvisioningFailed("db".into()).code(),
            "AUTH_USER_PROVISION_FAILED"
        );
    }

    #[test]
    fn status_403_only_for_role() {
        assert_eq!(AuthError::InsufficientRole.status(), StatusCode::FORBIDDEN);
        assert_eq!(
            AuthError::UserProvisioningFailed("x".into()).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(AuthError::Expired.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(AuthError::MissingToken.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn message_uses_haeyo() {
        assert_eq!(AuthError::Expired.message(), "토큰이 만료됐어요. 다시 로그인해 주세요");
    }

    #[tokio::test]
    async fn into_response_shape() {
        let resp = AuthError::Expired.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let body = to_bytes(resp.into_body(), 1024).await.expect("body");
        let parsed: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(parsed["error_code"], "AUTH_TOKEN_EXPIRED");
        assert_eq!(parsed["message"], "토큰이 만료됐어요. 다시 로그인해 주세요");
    }
}
```

- [ ] **Step 5: `crates/auth/README.md` 작성**

```markdown
# auth

공짱 인증 핵심 게이트 — Zitadel access_token JWT 검증 + first-sign-in 자동 생성.

- [errors](src/errors.rs) — `AuthError` + `IntoResponse`
- [claims](src/claims.rs) — `Claims` struct
- [jwks_cache](src/jwks_cache.rs) — JWKS 1h TTL 캐시
- [verifier](src/verifier.rs) — `JwtVerifier`
- [middleware](src/middleware.rs) — Axum tower layer
- [extractor](src/extractor.rs) — `AuthenticatedUser`
- [role_guard](src/role_guard.rs) — `require_role`

Spec: [docs/superpowers/specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md](../../docs/superpowers/specs/2026-05-03-sub-project-3-auth-zitadel-jwt-design.md)
```

- [ ] **Step 6: 다른 모듈 stub 추가 (compile-only)**

각 module 파일은 비어있어도 `lib.rs` 의 `pub mod` 가 컴파일 통과하도록 최소 docstring + `pub fn _placeholder()` 패턴. 또는 빈 파일 + 모듈 doc-comment.

```rust
// crates/auth/src/claims.rs
//! `Claims` (placeholder, T2 에서 구현).
```

같은 식으로 jwks_cache, verifier, middleware, extractor, role_guard 모두.

- [ ] **Step 7: commit + push + watch CI**

```bash
git add Cargo.toml crates/auth/
git commit -m "feat(auth): crate skeleton + AuthError + IntoResponse mapping (SP3 T1)

- 9 module files (lib + 7 modules + errors)
- AuthError 13 variants → HTTP code + 한국어 해요체 메시지
- IntoResponse impl with JSON body
- jsonwebtoken + reqwest workspace deps 추가
- 4 unit tests"

git push
gh run list --branch main --limit 3
gh run watch <id> --exit-status
```

CI 그린 (3 workflow) 확인. 실패 시 일반 패턴: clippy::doc_markdown (백틱), clippy::missing_const_for_fn, fmt --check.

---

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

### Task 3: `JwksCache` (1h TTL + lazy refetch)

**Files:**
- Modify: `crates/auth/src/jwks_cache.rs`

- [ ] **Step 1: 구현 + 테스트**

```rust
//! `JWKS` 캐시 — `kid` → `DecodingKey`, `1h TTL`, lazy refetch.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::AuthError;

const TTL: Duration = Duration::from_secs(3600);

/// `JWKS` (JSON Web Key Set) 응답 wrapper.
#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    #[serde(default)]
    alg: Option<String>,
    #[allow(dead_code)]
    #[serde(default, rename = "use")]
    use_: Option<String>,
}

/// 캐시 entry — 키 + 페치 시각.
struct Entry {
    key: Arc<DecodingKey>,
    fetched_at: Instant,
}

/// `JWKS` 캐시 (`kid` → `DecodingKey` + TTL).
pub struct JwksCache {
    jwks_url: String,
    http: reqwest::Client,
    entries: RwLock<HashMap<String, Entry>>,
}

impl JwksCache {
    /// 캐시 생성. 첫 페치는 `get_or_fetch` 호출 시 lazy 수행.
    #[must_use]
    pub fn new(jwks_url: String, http: reqwest::Client) -> Self {
        Self {
            jwks_url,
            http,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// `kid` 로 키 조회 — 캐시 만료 또는 미존재 시 `JWKS` 재페치.
    ///
    /// # Errors
    ///
    /// 페치 실패 → [`AuthError::JwksFetchFailed`].
    /// 페치 후에도 `kid` 없으면 [`AuthError::UnknownKey`].
    pub async fn get_or_fetch(&self, kid: &str) -> Result<Arc<DecodingKey>, AuthError> {
        // hot path: 캐시 hit + TTL 살아있음
        {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(kid) {
                if entry.fetched_at.elapsed() < TTL {
                    return Ok(entry.key.clone());
                }
            }
        }
        // miss or expired — 페치
        self.refetch().await?;
        let entries = self.entries.read().await;
        entries
            .get(kid)
            .map(|e| e.key.clone())
            .ok_or(AuthError::UnknownKey)
    }

    async fn refetch(&self) -> Result<(), AuthError> {
        let resp = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
        let jwks: Jwks = resp
            .json()
            .await
            .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.clear();
        for k in jwks.keys {
            if k.kty != "RSA" {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&k.n, &k.e)
                .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
            entries.insert(
                k.kid,
                Entry {
                    key: Arc::new(key),
                    fetched_at: now,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn ttl_one_hour() {
        assert_eq!(TTL.as_secs(), 3600);
    }

    #[tokio::test]
    async fn miss_returns_unknown_key_when_jwks_unreachable() {
        let cache = JwksCache::new(
            "http://127.0.0.1:1/jwks".into(), // 의도적으로 닫힌 포트
            reqwest::Client::new(),
        );
        let err = cache.get_or_fetch("any-kid").await.unwrap_err();
        assert!(matches!(err, AuthError::JwksFetchFailed(_)));
    }
}
```

> **참고:** 더 자세한 캐시 로직 단위 테스트 (lazy refetch 동작, kid 매칭) 는 mock HTTP server 필요 — wiremock 의존성 추가 비용 고려해 본 plan 은 통합 테스트 (T9 CI) 에서 진짜 Zitadel JWKS 로 검증.

- [ ] **Step 2: commit + push + CI 그린 확인**

```bash
git add crates/auth/src/jwks_cache.rs
git commit -m "feat(auth): JwksCache 1h TTL + lazy refetch + reqwest fetch (SP3 T3)

- JwksCache::get_or_fetch — hot path read-lock, miss/expired 시 write-lock 페치
- RSA only (kty='RSA'), DecodingKey::from_rsa_components(n, e)
- AuthError::JwksFetchFailed mapping
- 2 단위 테스트 (TTL 상수 + 페치 실패 경로); JWKS 매칭은 T9 e2e 로 검증"
git push
```

---

### Task 4: `JwtVerifier`

**Files:**
- Modify: `crates/auth/src/verifier.rs`

- [ ] **Step 1: 구현 + 테스트**

```rust
//! `JWT` 검증기 — RS256 + JWKS + iss/aud/exp/nbf.

use std::sync::Arc;

use jsonwebtoken::{decode, decode_header, Algorithm, Validation};

use crate::claims::Claims;
use crate::errors::AuthError;
use crate::jwks_cache::JwksCache;

/// Zitadel `JWT` 검증기.
pub struct JwtVerifier {
    issuer: String,
    audience: String,
    jwks: Arc<JwksCache>,
}

impl JwtVerifier {
    /// 검증기 생성. JWKS 페치는 첫 verify 호출 시 lazy 수행.
    #[must_use]
    pub fn new(issuer: String, audience: String, jwks: Arc<JwksCache>) -> Self {
        Self {
            issuer,
            audience,
            jwks,
        }
    }

    /// `JWT` 토큰을 검증해 [`Claims`] 를 반환해요.
    ///
    /// # Errors
    ///
    /// - 헤더 파싱 실패 → [`AuthError::MalformedToken`]
    /// - `kid` 없음 → [`AuthError::UnknownKey`]
    /// - 서명 실패 → [`AuthError::InvalidSignature`]
    /// - `exp` 만료 → [`AuthError::Expired`]
    /// - `iss` 불일치 → [`AuthError::InvalidIssuer`]
    /// - `aud` 불일치 → [`AuthError::InvalidAudience`]
    /// - `sub` 빈 값 → [`AuthError::MissingSubject`]
    pub async fn verify(&self, token: &str) -> Result<Claims, AuthError> {
        let header = decode_header(token).map_err(|_| AuthError::MalformedToken)?;
        if header.alg != Algorithm::RS256 {
            return Err(AuthError::InvalidSignature);
        }
        let kid = header.kid.ok_or(AuthError::UnknownKey)?;
        let key = self.jwks.get_or_fetch(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.validate_aud = false; // 직접 검증 (Audience::Single|Multiple)
        validation.leeway = 30; // clock skew 30s

        let data = decode::<Claims>(token, &key, &validation).map_err(|e| {
            use jsonwebtoken::errors::ErrorKind as E;
            match e.kind() {
                E::ExpiredSignature => AuthError::Expired,
                E::ImmatureSignature => AuthError::NotYetValid,
                E::InvalidIssuer => AuthError::InvalidIssuer,
                E::InvalidSignature => AuthError::InvalidSignature,
                _ => AuthError::InvalidSignature,
            }
        })?;

        if !data.claims.aud.contains(&self.audience) {
            return Err(AuthError::InvalidAudience);
        }
        if data.claims.sub.trim().is_empty() {
            return Err(AuthError::MissingSubject);
        }
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn malformed_token_returns_malformed() {
        let cache = Arc::new(JwksCache::new(
            "http://127.0.0.1:1/jwks".into(),
            reqwest::Client::new(),
        ));
        let v = JwtVerifier::new("http://issuer".into(), "aud".into(), cache);
        let err = v.verify("not-a-jwt").await.unwrap_err();
        assert_eq!(err, AuthError::MalformedToken);
    }
}
```

> 더 깊은 검증 (서명 / iss / aud / exp 분기) 는 T9 CI 의 진짜 Zitadel 토큰으로 검증.

- [ ] **Step 2: commit + push + CI 그린**

```bash
git add crates/auth/src/verifier.rs
git commit -m "feat(auth): JwtVerifier::verify — RS256 + iss/aud/exp + JWKS lookup (SP3 T4)

- Algorithm RS256만 허용 (downgrade attack 방지)
- jsonwebtoken Validation: iss check via lib, aud/sub 직접 검증
- clock skew 30s leeway
- ErrorKind → AuthError 매핑 (Expired/NotYetValid/InvalidIssuer/InvalidSignature)
- 1 happy-path 단위 테스트 (malformed); 깊은 검증은 T9 e2e"
git push
```

---

## Phase B: Middleware + extractor + role guard

