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

use std::env;
use std::process::ExitCode;
use std::sync::Arc;

use auth::middleware::{auth_layer, AuthState};
use axum::routing::get;
use axum::{middleware, Router};
use db::listing::PgListingRepository;
use db::listing_photo::PgListingPhotoRepository;
use db::user::PgUserRepository;
use listing_domain::repository::ListingRepository;
use listing_marker_serving::ListingMarkerServingGateway;
use listing_photo_domain::repository::ListingPhotoRepository;
use tower_http::trace::TraceLayer;
use user_domain::repository::UserRepository;

use crate::backend_authorization::{enforce_backend_roles, BackendAuthorizationState};
use crate::backend_rate_limit::{
    enforce_backend_rate_limit, AllowAllBackendRateLimiter, BackendRateLimitState,
    RedisBackendRateLimiter,
};
use crate::startup::{
    build_building_reader, build_internal_auth_secret, build_parcel_lookup,
    build_photo_download_issuer, build_photo_upload_issuer, build_redis_pool_shared,
    build_verifier, connect_postgres, init_tracing, is_production_env, required_env, StartupError,
};

mod http {
    pub mod mutation_ctx;
    pub mod problem;
    pub mod request_id;
}

mod observability;
mod photo_upload;
pub mod platform_core_anchor_import;
mod platform_core_auth;
mod platform_core_parcel_lookup;

mod backend_authorization;
mod backend_rate_limit;
mod building_reader;
mod listing_marker_policy;
mod listing_marker_serving;
mod startup;
mod traffic_auth_policy;

mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod buildings; // SP10 T3
    pub mod health;
    pub mod listing_marker_common;
    pub mod listing_marker_counts;
    pub mod listing_marker_filters;
    pub mod listing_marker_masks;
    pub mod listing_marker_tiles;
    pub mod listings;
    pub mod notifications;
    pub mod parcels; // SP10 T3
    pub mod platform_core_events;
    pub mod users;
}

#[tokio::main]
async fn main() -> ExitCode {
    // SP-Obs T5: Sentry init -- 가장 먼저 (panic hook 등록 우선). DSN 미설정 시 None.
    // _sentry_guard 가 main lifetime 동안 유지 — drop 시 flush.
    let _sentry_guard = observability::init_sentry();
    init_tracing();

    match async_main().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(event = "startup_failed", error = %error, "api startup failed");
            ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)] // env 로딩 + state 조립 + router 7 endpoint — 분해 시 중복.
async fn async_main() -> Result<(), StartupError> {
    let database_url = required_env("DATABASE_URL")?;
    let dev_mode = env::var("AUTH_DEV_MODE").unwrap_or_default() == "true";
    // audit 2026-05-08 round 2: production 모드 SSOT. NoOp wire / Redis 부재 / secret 누락
    // 등 모든 production guard 분기가 본 변수 공유 (중복 재계산 제거).
    let is_production = is_production_env();

    let pool = connect_postgres(&database_url).await?;

    let user_repo: Arc<dyn UserRepository> = Arc::new(PgUserRepository::new(pool.clone()));
    let app_state = routes::users::UsersState {
        user_repo: user_repo.clone(),
    };

    let listing_repo: Arc<dyn ListingRepository> = Arc::new(PgListingRepository::new(pool.clone()));
    let photo_repo: Arc<dyn ListingPhotoRepository> =
        Arc::new(PgListingPhotoRepository::new(pool.clone()));

    let parcel_lookup = build_parcel_lookup(is_production)?;
    let photo_upload_issuer = build_photo_upload_issuer(is_production)?;
    let photo_download_issuer = build_photo_download_issuer(is_production)?;
    let photo_object_verifier = startup::build_photo_object_verifier(is_production)?;

    let listings_state = routes::listings::ListingsState {
        listing_repo,
        photo_repo,
        parcel_lookup,
        photo_upload_issuer,
        photo_download_issuer,
        photo_object_verifier,
    };

    let verifier = build_verifier(dev_mode, is_production)?;
    // SP-Obs T7: Redis pool 을 jti_denylist + health check 양쪽이 공유.
    // REDIS_URL 미설정 → 둘 다 None (개발 환경 fail-open). production 은 fail-fast.
    //
    // audit 2026-05-08 round 2 (Codex 발견): REDIS_URL 미설정 → `jti_denylist = None` →
    // middleware 가 `if let Some(dl)` 로 검사 자체 skip = revoked JTI 통과 (fail-open).
    // `fail_closed_on_denylist_error` 는 *Redis error* 만 잡지 *Redis 부재* 는 못 잡음.
    // 따라서 production 에서는 Redis 자체가 없으면 startup 차단.
    let redis_pool_shared = build_redis_pool_shared(is_production)?;
    let backend_rate_limiter: Arc<dyn backend_rate_limit::BackendRateLimiter> =
        if let Some(redis_pool) = &redis_pool_shared {
            Arc::new(RedisBackendRateLimiter::new(redis_pool.clone()))
        } else {
            Arc::new(AllowAllBackendRateLimiter)
        };
    let backend_rate_limit_state = BackendRateLimitState::new(
        backend_rate_limiter,
        traffic_auth_policy::BACKEND_RATE_POLICIES,
    );
    let backend_authorization_state =
        BackendAuthorizationState::new(traffic_auth_policy::BACKEND_ROLE_POLICIES);

    let jti_denylist: Option<Arc<dyn auth::jti_denylist::JtiDenylist>> =
        redis_pool_shared.as_ref().map(|pool| {
            let dl: Arc<dyn auth::jti_denylist::JtiDenylist> = Arc::new(
                auth::jti_denylist::RedisJtiDenylist::new(pool.as_ref().clone()),
            );
            dl
        });

    let auth_state = AuthState {
        verifier,
        user_repo,
        jti_denylist,
        fail_closed_on_denylist_error: is_production,
    };

    // audit 2026-05-08 fix: /internal/auth/event 가 무인증 → shared secret 헤더 검증 추가.
    // production 에서 INTERNAL_AUTH_SECRET 미설정 시 fail-fast (init 단계 panic 차단).
    let internal_auth_secret = build_internal_auth_secret(is_production)?;
    let auth_event_state = routes::auth_event::AuthEventState {
        pool,
        internal_auth_secret,
    };

    // SP-Obs T7: health check state -- DB pool + (optional) Redis pool 공유.
    let health_state = routes::health::HealthState {
        pool: auth_event_state.pool.clone(),
        redis_pool: redis_pool_shared.clone(),
    };
    let listing_marker_serving = Arc::new(ListingMarkerServingGateway::new(
        listings_state.listing_repo.clone(),
        redis_pool_shared.clone(),
    ));

    // SP-Obs T7: K8s/ECS liveness vs readiness 분리. /healthz = liveness 으로
    // 변경 (이전 SP1 의 stateless `health()` 와 동등 — body shape 만 JSON 으로).
    let public: Router<()> = routes::health::public_router(health_state, !is_production);
    let protected: Router<()> = Router::new()
        .route("/users/me", get(routes::users::me))
        .route("/users/:id", get(routes::users::get_user))
        .with_state(app_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));
    // /listings 라우터 — auth_layer 통과 후 GET 검색/상세 (SP6-ii/iii) +
    // POST/PATCH/transitions/photos (SP6-iv). 모든 mutation 핸들러는 require_role(Broker)
    // + ownership check.
    let listing_marker_tiles_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-tiles/listing/:z/:x/:y_pbf",
            get(routes::listing_marker_tiles::get_listing_marker_tile),
        )
        .with_state(routes::listing_marker_tiles::ListingMarkerTilesState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_counts_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-counts/listing",
            get(routes::listing_marker_counts::get_listing_marker_count),
        )
        .with_state(routes::listing_marker_counts::ListingMarkerCountsState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_filters_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-filters/listing",
            axum::routing::post(routes::listing_marker_filters::post_listing_marker_filter),
        )
        .with_state(routes::listing_marker_filters::ListingMarkerFiltersState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));
    let listing_marker_masks_router: Router<()> = Router::new()
        .route(
            "/map/v1/marker-masks/listing/:z/:x/:y",
            get(routes::listing_marker_masks::get_listing_marker_mask),
        )
        .with_state(routes::listing_marker_masks::ListingMarkerMasksState {
            serving: listing_marker_serving.clone(),
        })
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ));

    let listings_router: Router<()> = Router::new()
        .route(
            "/listings",
            get(routes::listings::get_listings).post(routes::listings::create_listing),
        )
        .route(
            "/listings/:id",
            get(routes::listings::get_listing_detail).patch(routes::listings::patch_listing),
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
            get(routes::listings::get_photo_download_redirect)
                .delete(routes::listings::delete_photo),
        )
        .route(
            "/listings/:listing_id/photos/:photo_id/confirm",
            axum::routing::post(routes::listings::confirm_photo_upload),
        )
        .with_state(listings_state.clone())
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP10 T3: Panel system backing endpoints — pure REST resource. Spec § 7 F1.
    let parcels_state = routes::parcels::ParcelsState {
        parcel_lookup: listings_state.parcel_lookup.clone(),
    };
    let parcels_router: Router<()> = Router::new()
        .route("/api/parcels/:pnu", get(routes::parcels::get_parcel))
        .with_state(parcels_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // Building data is a Platform Core Catalog concern. Gongzzang keeps only
    // the route-facing response shape and consumes the published Platform Core
    // HTTP contract. Dev/CI may fall back to an empty reader; production fails
    // fast when PLATFORM_CORE_API_BASE_URL is missing.
    let building_reader = build_building_reader(is_production)?;
    let buildings_state = routes::buildings::BuildingsState {
        reader: building_reader,
    };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP6-v: 공유 repository 인스턴스 — bookmarks/admin/notifications 가 같이 사용.
    let notification_repo: Arc<dyn notification_domain::repository::NotificationRepository> =
        Arc::new(db::notification::PgNotificationRepository::new(
            auth_event_state.pool.clone(),
        ));
    let listing_repo_shared: Arc<dyn listing_domain::repository::ListingRepository> = Arc::new(
        db::listing::PgListingRepository::new(auth_event_state.pool.clone()),
    );

    // SP6-iii/v: bookmarks 라우터 (auth_layer 통과). 멱등 design.
    // SP6-v: listing_repo + notification_repo 추가 — bookmarker != owner 면 알림 INSERT.
    let bookmark_repo: Arc<dyn bookmark_domain::repository::BookmarkRepository> = Arc::new(
        db::bookmark::PgBookmarkRepository::new(auth_event_state.pool.clone()),
    );
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
        .route("/me/bookmarks", get(routes::bookmarks::list_my_bookmarks))
        .with_state(bookmarks_state)
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

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
        .layer(middleware::from_fn_with_state(
            backend_authorization_state.clone(),
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state.clone(),
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(
            auth_state.clone(),
            auth_layer,
        ));

    // SP6-v: /me/notifications 라우터 (인증 사용자 본인 알림 조회/읽음).
    let notifications_state = routes::notifications::NotificationsState { notification_repo };
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
        .layer(middleware::from_fn_with_state(
            backend_authorization_state,
            enforce_backend_roles,
        ))
        .layer(middleware::from_fn_with_state(
            backend_rate_limit_state,
            enforce_backend_rate_limit,
        ))
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));
    // SECURITY (audit 2026-05-08 fix): /internal/auth/event 는 frontend BFF (apps/web)
    // 의 server-side 호출 전용. handler 내부에서 `X-Internal-Auth` shared secret 헤더
    // *constant-time* 비교 (auth_event::post_auth_event). production 에서 secret 미설정
    // 시 init 단계 fail-fast (위 internal_auth_secret 로딩).
    // 추가 layer (defence in depth, 후속): network ACL 로 ingress 차단 (SP6-iam-infra).
    let platform_core_events_router: Router<()> = Router::new()
        .route(
            "/internal/platform-core/events",
            axum::routing::post(routes::platform_core_events::post_platform_core_event),
        )
        .with_state(routes::platform_core_events::PlatformCoreEventsState {
            pool: auth_event_state.pool.clone(),
            internal_auth_secret: auth_event_state.internal_auth_secret.clone(),
        });

    let internal: Router<()> = Router::new()
        .route(
            "/internal/auth/event",
            axum::routing::post(routes::auth_event::post_auth_event),
        )
        .with_state(auth_event_state);

    let app = public
        .merge(protected)
        .merge(listing_marker_tiles_router)
        .merge(listing_marker_counts_router)
        .merge(listing_marker_filters_router)
        .merge(listing_marker_masks_router)
        .merge(listings_router)
        .merge(parcels_router) // SP10 T3
        .merge(buildings_router) // SP10 T3
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(platform_core_events_router)
        .merge(internal)
        // SP-Obs T2: X-Request-Id 가 outermost — TraceLayer 와 auth_layer 보다 먼저
        // 실행돼 모든 trace 가 같은 request_id 공유. 인증 실패해도 trace ID 부여.
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));

    // env 로 listen 주소 override 가능 — port 충돌 (예: Apache httpd 가 8080 점유) 우회용.
    // production 은 Pulumi / ECS task 가 PORT env 주입 표준, default 는 dev 호환 유지.
    let addr = std::env::var("API_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|source| StartupError::Bind {
            addr: addr.clone(),
            source,
        })?;
    axum::serve(listener, app)
        .await
        .map_err(|source| StartupError::Serve { source })
}
