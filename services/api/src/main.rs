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
// FU 26 — JWKS reqwest::Client 초기화는 legitimate (auth crate 가 wrapper).
#![allow(clippy::disallowed_types)]

use std::env;
use std::process::ExitCode;
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
use raw_capture_client::RawCapture;
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
mod raw_capture_metadata;

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
                        Vec<building_domain::entity::Building>,
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

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error("{name} must be set")]
    MissingEnv { name: &'static str },
    #[error("{name} must not be empty")]
    EmptyEnv { name: &'static str },
    #[error("connect to Postgres: {source}")]
    PostgresConnect {
        #[source]
        source: Box<sqlx::Error>,
    },
    #[error("build JWKS HTTP client: {source}")]
    JwksHttpClient {
        #[source]
        source: Box<reqwest::Error>,
    },
    #[error("create Redis pool: {detail}")]
    RedisPool { detail: String },
    #[error("bind API listener {addr}: {source}")]
    Bind {
        addr: String,
        #[source]
        source: std::io::Error,
    },
    #[error("serve API: {source}")]
    Serve {
        #[source]
        source: std::io::Error,
    },
    #[error("production startup config invalid: {reason}")]
    ProductionConfig { reason: String },
}

fn required_env(name: &'static str) -> Result<String, StartupError> {
    let value = env::var(name).map_err(|_| StartupError::MissingEnv { name })?;
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(StartupError::EmptyEnv { name });
    }
    Ok(value)
}

fn optional_env(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn production_config_error(reason: impl Into<String>) -> StartupError {
    StartupError::ProductionConfig {
        reason: reason.into(),
    }
}

fn create_redis_pool(url: String) -> Result<Arc<deadpool_redis::Pool>, StartupError> {
    let pool = RedisCfg::from_url(url)
        .create_pool(Some(RedisRt::Tokio1))
        .map_err(|source| StartupError::RedisPool {
            detail: source.to_string(),
        })?;
    Ok(Arc::new(pool))
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

async fn connect_postgres(database_url: &str) -> Result<sqlx::PgPool, StartupError> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .map_err(|source| StartupError::PostgresConnect {
            source: Box::new(source),
        })
}

fn is_production_env() -> bool {
    std::env::var("APP_ENV").as_deref() == Ok("production")
        || std::env::var("NODE_ENV").as_deref() == Ok("production")
}

fn build_raw_capture(
    is_production: bool,
    pool: sqlx::PgPool,
) -> Result<Arc<dyn RawCapture>, StartupError> {
    let inner = match r2_raw_capture::R2RawCaptureConfig::from_env() {
        Ok(cfg) => build_r2_raw_capture(cfg, is_production)?,
        Err(error) => build_noop_raw_capture(&error, is_production)?,
    };
    Ok(Arc::new(raw_capture_metadata::TrackedRawCapture::new(
        inner, pool,
    )))
}

fn build_r2_raw_capture(
    cfg: r2_raw_capture::R2RawCaptureConfig,
    is_production: bool,
) -> Result<Arc<dyn RawCapture>, StartupError> {
    if is_production && cfg.fallback_dir.is_none() {
        return Err(production_config_error(
            "BRONZE_FALLBACK_DIR 미설정 — R2 PUT 실패 시 raw 영구 손실. \
             production 은 ADR 0026 의 디스크 fallback 필수 \
             (예: /var/lib/gongzzang/bronze-fallback)",
        ));
    }
    if let Err(error) = cfg.ensure_fallback_writable() {
        handle_unwritable_raw_capture_fallback(&cfg, is_production, &error)?;
    }
    tracing::info!(
        "raw_capture: R2 live (bucket={}, prefix={}, fallback={:?}) — ADR 0026",
        cfg.bucket,
        cfg.bronze_prefix,
        cfg.fallback_dir,
    );
    Ok(Arc::new(r2_raw_capture::R2RawCapture::new(cfg)))
}

fn handle_unwritable_raw_capture_fallback(
    cfg: &r2_raw_capture::R2RawCaptureConfig,
    is_production: bool,
    error: &std::io::Error,
) -> Result<(), StartupError> {
    if is_production {
        return Err(production_config_error(format!(
            "BRONZE_FALLBACK_DIR ({:?}) mkdir/write probe 실패 — production 차단: {error}",
            cfg.fallback_dir
        )));
    }
    tracing::warn!(
        error = %error,
        fallback_dir = ?cfg.fallback_dir,
        "BRONZE_FALLBACK_DIR not writable (dev only — production 은 fail-fast)"
    );
    Ok(())
}

fn build_noop_raw_capture(
    error: &r2_raw_capture::R2ConfigError,
    is_production: bool,
) -> Result<Arc<dyn RawCapture>, StartupError> {
    if is_production {
        return Err(production_config_error(format!(
            "R2 env (R2_ACCOUNT_ID/ACCESS_KEY/SECRET_KEY/BUCKET) 미설정 — Bronze raw_response 보존 path 0 (ADR 0026): {error}"
        )));
    }
    tracing::warn!(
        error = %error,
        "raw_capture: R2 env missing → NoOp (dev only; production 은 fail-fast)"
    );
    Ok(Arc::new(raw_capture_client::NoOpRawCapture::new()))
}

fn build_parcel_lookup(
    is_production: bool,
    raw_capture: &Arc<dyn RawCapture>,
) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    match VWorldConfig::from_env() {
        Ok(cfg) => Ok(build_vworld_parcel_lookup(cfg, raw_capture)),
        Err(error) => build_noop_parcel_lookup(&error, is_production),
    }
}

fn build_vworld_parcel_lookup(
    cfg: VWorldConfig,
    raw_capture: &Arc<dyn RawCapture>,
) -> Arc<dyn ParcelInfoLookup> {
    tracing::info!("parcel_lookup: V-World live (LP_PA_CBND_BUBUN) + PgRawCapture");
    let client = Arc::new(VWorldClient::new(cfg));
    let reader: Arc<dyn ParcelReader> =
        Arc::new(VWorldParcelReader::new(client, Arc::clone(raw_capture)));
    Arc::new(VWorldParcelInfoLookup::new(reader))
}

fn build_noop_parcel_lookup(
    error: &vworld_client::ConfigError,
    is_production: bool,
) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    if is_production {
        return Err(production_config_error(format!(
            "parcel_lookup VWORLD env 미설정 (audit 2026-05-08): {error}"
        )));
    }
    tracing::warn!(
        error = %error,
        "parcel_lookup: VWORLD env missing → NoOp fallback (dev only; production 은 fail-fast)"
    );
    Ok(Arc::new(NoOpParcelInfoLookup::new()))
}

fn build_verifier(dev_mode: bool) -> Result<Arc<Verifier>, StartupError> {
    if dev_mode {
        tracing::warn!(
            "AUTH_DEV_MODE=true — using mock verifier (DEV.<sub> tokens). Production must NOT set this."
        );
        return Ok(Arc::new(Verifier::Dev));
    }
    let issuer = required_env("ZITADEL_ISSUER")?;
    let audience = required_env("ZITADEL_AUDIENCE")?;
    let jwks_url = format!("{issuer}/oauth/v2/keys");
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|source| StartupError::JwksHttpClient {
            source: Box::new(source),
        })?;
    let jwks = Arc::new(JwksCache::new(jwks_url, http));
    Ok(Arc::new(Verifier::Real(JwtVerifier::new(
        issuer, audience, jwks,
    ))))
}

fn build_redis_pool_shared(
    is_production: bool,
) -> Result<Option<Arc<deadpool_redis::Pool>>, StartupError> {
    let pool = optional_env("REDIS_URL")
        .map(create_redis_pool)
        .transpose()?;
    if pool.is_none() {
        if is_production {
            return Err(production_config_error(
                "REDIS_URL 미설정 — JTI denylist None = revoked token 통과 (fail-open). production 은 Redis 필수.",
            ));
        }
        tracing::warn!(
            "REDIS_URL not set — JTI denylist + readiness Redis check disabled (dev fail-open). production 은 fail-fast 차단됨."
        );
    }
    Ok(pool)
}

fn build_internal_auth_secret(is_production: bool) -> Result<Arc<str>, StartupError> {
    if let Some(secret) = optional_env("INTERNAL_AUTH_SECRET") {
        return Ok(Arc::<str>::from(secret));
    }
    if is_production {
        return Err(production_config_error(
            "INTERNAL_AUTH_SECRET 미설정 — /internal/auth/event 무인증 차단 위해 필수",
        ));
    }
    tracing::warn!(
        "INTERNAL_AUTH_SECRET 미설정 (dev) — random fallback 사용. Next.js \
         측 동일 값 설정 필요 (apps/web/.env.local)."
    );
    Ok(Arc::<str>::from(format!(
        "dev-random-{}",
        ulid::Ulid::new()
    )))
}

fn build_building_reader(
    is_production: bool,
    raw_capture: &Arc<dyn RawCapture>,
) -> Result<Arc<dyn routes::buildings::BuildingRegisterReader>, StartupError> {
    if let Some(key) =
        optional_env("DATA_GO_KR_API_KEY").or_else(|| optional_env("ODP_SERVICE_KEY"))
    {
        return Ok(build_data_go_kr_building_reader(key, raw_capture));
    }
    if is_production {
        return Err(production_config_error(
            "DATA_GO_KR_API_KEY (또는 ODP_SERVICE_KEY) 미설정 — building_register NoOp \
             가 사용자한테 silent empty list 반환 (audit 2026-05-08)",
        ));
    }
    tracing::warn!("building_register: DATA_GO_KR_API_KEY missing → NoOp empty list (dev only)");
    Ok(Arc::new(NoOpBuildingRegisterReader))
}

fn build_data_go_kr_building_reader(
    service_key: String,
    raw_capture: &Arc<dyn RawCapture>,
) -> Arc<dyn routes::buildings::BuildingRegisterReader> {
    tracing::info!(
        "building_register: data.go.kr live (getBrTitleInfo via DataGoKrBuildingRegisterReader)"
    );
    let base_url =
        std::env::var("ODP_BASE_URL").unwrap_or_else(|_| "https://apis.data.go.kr".to_owned());
    let client = Arc::new(data_go_kr_client::DataGoKrClient::new(
        data_go_kr_client::DataGoKrConfig {
            service_key,
            base_url,
        },
    ));
    Arc::new(building_reader::DataGoKrBuildingRegisterReader::new(
        client,
        Arc::clone(raw_capture),
    ))
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
    let raw_capture = build_raw_capture(is_production, pool.clone())?;

    let parcel_lookup = build_parcel_lookup(is_production, &raw_capture)?;

    let listings_state = routes::listings::ListingsState {
        listing_repo,
        photo_repo,
        parcel_lookup,
    };

    let verifier = build_verifier(dev_mode)?;
    // SP-Obs T7: Redis pool 을 jti_denylist + health check 양쪽이 공유.
    // REDIS_URL 미설정 → 둘 다 None (개발 환경 fail-open). production 은 fail-fast.
    //
    // audit 2026-05-08 round 2 (Codex 발견): REDIS_URL 미설정 → `jti_denylist = None` →
    // middleware 가 `if let Some(dl)` 로 검사 자체 skip = revoked JTI 통과 (fail-open).
    // `fail_closed_on_denylist_error` 는 *Redis error* 만 잡지 *Redis 부재* 는 못 잡음.
    // 따라서 production 에서는 Redis 자체가 없으면 startup 차단.
    let redis_pool_shared = build_redis_pool_shared(is_production)?;

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
    let building_reader = build_building_reader(is_production, &raw_capture)?;
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

#[cfg(test)]
mod startup_tests {
    use super::{required_env, StartupError};

    #[test]
    fn required_env_returns_typed_error_when_missing() {
        const NAME: &str = "GONGZZANG_TEST_REQUIRED_ENV";
        std::env::remove_var(NAME);

        let result = required_env(NAME);

        assert!(matches!(result, Err(StartupError::MissingEnv { name }) if name == NAME));
    }
}
