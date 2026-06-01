# Sub-project 3 Auth Zitadel JWT - Part 01A: Overview And AuthError

Parent index: [Sub-project 3 Auth Zitadel JWT - Part 01](./2026-05-03-sub-project-3-auth-zitadel-jwt.part-01.md).
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
