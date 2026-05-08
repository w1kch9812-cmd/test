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
use parcel_domain::reader::ParcelReader;
use parcel_lookup::{NoOpParcelInfoLookup, ParcelInfoLookup, VWorldParcelInfoLookup};
use serde::Serialize;
use shared_kernel::id::{Id, UserMarker};
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use user_domain::entity::{User, UserKind};
use user_domain::repository::UserRepository;
use vworld_client::{VWorldClient, VWorldConfig, VWorldParcelReader};

mod http {
    pub mod mutation_ctx;
    pub mod problem;
    pub mod request_id;
}

mod observability;

mod building_reader;
mod r2_raw_capture;

mod routes {
    pub mod admin_listings;
    pub mod auth_event;
    pub mod bookmarks;
    pub mod buildings; // SP10 T3
    pub mod health;
    pub mod listings;
    pub mod notifications;
    pub mod parcels; // SP10 T3
}

/// SP10 T3: `NoOp` building reader — `DATA_GO_KR_API_KEY` 미설정 시 fallback (빈 list).
/// production 은 SP4-iii-a 의 live reader 로 swap.
struct NoOpBuildingRegisterReader;

impl routes::buildings::BuildingRegisterReader for NoOpBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a shared_kernel::pnu::Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Vec<routes::buildings::BuildingItem>,
                        routes::buildings::BuildingRegisterError,
                    >,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async { Ok(Vec::new()) })
    }
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

// SP-Obs T7: 본 함수는 routes::health::liveness 가 대체. 본 stub 유지 안 함.

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

/// audit 2026-05-08 — production 환경에서 critical config 미설정 시 fail-fast.
/// `tracing::error!` 로 구조화 emit (Sentry tracing layer 가 자동 capture) 후 즉시 종료.
fn fail_fast_production(reason: &str) -> ! {
    tracing::error!(event = "startup_fail_fast", reason = %reason, "production 차단");
    // process::exit 는 Drop 미실행 — `_sentry_guard` flush 안 됨. 향후 SIGTERM 으로 graceful.
    std::process::exit(1);
}

#[allow(clippy::too_many_lines)] // env 로딩 + state 조립 + router 7 endpoint — 분해 시 중복.
#[tokio::main]
async fn main() {
    // SP-Obs T5: Sentry init -- 가장 먼저 (panic hook 등록 우선). DSN 미설정 시 None.
    // _sentry_guard 가 main lifetime 동안 유지 — drop 시 flush.
    let _sentry_guard = observability::init_sentry();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let dev_mode = env::var("AUTH_DEV_MODE").unwrap_or_default() == "true";
    // audit 2026-05-08 round 2: production 모드 SSOT. NoOp wire / Redis 부재 / secret 누락
    // 등 모든 fail_fast_production 분기가 본 변수 공유 (중복 재계산 제거).
    let is_production = std::env::var("APP_ENV").as_deref() == Ok("production")
        || std::env::var("NODE_ENV").as_deref() == Ok("production");

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

    // SP9 T4 / ADR 0018: PNU lookup. VWORLD_API_KEY 미설정 → NoOp fallback (dev/CI).
    // audit 2026-05-08 fix: production 에서 NoOp fallback 차단 — production 은
    // VWORLD_API_KEY *반드시* (startup fail-fast).
    //
    // ADR 0026: Bronze API archive = R2 (S3-호환 객체 저장소). Postgres jsonb 폐기 —
    // cost (~7-10x) + UPSERT 시계열 손실 + connection pool 부담. R2 키 구조:
    //   `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json`
    // V-World parcel_lookup 과 data.go.kr building_register 둘 다 같은 sink 공유.
    let raw_capture: Arc<dyn raw_capture_client::RawCapture> = match r2_raw_capture::R2RawCaptureConfig::from_env() {
        Ok(cfg) => {
            // audit 2026-05-08 round 3 (Codex): production 에서 BRONZE_FALLBACK_DIR
            // 미설정 = R2 PUT 실패 시 raw 영구 손실 차단 path 0. production 은 *반드시*.
            if is_production && cfg.fallback_dir.is_none() {
                fail_fast_production(
                    "BRONZE_FALLBACK_DIR 미설정 — R2 PUT 실패 시 raw 영구 손실. \
                     production 은 ADR 0026 의 디스크 fallback 필수 \
                     (예: /var/lib/gongzzang/bronze-fallback)",
                );
            }
            // audit 2026-05-08 round 4 (Codex): env 존재만 검사하면 잘못된 경로 / 권한
            // / 디스크 풀 케이스 잡지 못함. mkdir + write probe 로 *진짜* writable 확정.
            if let Err(e) = cfg.ensure_fallback_writable() {
                if is_production {
                    fail_fast_production(&format!(
                        "BRONZE_FALLBACK_DIR ({:?}) mkdir/write probe 실패 — production 차단: {e}",
                        cfg.fallback_dir
                    ));
                }
                tracing::warn!(
                    error = %e,
                    fallback_dir = ?cfg.fallback_dir,
                    "BRONZE_FALLBACK_DIR not writable (dev only — production 은 fail-fast)"
                );
            }
            tracing::info!(
                "raw_capture: R2 live (bucket={}, prefix={}, fallback={:?}) — ADR 0026",
                cfg.bucket,
                cfg.bronze_prefix,
                cfg.fallback_dir,
            );
            Arc::new(r2_raw_capture::R2RawCapture::new(cfg))
        }
        Err(e) => {
            if is_production {
                fail_fast_production(&format!(
                    "R2 env (R2_ACCOUNT_ID/ACCESS_KEY/SECRET_KEY/BUCKET) 미설정 — Bronze raw_response 보존 path 0 (ADR 0026): {e}"
                ));
            }
            tracing::warn!(
                error = %e,
                "raw_capture: R2 env missing → NoOp (dev only; production 은 fail-fast)"
            );
            Arc::new(raw_capture_client::NoOpRawCapture::new())
        }
    };

    let parcel_lookup: Arc<dyn ParcelInfoLookup> = match VWorldConfig::from_env() {
        Ok(cfg) => {
            tracing::info!("parcel_lookup: V-World live (LP_PA_CBND_BUBUN) + PgRawCapture");
            let client = Arc::new(VWorldClient::new(cfg));
            // audit 2026-05-08 round 2 (P1 — Codex 발견): NoOpRawCapture → PgRawCapture.
            // raw_response JSONB 영구 저장 (parcel_external_data, source='vworld').
            // CHECK (source in ('vworld', ...)) 정합 검증 — migrations/30006_parcel_external_data.sql:13-19.
            let reader: Arc<dyn ParcelReader> = Arc::new(VWorldParcelReader::new(
                client,
                Arc::clone(&raw_capture),
            ));
            Arc::new(VWorldParcelInfoLookup::new(reader))
        }
        Err(e) => {
            if is_production {
                fail_fast_production(&format!(
                    "parcel_lookup VWORLD env 미설정 (audit 2026-05-08): {e}"
                ));
            }
            tracing::warn!(
                error = %e,
                "parcel_lookup: VWORLD env missing → NoOp fallback (dev only; production 은 fail-fast)"
            );
            Arc::new(NoOpParcelInfoLookup::new())
        }
    };

    let listings_state = routes::listings::ListingsState {
        listing_repo,
        photo_repo,
        parcel_lookup,
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
    // SP-Obs T7: Redis pool 을 jti_denylist + health check 양쪽이 공유.
    // REDIS_URL 미설정 → 둘 다 None (개발 환경 fail-open). production 은 fail-fast.
    //
    // audit 2026-05-08 round 2 (Codex 발견): REDIS_URL 미설정 → `jti_denylist = None` →
    // middleware 가 `if let Some(dl)` 로 검사 자체 skip = revoked JTI 통과 (fail-open).
    // `fail_closed_on_denylist_error` 는 *Redis error* 만 잡지 *Redis 부재* 는 못 잡음.
    // 따라서 production 에서는 Redis 자체가 없으면 startup 차단.
    let redis_pool_shared: Option<Arc<deadpool_redis::Pool>> =
        env::var("REDIS_URL").ok().map(|url| {
            let pool = RedisCfg::from_url(url)
                .create_pool(Some(RedisRt::Tokio1))
                .expect("redis pool");
            Arc::new(pool)
        });
    if redis_pool_shared.is_none() {
        if is_production {
            fail_fast_production(
                "REDIS_URL 미설정 — JTI denylist None = revoked token 통과 (fail-open). production 은 Redis 필수.",
            );
        }
        tracing::warn!(
            "REDIS_URL not set — JTI denylist + readiness Redis check disabled (dev fail-open). production 은 fail-fast 차단됨."
        );
    }

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
    let internal_auth_secret = match std::env::var("INTERNAL_AUTH_SECRET") {
        Ok(s) if !s.trim().is_empty() => Arc::<str>::from(s),
        _ => {
            if is_production {
                fail_fast_production(
                    "INTERNAL_AUTH_SECRET 미설정 — /internal/auth/event 무인증 차단 위해 필수",
                );
            }
            // dev 안전 fallback — 시작 시 1회 random 발급. Next.js 측이 같은 값 사용 못
            // 하면 401 리턴 → audit 못 박힘 (서비스 자체 OK). dev 검증 시 환경변수 명시.
            tracing::warn!(
                "INTERNAL_AUTH_SECRET 미설정 (dev) — random fallback 사용. Next.js \
                 측 동일 값 설정 필요 (apps/web/.env.local)."
            );
            Arc::<str>::from(format!("dev-random-{}", ulid::Ulid::new()))
        }
    };
    let auth_event_state = routes::auth_event::AuthEventState {
        pool,
        internal_auth_secret,
    };

    // SP-Obs T7: health check state -- DB pool + (optional) Redis pool 공유.
    let health_state = routes::health::HealthState {
        pool: auth_event_state.pool.clone(),
        redis_pool: redis_pool_shared,
    };

    // SP-Obs T7: K8s/ECS liveness vs readiness 분리. /healthz = liveness 으로
    // 변경 (이전 SP1 의 stateless `health()` 와 동등 — body shape 만 JSON 으로).
    let public: Router<()> = Router::new()
        .route("/healthz", get(routes::health::liveness))
        .route("/healthz/ready", get(routes::health::readiness))
        .route("/healthz/db", get(routes::health::db_health))
        .with_state(health_state);
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
            axum::routing::delete(routes::listings::delete_photo),
        )
        .with_state(listings_state.clone())
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
            auth_state.clone(),
            auth_layer,
        ));

    // SP10 T3 + audit 2026-05-08 round 2 (P2): building_register reader 라이브 wire.
    //
    // env (`DATA_GO_KR_API_KEY` 또는 `ODP_SERVICE_KEY`) 있으면 `DataGoKrBuildingRegisterReader`
    // (data.go.kr `getBrTitleInfo` 라이브 호출). 없으면 dev fallback NoOp + production fail-fast.
    //
    // 본 reader 는 *panel 응답용 좁은 subset* (BuildingItem) 만 채움. rich Building entity
    // (V-World 폴리곤 합성) 는 FU 40 의 R2 PMTiles 에서 별도 도입.
    let building_reader: Arc<dyn routes::buildings::BuildingRegisterReader> = {
        let service_key = std::env::var("DATA_GO_KR_API_KEY")
            .or_else(|_| std::env::var("ODP_SERVICE_KEY"))
            .ok()
            .filter(|v| !v.trim().is_empty());
        service_key.map_or_else(
            || {
                if is_production {
                    fail_fast_production(
                        "DATA_GO_KR_API_KEY (또는 ODP_SERVICE_KEY) 미설정 — building_register NoOp \
                         가 사용자한테 silent empty list 반환 (audit 2026-05-08)",
                    );
                }
                tracing::warn!(
                    "building_register: DATA_GO_KR_API_KEY missing → NoOp empty list (dev only)"
                );
                Arc::new(NoOpBuildingRegisterReader) as Arc<dyn routes::buildings::BuildingRegisterReader>
            },
            |key| {
                tracing::info!(
                    "building_register: data.go.kr live (getBrTitleInfo via DataGoKrBuildingRegisterReader)"
                );
                let base_url = std::env::var("ODP_BASE_URL")
                    .unwrap_or_else(|_| "https://apis.data.go.kr".to_owned());
                let client = Arc::new(data_go_kr_client::DataGoKrClient::new(
                    data_go_kr_client::DataGoKrConfig {
                        service_key: key,
                        base_url,
                    },
                ));
                // audit 2026-05-08 round 2 (P2 ship-safety fix): raw_capture 공유 →
                // parcel_external_data (pnu, 'data_go_kr_building') UPSERT.
                Arc::new(building_reader::DataGoKrBuildingRegisterReader::new(
                    client,
                    Arc::clone(&raw_capture),
                )) as Arc<dyn routes::buildings::BuildingRegisterReader>
            },
        )
    };
    let buildings_state = routes::buildings::BuildingsState {
        reader: building_reader,
    };
    let buildings_router: Router<()> = Router::new()
        .route("/api/buildings", get(routes::buildings::list_buildings))
        .with_state(buildings_state)
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
        .layer(middleware::from_fn_with_state(auth_state, auth_layer));
    // SECURITY (audit 2026-05-08 fix): /internal/auth/event 는 frontend BFF (apps/web)
    // 의 server-side 호출 전용. handler 내부에서 `X-Internal-Auth` shared secret 헤더
    // *constant-time* 비교 (auth_event::post_auth_event). production 에서 secret 미설정
    // 시 init 단계 fail-fast (위 internal_auth_secret 로딩).
    // 추가 layer (defence in depth, 후속): network ACL 로 ingress 차단 (SP6-iam-infra).
    let internal: Router<()> = Router::new()
        .route(
            "/internal/auth/event",
            axum::routing::post(routes::auth_event::post_auth_event),
        )
        .with_state(auth_event_state);

    let app = public
        .merge(protected)
        .merge(listings_router)
        .merge(parcels_router) // SP10 T3
        .merge(buildings_router) // SP10 T3
        .merge(bookmarks_router)
        .merge(admin_router)
        .merge(notifications_router)
        .merge(internal)
        // SP-Obs T2: X-Request-Id 가 outermost — TraceLayer 와 auth_layer 보다 먼저
        // 실행돼 모든 trace 가 같은 request_id 공유. 인증 실패해도 trace ID 부여.
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(http::request_id::request_id_layer));

    // env 로 listen 주소 override 가능 — port 충돌 (예: Apache httpd 가 8080 점유) 우회용.
    // production 은 Pulumi / ECS task 가 PORT env 주입 표준, default 는 dev 호환 유지.
    let addr = std::env::var("API_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_owned());
    tracing::info!("api listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
