//! 공짱 `HTTP` `API` service — `Walking Skeleton` + `Auth` (`SP3`) + `SP6-i` + `SP6-ii`.
//!
//! 라우트:
//! - `GET /healthz` — public liveness probe
//! - `GET /users/me` — 인증된 자신 조회 (`AuthenticatedUser` extractor)
//! - `GET /users/:id` — 인증된 자신만 (`auth.user.id == path id`), 다른 id 는 `403`
//! - `GET /listings` — 카드 list 검색 (인증 필수, SP6-ii)
//! - `POST /internal/auth/event` — frontend `AuthEvent` 수신 → `audit_log` INSERT
//!
//! `POST /users` 는 제거 — first-sign-in 자동 생성으로 대체.

#![forbid(unsafe_code)]
// `main.rs`: init failure panic은 정답이라 expect/unwrap 허용해요.
#![allow(clippy::expect_used, clippy::unwrap_used)]
// FU 26 — JWKS reqwest::Client 초기화는 legitimate (auth crate 가 wrapper).
#![allow(clippy::disallowed_types)]

use std::env;
use std::sync::Arc;

use auth::jwks_cache::JwksCache;
use auth::middleware::{auth_layer, AuthState, AuthenticatedUser};
use auth::verifier::{JwtVerifier, Verifier};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{middleware, Json, Router};
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use deadpool_redis::{Config as RedisCfg, Runtime as RedisRt};
use listing_domain::repository::ListingRepository;
use listing_photo_domain::repository::ListingPhotoRepository;
use serde::Serialize;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;

mod http {
    pub mod mutation_ctx;
    pub mod problem;
    pub mod request_id;
}

mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod listings;
    pub mod notifications;
}

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

#[allow(clippy::too_many_lines)] // env 로딩 + state 조립 + router 7 endpoint — 분해 시 중복.
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let dev_mode = env::var("AUTH_DEV_MODE").unwrap_or_default() == "true";

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
    let app_state = AppState {
        user_repo: user_repo.clone(),
    };

    let listing_repo: Arc<dyn ListingRepository> = Arc::new(PgListingRepository::new(pool.clone()));
    let photo_repo: Arc<dyn ListingPhotoRepository> =
        Arc::new(PgListingPhotoRepository::new(pool.clone()));
    let listings_state = routes::listings::ListingsState {
        listing_repo,
        photo_repo,
    };

    let verifier = if dev_mode {
        tracing::warn!(
            "AUTH_DEV_MODE=true — using mock verifier (DEV.<sub> tokens). Production must NOT set this."
        );
        Arc::new(Verifier::Dev)
    } else {
        let issuer = env::var("ZITADEL_ISSUER").expect("ZITADEL_ISSUER must be set");
        let audience = env::var("ZITADEL_AUDIENCE").expect("ZITADEL_AUDIENCE must be set");
        let jwks_url = format!("{issuer}/oauth/v2/keys");
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("reqwest");
        let jwks = Arc::new(JwksCache::new(jwks_url, http));
        Arc::new(Verifier::Real(JwtVerifier::new(issuer, audience, jwks)))
    };
    let jti_denylist: Option<Arc<dyn auth::jti_denylist::JtiDenylist>> =
        env::var("REDIS_URL").map_or_else(
            |_| {
                tracing::warn!(
                    "REDIS_URL not set — JTI denylist disabled (fail-open). Set REDIS_URL in production."
                );
                None
            },
            |url| {
                let pool = RedisCfg::from_url(url)
                    .create_pool(Some(RedisRt::Tokio1))
                    .expect("redis pool");
                let dl: Arc<dyn auth::jti_denylist::JtiDenylist> =
                    Arc::new(auth::jti_denylist::RedisJtiDenylist::new(pool));
                Some(dl)
            },
        );

    let auth_state = AuthState {
        verifier,
        user_repo,
        jti_denylist,
    };

    let auth_event_state = routes::auth_event::AuthEventState { pool };
    let public: Router<()> = Router::new().route("/healthz", get(health));
    let protected: Router<()> = Router::new()
        .route("/users/me", get(me))
        .route("/users/:id", get(get_user))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));
    // /listings 라우터 — auth_layer 통과 후 GET 검색/상세 (SP6-ii/iii) +
    // POST/PATCH/transitions/photos (SP6-iv). 모든 mutation 핸들러는 require_role(Broker)
    // + ownership check.
    let listings_router: Router<()> = Router::new()
        .route(
            "/listings",
            get(routes::listings::get_listings).post(routes::listings::create_listing),
        )
        .route(
            "/listings/:id",
            get(routes::listings::get_listing_detail)
                .patch(routes::listings::patch_listing),
        )
        .route(
            "/listings/:id/submit-for-review",
            axum::routing::post(routes::listings::submit_for_review),
        )
        .route(
            "/listings/:id/revise",
            axum::routing::post(routes::listings::revise),
        )
        .route(
            "/listings/:id/photos",
            axum::routing::post(routes::listings::request_photo_upload),
        )
        .route(
            "/listings/:listing_id/photos/:photo_id",
            axum::routing::delete(routes::listings::delete_photo),
        )
        .with_state(listings_state)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_layer));

    // SP6-v: 공유 repository 인스턴스 — bookmarks/admin/notifications 가 같이 사용.
    let notification_repo: Arc<dyn notification_domain::repository::NotificationRepository> =
        Arc::new(db::notification::PgNotificationRepository::new(
            auth_event_state.pool.clone(),
        ));
    let listing_repo_shared: Arc<dyn listing_domain::repository::ListingRepository> =
        Arc::new(db::listing::PgListingRepository::new(
            auth_event_state.pool.clone(),
        ));

    // SP6-iii/v: bookmarks 라우터 (auth_layer 통과). 멱등 design.
    // SP6-v: listing_repo + notification_repo 추가 — bookmarker != owner 면 알림 INSERT.
    let bookmark_repo: Arc<dyn bookmark_domain::repository::BookmarkRepository> =
        Arc::new(db::bookmark::PgBookmarkRepository::new(
            auth_event_state.pool.clone(),
        ));
    let bookmarks_state = routes::bookmarks::BookmarksState {
        bookmark_repo,
        listing_repo: listing_repo_shared.clone(),
        notification_repo: notification_repo.clone(),
    };
    let bookmarks_router: Router<()> = Router::new()
        .route(
            "/listings/:id/bookmark",
            axum::routing::post(routes::bookmarks::toggle_bookmark)
                .delete(routes::bookmarks::delete_bookmark),
        )
        .route(
            "/me/bookmarks",
            get(routes::bookmarks::list_my_bookmarks),
        )
        .with_state(bookmarks_state)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_layer));

    // SP6-v: admin_listings 라우터 — Admin/Operator 매물 승인/반려 + 알림 trigger.
    let admin_listings_state = routes::admin_listings::AdminListingsState {
        listing_repo: listing_repo_shared,
        notification_repo: notification_repo.clone(),
    };
    let admin_router: Router<()> = Router::new()
        .route(
            "/admin/listings/:id/approve",
            axum::routing::post(routes::admin_listings::approve_listing),
        )
        .route(
            "/admin/listings/:id/reject",
            axum::routing::post(routes::admin_listings::reject_listing),
        )
        .with_state(admin_listings_state)
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth_layer));

    // SP6-v: /me/notifications 라우터 (인증 사용자 본인 알림 조회/읽음).
    let notifications_state = routes::notifications::NotificationsState {
        notification_repo,
    };
    let notifications_router: Router<()> = Router::new()
        .route(
            "/me/notifications",
            get(routes::notifications::list_notifications),
        )
        .route(
            "/me/notifications/unread-count",
            get(routes::notifications::unread_count),
        )
        .route(
            "/me/notifications/:id/read",
            axum::routing::patch(routes::notifications::mark_read),
        )
        .route(
            "/me/notifications/mark-all-read",
            axum::routing::post(routes::notifications::mark_all_read),
        )
        .with_state(notifications_state)
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));
    // SECURITY: /internal/auth/event 는 현재 unauthenticated.
    // frontend (apps/web/app/api/auth/*) 가 server-side 호출 가정 — production 배포 전 반드시
    // network-level 보호 필요 (SP6-iam-infra 가 ingress ACL / VPC 격리 / shared secret).
    // 외부 노출 시 audit_log 오염 가능 (사용자가 임의 AuthEvent inject).
    let internal: Router<()> = Router::new()
        .route(
            "/internal/auth/event",
            axum::routing::post(routes::auth_event::post_auth_event),
        )
        .with_state(auth_event_state);

    let app = public
        .merge(protected)
        .merge(listings_router)
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(internal)
        // SP-Obs T2: X-Request-Id 가 outermost — TraceLayer 와 auth_layer 보다 먼저
        // 실행돼 모든 trace 가 같은 request_id 공유. 인증 실패해도 trace ID 부여.
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));

    let addr = "0.0.0.0:8080";
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
