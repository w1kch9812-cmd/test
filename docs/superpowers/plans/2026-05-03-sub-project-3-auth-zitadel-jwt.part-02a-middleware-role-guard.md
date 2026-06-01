# Sub-project 3 Auth Zitadel JWT - Part 02A: Middleware And Role Guard

Parent index: [Sub-project 3 Auth Zitadel JWT - Part 02](./2026-05-03-sub-project-3-auth-zitadel-jwt.part-02.md).
### Task 5: `AuthMiddleware`

**Files:**
- Modify: `crates/auth/src/middleware.rs`

- [ ] **Step 1: 구현**

```rust
//! Axum tower middleware — Bearer 추출 → verify → User 자동 생성 → Extension 주입.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use tracing::warn;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use crate::claims::Claims;
use crate::errors::AuthError;
use crate::verifier::JwtVerifier;

/// 핸들러로 주입되는 인증된 사용자 컨텍스트.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// `User` Aggregate (`find_by_zitadel_sub` 또는 자동 생성).
    pub user: User,
    /// 검증 통과한 `JWT` claims.
    pub claims: Claims,
}

/// 미들웨어 의존 — verifier + user repository.
#[derive(Clone)]
pub struct AuthState {
    /// `JWT` 검증기.
    pub verifier: Arc<JwtVerifier>,
    /// `User` 저장소.
    pub user_repo: Arc<dyn UserRepository>,
}

/// `Bearer <jwt>` 검증 + `User` 자동 생성 + `Extension<AuthenticatedUser>` 주입.
///
/// # Errors
///
/// 모든 인증 실패는 [`AuthError`] 로 매핑되어 `IntoResponse` 됨.
pub async fn auth_layer(
    State(state): State<AuthState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let header = req
        .headers()
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingToken)?;
    let header_str = header.to_str().map_err(|_| AuthError::InvalidFormat)?;
    let token = header_str
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidFormat)?
        .trim();
    if token.is_empty() {
        return Err(AuthError::InvalidFormat);
    }

    let claims = state.verifier.verify(token).await?;
    let user = resolve_or_create_user(&state, &claims).await?;
    req.extensions_mut().insert(AuthenticatedUser {
        user,
        claims: claims.clone(),
    });
    Ok(next.run(req).await)
}

async fn resolve_or_create_user(
    state: &AuthState,
    claims: &Claims,
) -> Result<User, AuthError> {
    if let Some(existing) = state
        .user_repo
        .find_by_zitadel_sub(&claims.sub)
        .await
        .map_err(|e| AuthError::UserProvisioningFailed(e.to_string()))?
    {
        return Ok(existing);
    }

    // 자동 생성
    let email_str = claims.effective_email().ok_or_else(|| {
        AuthError::UserProvisioningFailed("token has no email or preferred_username".into())
    })?;
    let email = Email::try_new(email_str)
        .map_err(|e| AuthError::UserProvisioningFailed(format!("invalid email: {e}")))?;
    let display = claims.effective_display_name();
    let now = Utc::now();
    let user = User::try_new(
        Id::new(),
        &claims.sub,
        email,
        &display,
        UserKind::Individual,
        vec![], // 역할 없음 (어드민이 별도 부여)
        now,
    )
    .map_err(|e| AuthError::UserProvisioningFailed(format!("domain validation: {e}")))?;

    // race: 동시 첫 로그인 — save unique 충돌 시 fetch 재시도
    if let Err(save_err) = state.user_repo.save(&user).await {
        warn!(?save_err, sub = %claims.sub, "save failed, retrying find");
        if let Some(existing) = state
            .user_repo
            .find_by_zitadel_sub(&claims.sub)
            .await
            .map_err(|e| AuthError::UserProvisioningFailed(e.to_string()))?
        {
            return Ok(existing);
        }
        return Err(AuthError::UserProvisioningFailed(save_err.to_string()));
    }
    Ok(user)
}
```

> `User::try_new` 실제 시그니처는 `crates/domain/core/user/src/entity.rs:163-228` 와 일치해야 함. `roles: Vec<UserRole>` 도 인자에 포함됨. 혹시 시그니처 불일치 시 컴파일 에러 메시지 따라 수정.

- [ ] **Step 2: commit + push + CI 그린**

```bash
git add crates/auth/src/middleware.rs
git commit -m "feat(auth): AuthMiddleware — Bearer + verify + auto-create User + Extension (SP3 T5)

- Header Authorization 파싱 (Bearer 접두 + 빈 토큰 거부)
- JwtVerifier.verify → Claims
- find_by_zitadel_sub → 없으면 User::try_new + save
- save unique 충돌 race → fetch retry
- email Email value object 검증
- AuthState { verifier, user_repo }"
git push
```

---

### Task 6: `AuthenticatedUser` extractor + `RoleGuard`

**Files:**
- Modify: `crates/auth/src/extractor.rs`
- Modify: `crates/auth/src/role_guard.rs`

- [ ] **Step 1: extractor 구현**

`crates/auth/src/extractor.rs`:

```rust
//! `AuthenticatedUser` extractor — middleware 주입한 `Extension` 을 핸들러용으로 노출.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AuthError::MissingToken)
    }
}
```

- [ ] **Step 2: role guard 구현**

`crates/auth/src/role_guard.rs`:

```rust
//! 역할 가드 — `require_role` helper.

use user_domain::entity::UserRole;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

/// `auth.user.roles` 가 `role` 을 포함하는지 확인.
///
/// # Errors
///
/// 미포함 → [`AuthError::InsufficientRole`].
pub fn require_role(auth: &AuthenticatedUser, role: UserRole) -> Result<(), AuthError> {
    if auth.user.roles.contains(&role) {
        Ok(())
    } else {
        Err(AuthError::InsufficientRole)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use user_domain::entity::{User, UserKind, UserRole};

    use crate::claims::{Audience, Claims};

    fn fixture(roles: Vec<UserRole>) -> AuthenticatedUser {
        let email = Email::try_new("a@b.com").expect("email");
        let user = User::try_new(
            Id::new(),
            "sub-1",
            email,
            "alice",
            UserKind::Individual,
            roles,
            Utc::now(),
        )
        .expect("user");
        let claims = Claims {
            sub: "sub-1".into(),
            email: Some("a@b.com".into()),
            name: Some("alice".into()),
            preferred_username: None,
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        AuthenticatedUser { user, claims }
    }

    #[test]
    fn allows_when_role_present() {
        let auth = fixture(vec![UserRole::Buyer]);
        assert!(require_role(&auth, UserRole::Buyer).is_ok());
    }

    #[test]
    fn denies_when_role_missing() {
        let auth = fixture(vec![UserRole::Buyer]);
        let err = require_role(&auth, UserRole::Admin).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }

    #[test]
    fn denies_when_no_roles() {
        let auth = fixture(vec![]);
        let err = require_role(&auth, UserRole::Buyer).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }
}
```

- [ ] **Step 3: commit + push + CI 그린**

```bash
git add crates/auth/src/extractor.rs crates/auth/src/role_guard.rs
git commit -m "feat(auth): AuthenticatedUser extractor + require_role guard (SP3 T6)

- FromRequestParts<AuthenticatedUser> from Extension
- require_role(auth, UserRole) -> Result<(), AuthError::InsufficientRole>
- 3 unit tests for role guard"
git push
```

---
