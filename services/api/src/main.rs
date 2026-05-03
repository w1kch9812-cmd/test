//! 공짱 `HTTP` `API` service — `Walking Skeleton` + `Auth` (`SP3`).
//!
//! 라우트:
//! - `GET /healthz` — public liveness probe
//! - `GET /users/me` — 인증된 자신 조회 (`AuthenticatedUser` extractor)
//! - `GET /users/:id` — 인증된 자신만 (`auth.user.id == path id`), 다른 id 는 `403`
//!
//! `POST /users` 는 제거 — first-sign-in 자동 생성으로 대체.

#![forbid(unsafe_code)]
// `main.rs`: init failure panic은 정답이라 expect/unwrap 허용해요.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::env;
use std::sync::Arc;

use auth::jwks_cache::JwksCache;
use auth::middleware::{auth_layer, AuthState, AuthenticatedUser};
use auth::verifier::JwtVerifier;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{middleware, Json, Router};
use db::user::PgUserRepository;
use serde::Serialize;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

/// `Axum` 핸들러에 주입할 공유 상태.
#[derive(Clone)]
struct AppState {
    user_repo: Arc<dyn UserRepository>,
}

/// `User` 응답 직렬화 형태.
#[derive(Serialize)]
struct UserResponse {
    id: String,
    zitadel_sub: String,
    email: String,
    display_name: String,
    user_kind: String,
    roles: Vec<String>,
    created_at: String,
    updated_at: String,
    version: i64,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id.as_str().to_owned(),
            zitadel_sub: u.zitadel_sub,
            email: u.email.as_str().to_owned(),
            display_name: u.display_name,
            user_kind: match u.user_kind {
                UserKind::Individual => "individual".to_owned(),
                UserKind::Corporation => "corporation".to_owned(),
            },
            roles: u.roles.iter().map(|r| r.as_str().to_owned()).collect(),
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
            version: u.version,
        }
    }
}

/// `GET /healthz` — liveness probe (`DB` 미접속).
async fn health() -> &'static str {
    "ok"
}

/// `GET /users/me` — 인증된 사용자 자신 조회.
async fn me(auth: AuthenticatedUser) -> Json<UserResponse> {
    Json(auth.user.into())
}

/// `GET /users/:id` — `auth.user.id == path id` 인 경우만 허용 (`403` otherwise).
///
/// 다른 사용자 조회 권한은 후속 sub-project 에서 admin/operator 역할에 부여.
async fn get_user(
    State(state): State<AppState>,
    auth: AuthenticatedUser,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let id = Id::<UserMarker>::try_from_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid id: {e}")))?;
    if id.as_str() != auth.user.id.as_str() {
        return Err((
            StatusCode::FORBIDDEN,
            "이 사용자 정보는 조회할 권한이 없어요".to_owned(),
        ));
    }
    let user = state
        .user_repo
        .find_by_id(&id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("find failed: {e}"),
            )
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "user not found".to_owned()))?;
    Ok(Json(user.into()))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let issuer = env::var("ZITADEL_ISSUER").expect("ZITADEL_ISSUER must be set");
    let audience = env::var("ZITADEL_AUDIENCE").expect("ZITADEL_AUDIENCE must be set");
    let jwks_url = format!("{issuer}/oauth/v2/keys");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool));
    let app_state = AppState {
        user_repo: user_repo.clone(),
    };

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("reqwest");
    let jwks = Arc::new(JwksCache::new(jwks_url, http));
    let verifier = Arc::new(JwtVerifier::new(issuer, audience, jwks));
    let auth_state = AuthState {
        verifier,
        user_repo,
    };

    let public: Router<()> = Router::new().route("/healthz", get(health));
    let protected: Router<()> = Router::new()
        .route("/users/me", get(me))
        .route("/users/:id", get(get_user))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));

    let app = public.merge(protected).layer(TraceLayer::new_for_http());

    let addr = "0.0.0.0:8080";
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
