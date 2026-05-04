//! Axum tower middleware — `Bearer` 추출 → verify → `User` 자동 생성 → `Extension` 주입.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use shared_kernel::email::Email;
use shared_kernel::id::Id;
use shared_kernel::mutation::MutationContext;
use tracing::warn;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

use crate::claims::Claims;
use crate::errors::AuthError;
use crate::verifier::Verifier;

/// 핸들러로 주입되는 인증된 사용자 컨텍스트.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// `User` `Aggregate` (`find_by_zitadel_sub` 또는 자동 생성).
    pub user: User,
    /// 검증 통과한 `JWT` `Claims`.
    pub claims: Claims,
}

/// 미들웨어 의존 — `verifier` + `user_repo`.
#[derive(Clone)]
pub struct AuthState {
    /// 토큰 검증기 (`Real` 또는 `Dev`).
    pub verifier: Arc<Verifier>,
    /// `User` 저장소.
    pub user_repo: Arc<dyn UserRepository>,
}

/// `Bearer <jwt>` 검증 + `User` 자동 생성 + `Extension<AuthenticatedUser>` 주입.
///
/// `Authorization` 헤더에서 `Bearer ` 접두 토큰을 꺼내 [`Verifier`] 로 검증한 뒤,
/// `zitadel_sub` 으로 [`UserRepository`] 를 조회해요. 처음 보는 sub 이면 `User` 를
/// 자동 생성해 저장하고, 동시 첫 로그인으로 인한 save 충돌이 발생하면 한 번 더
/// fetch 해서 흡수해요. 인증된 컨텍스트는 [`AuthenticatedUser`] 로 요청 extension
/// 에 주입돼 다운스트림 핸들러에서 추출돼요.
///
/// # Errors
///
/// 모든 인증 실패는 [`AuthError`] 로 매핑되어 `IntoResponse` 됩니다.
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

async fn resolve_or_create_user(state: &AuthState, claims: &Claims) -> Result<User, AuthError> {
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
        now,
    )
    .map_err(|e| AuthError::UserProvisioningFailed(format!("domain validation: {e}")))?;

    // SP5-iv: first-sign-in 은 시스템 액션 — `actor_id = None`, `action = "first_sign_in"`.
    // `correlation_id` 는 zitadel sub (HTTP request_id 자동 주입은 SP7 후속).
    let ctx = MutationContext::new_system_action(claims.sub.clone(), "first_sign_in")
        .with_metadata(serde_json::json!({"zitadel_sub": &claims.sub}));

    // race: 동시 첫 로그인 — save 실패 시 fetch 재시도
    if let Err(save_err) = state.user_repo.save(&user, ctx).await {
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
