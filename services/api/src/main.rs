//! 공짱 `HTTP` `API` service — Walking Skeleton.
//!
//! 3 endpoint:
//! - `GET /healthz` — liveness probe
//! - `POST /users` — `User` 생성
//! - `GET /users/:id` — `User` 조회
//!
//! 인증·관측성·에러 매핑 등은 sub-project 3, 5, 7에서. Walking Skeleton은
//! *작동 확인*만이 목표예요.

#![forbid(unsafe_code)]
// `main.rs`: init failure panic은 정답이라 expect/unwrap 허용해요.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::env;
use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use db::user::PgUserRepository;
use serde::{Deserialize, Serialize};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::time::now_utc;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

/// `Axum` 핸들러에 주입할 공유 상태.
#[derive(Clone)]
struct AppState {
    user_repo: Arc<dyn UserRepository>,
}

/// `POST /users` 요청 본문.
#[derive(Deserialize)]
struct CreateUserRequest {
    zitadel_sub: String,
    email: String,
    display_name: String,
    /// `"individual"` | `"corporation"`.
    user_kind: String,
}

/// `User` 응답 직렬화 형태.
#[derive(Serialize)]
struct UserResponse {
    id: String,
    zitadel_sub: String,
    email: String,
    display_name: String,
    user_kind: String,
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

/// `POST /users` — `User`를 생성해요.
async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, String)> {
    let email = Email::try_new(&req.email)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid email: {e}")))?;

    let kind = match req.user_kind.as_str() {
        "individual" => UserKind::Individual,
        "corporation" => UserKind::Corporation,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "user_kind must be 'individual' or 'corporation'".into(),
            ));
        }
    };

    let now = now_utc();
    let user = User::try_new(
        Id::new(),
        &req.zitadel_sub,
        email,
        &req.display_name,
        kind,
        now,
    )
    .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid user: {e}")))?;

    state
        .user_repo
        .save(&user)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("save failed: {e}")))?;

    Ok((StatusCode::CREATED, Json(user.into())))
}

/// `GET /users/:id` — `id`로 `User`를 조회해요.
async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let id = Id::<UserMarker>::try_from_str(&id)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid id: {e}")))?;

    let user = state
        .user_repo
        .find_by_id(&id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("find failed: {e}")))?
        .ok_or((StatusCode::NOT_FOUND, "user not found".into()))?;

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

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool));
    let state = AppState { user_repo };

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = "0.0.0.0:8080";
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
