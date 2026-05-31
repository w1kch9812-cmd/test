#![allow(clippy::expect_used, clippy::panic)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::get;
use axum::{Extension, Router};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use tower::ServiceExt;
use user_domain::entity::{User, UserKind};
use user_domain::repository::{RepoError, UserRepository};

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
