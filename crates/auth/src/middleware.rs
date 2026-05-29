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

/// 미들웨어 의존 — `verifier` + `user_repo` + optional `jti_denylist`.
#[derive(Clone)]
pub struct AuthState {
    /// 토큰 검증기 (`Real` 또는 `Dev`).
    pub verifier: Arc<Verifier>,
    /// `User` 저장소.
    pub user_repo: Arc<dyn UserRepository>,
    /// `JTI` denylist (`SP6-i`) — `None` 이면 검증 skip (fail-open).
    pub jti_denylist: Option<Arc<dyn crate::jti_denylist::JtiDenylist>>,
    /// audit 2026-05-08: production 모드 여부. `true` 시 denylist Redis error → fail-closed
    /// (Expired). `false` (dev) 시 fail-open — gracefulm degradation 으로 dev UX 보존.
    pub fail_closed_on_denylist_error: bool,
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
// audit 2026-05-08: env-aware fail-closed 분기 추가 → cognitive complexity 20/15.
// 분해 시 *분기* 가 흩어져 흐름 추적 어려움 — 의식적 allow.
#[allow(clippy::cognitive_complexity)]
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

    // SP6-i: JTI denylist (logout / refresh rotation / role change 시 즉시 무효).
    //
    // 정책 (audit 2026-05-08 — environment-aware):
    // - production (`fail_closed_on_denylist_error = true`): Redis error → 401 (fail-closed).
    //   logout 후 revoked token 의 5분 window 동안 재사용 risk 차단. cascade 장애 시
    //   사용자 401 — observability (Sentry) 가 즉시 alert.
    // - dev (`false`): Redis 장애 시 fail-open — graceful degradation 으로 dev UX 보존.
    //   mitigation: access_token TTL 5분 + JWT signature 검증 + audit_log warn.
    if let Some(dl) = &state.jti_denylist {
        match dl.is_denied(&claims.jti).await {
            Ok(true) => return Err(AuthError::Expired),
            Ok(false) => {}
            Err(e) => {
                if state.fail_closed_on_denylist_error {
                    tracing::error!(
                        error = %e,
                        jti = %claims.jti,
                        "jti denylist check failed (fail-CLOSED, production)"
                    );
                    return Err(AuthError::Expired);
                }
                tracing::warn!(
                    error = %e,
                    jti = %claims.jti,
                    "jti denylist check failed (fail-open, dev)"
                );
            }
        }
    }

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

    let user = provision_new_user(state, claims).await?;
    Ok(user)
}

/// 존재하지 않는 sub 에 대해 `User` 를 생성·저장하고 `external_account` 를 link.
// first-sign-in 흐름 특성상 race retry + best-effort side-effects 로 복잡도 불가피.
#[allow(clippy::cognitive_complexity)]
async fn provision_new_user(state: &AuthState, claims: &Claims) -> Result<User, AuthError> {
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
        return state
            .user_repo
            .find_by_zitadel_sub(&claims.sub)
            .await
            .map_err(|e| AuthError::UserProvisioningFailed(e.to_string()))?
            .ok_or_else(|| AuthError::UserProvisioningFailed(save_err.to_string()));
    }

    // SP6-i: first sign-in 시 external_account('zitadel') 행 삽입. best-effort.
    if let Err(e) = state
        .user_repo
        .link_zitadel_account(&user.id, &claims.sub)
        .await
    {
        tracing::warn!(error = %e, user_id = %user.id, "external_account zitadel insert failed (best-effort)");
    }

    Ok(user)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::middleware::from_fn_with_state;
    use axum::routing::get;
    use axum::{Extension, Router};
    use shared_kernel::id::UserMarker;
    use tower::ServiceExt;
    use user_domain::repository::RepoError;

    use super::*;
    use crate::claims::Audience;
    use crate::jti_denylist::{JtiDenylist, JtiError};

    #[derive(Default)]
    struct RepoState {
        users_by_sub: HashMap<String, User>,
        fail_save: bool,
        save_calls: usize,
        link_calls: usize,
    }

    #[derive(Clone, Default)]
    struct FakeUserRepo {
        state: Arc<Mutex<RepoState>>,
    }

    impl FakeUserRepo {
        fn insert(&self, user: User) {
            self.state
                .lock()
                .expect("repo state lock")
                .users_by_sub
                .insert(user.zitadel_sub.clone(), user);
        }

        fn fail_save(&self) {
            self.state.lock().expect("repo state lock").fail_save = true;
        }

        fn save_calls(&self) -> usize {
            self.state.lock().expect("repo state lock").save_calls
        }

        fn link_calls(&self) -> usize {
            self.state.lock().expect("repo state lock").link_calls
        }
    }

    #[async_trait]
    impl UserRepository for FakeUserRepo {
        async fn find_by_id(&self, id: &Id<UserMarker>) -> Result<Option<User>, RepoError> {
            Ok(self
                .state
                .lock()
                .expect("repo state lock")
                .users_by_sub
                .values()
                .find(|user| &user.id == id)
                .cloned())
        }

        async fn find_by_zitadel_sub(&self, sub: &str) -> Result<Option<User>, RepoError> {
            Ok(self
                .state
                .lock()
                .expect("repo state lock")
                .users_by_sub
                .get(sub)
                .cloned())
        }

        async fn find_by_email(&self, email: &Email) -> Result<Option<User>, RepoError> {
            Ok(self
                .state
                .lock()
                .expect("repo state lock")
                .users_by_sub
                .values()
                .find(|user| &user.email == email)
                .cloned())
        }

        async fn save(&self, user: &User, _ctx: MutationContext) -> Result<(), RepoError> {
            let mut state = self.state.lock().expect("repo state lock");
            state.save_calls += 1;
            if state.fail_save {
                return Err(RepoError::Database("save failed".to_owned()));
            }
            state
                .users_by_sub
                .insert(user.zitadel_sub.clone(), user.clone());
            drop(state);
            Ok(())
        }

        async fn link_zitadel_account(
            &self,
            _user_id: &Id<UserMarker>,
            _zitadel_sub: &str,
        ) -> Result<(), RepoError> {
            self.state.lock().expect("repo state lock").link_calls += 1;
            Ok(())
        }
    }

    #[derive(Clone, Copy)]
    enum DenyMode {
        Allowed,
        Denied,
        Error,
    }

    struct FakeDenylist {
        mode: DenyMode,
    }

    #[async_trait]
    impl JtiDenylist for FakeDenylist {
        async fn is_denied(&self, _jti: &str) -> Result<bool, JtiError> {
            match self.mode {
                DenyMode::Allowed => Ok(false),
                DenyMode::Denied => Ok(true),
                DenyMode::Error => Err(JtiError::Redis("unavailable".to_owned())),
            }
        }

        async fn deny(&self, _jti: &str, _ttl_sec: u64) -> Result<(), JtiError> {
            Ok(())
        }
    }

    fn claims(sub: &str) -> Claims {
        Claims {
            sub: sub.to_owned(),
            email: Some(format!("{sub}@example.com")),
            name: Some(format!("{sub} name")),
            preferred_username: None,
            jti: format!("jti-{sub}"),
            exp: i64::MAX,
            nbf: None,
            iss: "dev-mode".to_owned(),
            aud: Audience::Single("dev-mode".to_owned()),
        }
    }

    fn user(sub: &str) -> User {
        User::try_new(
            Id::new(),
            sub,
            Email::try_new(&format!("{sub}@example.com")).expect("valid email"),
            sub,
            UserKind::Individual,
            Utc::now(),
        )
        .expect("valid user")
    }

    fn state(repo: FakeUserRepo) -> AuthState {
        AuthState {
            verifier: Arc::new(Verifier::Dev),
            user_repo: Arc::new(repo),
            jti_denylist: None,
            fail_closed_on_denylist_error: true,
        }
    }

    fn state_with_denylist(repo: FakeUserRepo, mode: DenyMode, fail_closed: bool) -> AuthState {
        AuthState {
            verifier: Arc::new(Verifier::Dev),
            user_repo: Arc::new(repo),
            jti_denylist: Some(Arc::new(FakeDenylist { mode })),
            fail_closed_on_denylist_error: fail_closed,
        }
    }

    async fn current_user(Extension(auth): Extension<AuthenticatedUser>) -> String {
        auth.user.zitadel_sub
    }

    async fn call_app(state: AuthState, authorization: Option<&str>) -> (StatusCode, String) {
        let app = Router::new()
            .route("/", get(current_user))
            .route_layer(from_fn_with_state(state, auth_layer));
        let mut request = Request::builder().uri("/");
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        let response = app
            .oneshot(request.body(Body::empty()).expect("request body"))
            .await
            .expect("response");
        let status = response.status();
        let body = to_bytes(response.into_body(), 4096)
            .await
            .expect("response body");
        (
            status,
            String::from_utf8(body.to_vec()).expect("response body utf8"),
        )
    }

    #[tokio::test]
    async fn auth_layer_rejects_missing_authorization_header() {
        let (status, body) = call_app(state(FakeUserRepo::default()), None).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("AUTH_MISSING_TOKEN"));
    }

    #[tokio::test]
    async fn auth_layer_rejects_invalid_authorization_format() {
        let (status, body) = call_app(state(FakeUserRepo::default()), Some("Basic abc")).await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("AUTH_INVALID_FORMAT"));
    }

    #[tokio::test]
    async fn auth_layer_injects_existing_user() {
        let repo = FakeUserRepo::default();
        repo.insert(user("existing-sub"));

        let (status, body) = call_app(state(repo.clone()), Some("Bearer DEV.existing-sub")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "existing-sub");
        assert_eq!(repo.save_calls(), 0);
    }

    #[tokio::test]
    async fn auth_layer_provisions_new_user_and_links_account() {
        let repo = FakeUserRepo::default();

        let (status, body) = call_app(state(repo.clone()), Some("Bearer DEV.new-sub")).await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "new-sub");
        assert_eq!(repo.save_calls(), 1);
        assert_eq!(repo.link_calls(), 1);
    }

    #[tokio::test]
    async fn auth_layer_rejects_denied_jti() {
        let repo = FakeUserRepo::default();
        repo.insert(user("denied-sub"));

        let (status, body) = call_app(
            state_with_denylist(repo, DenyMode::Denied, true),
            Some("Bearer DEV.denied-sub"),
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("AUTH_TOKEN_EXPIRED"));
    }

    #[tokio::test]
    async fn auth_layer_fail_closed_rejects_denylist_errors() {
        let repo = FakeUserRepo::default();
        repo.insert(user("closed-sub"));

        let (status, body) = call_app(
            state_with_denylist(repo, DenyMode::Error, true),
            Some("Bearer DEV.closed-sub"),
        )
        .await;

        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert!(body.contains("AUTH_TOKEN_EXPIRED"));
    }

    #[tokio::test]
    async fn auth_layer_fail_open_allows_denylist_errors_in_dev() {
        let repo = FakeUserRepo::default();
        repo.insert(user("open-sub"));

        let (status, body) = call_app(
            state_with_denylist(repo, DenyMode::Error, false),
            Some("Bearer DEV.open-sub"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "open-sub");
    }

    #[tokio::test]
    async fn resolve_or_create_user_returns_existing_user() {
        let repo = FakeUserRepo::default();
        repo.insert(user("existing-direct"));
        let state = state(repo.clone());

        let resolved = resolve_or_create_user(&state, &claims("existing-direct"))
            .await
            .expect("resolved user");

        assert_eq!(resolved.zitadel_sub, "existing-direct");
        assert_eq!(repo.save_calls(), 0);
    }

    #[tokio::test]
    async fn provision_new_user_requires_email_or_username() {
        let repo = FakeUserRepo::default();
        let state = state(repo);
        let mut claims = claims("missing-email");
        claims.email = None;
        claims.preferred_username = None;

        let Err(err) = provision_new_user(&state, &claims).await else {
            panic!("expected provisioning error");
        };

        assert!(matches!(err, AuthError::UserProvisioningFailed(_)));
    }

    #[tokio::test]
    async fn provision_new_user_refetches_existing_user_after_save_race() {
        let repo = FakeUserRepo::default();
        repo.insert(user("race-sub"));
        repo.fail_save();
        let state = state(repo.clone());

        let resolved = provision_new_user(&state, &claims("race-sub"))
            .await
            .expect("refetched user");

        assert_eq!(resolved.zitadel_sub, "race-sub");
        assert_eq!(repo.save_calls(), 1);
        assert_eq!(repo.link_calls(), 0);
    }

    #[tokio::test]
    async fn provision_new_user_reports_save_error_when_race_refetch_misses() {
        let repo = FakeUserRepo::default();
        repo.fail_save();
        let state = state(repo);

        let Err(err) = provision_new_user(&state, &claims("missing-after-save")).await else {
            panic!("expected provisioning error");
        };

        assert!(matches!(err, AuthError::UserProvisioningFailed(_)));
    }

    #[tokio::test]
    async fn denylist_allowed_path_continues_to_handler() {
        let repo = FakeUserRepo::default();
        repo.insert(user("allowed-sub"));

        let (status, body) = call_app(
            state_with_denylist(repo, DenyMode::Allowed, true),
            Some("Bearer DEV.allowed-sub"),
        )
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "allowed-sub");
    }
}
