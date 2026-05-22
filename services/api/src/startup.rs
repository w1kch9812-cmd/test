use std::env;
use std::sync::Arc;

use auth::jwks_cache::JwksCache;
use auth::verifier::{JwtVerifier, Verifier};
use deadpool_redis::{Config as RedisCfg, Runtime as RedisRt};
use parcel_domain::reader::ParcelReader;
use parcel_lookup::{NoOpParcelInfoLookup, ParcelInfoLookup, VWorldParcelInfoLookup};
use raw_capture_client::RawCapture;
use sqlx::postgres::PgPoolOptions;
use vworld_client::{VWorldClient, VWorldConfig, VWorldParcelReader};

use crate::building_reader;
use crate::photo_upload;
use crate::r2_raw_capture;
use crate::raw_capture_metadata;
use crate::routes;

/// SP10 T3: `NoOp` building reader — `DATA_GO_KR_API_KEY` missing fallback.
/// Production blocks this fallback before wiring reaches handlers.
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

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
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

pub fn required_env(name: &'static str) -> Result<String, StartupError> {
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

pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

pub async fn connect_postgres(database_url: &str) -> Result<sqlx::PgPool, StartupError> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
        .map_err(|source| StartupError::PostgresConnect {
            source: Box::new(source),
        })
}

pub fn is_production_env() -> bool {
    std::env::var("APP_ENV").as_deref() == Ok("production")
        || std::env::var("NODE_ENV").as_deref() == Ok("production")
}

pub fn build_raw_capture(
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

pub fn build_parcel_lookup(
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

pub fn build_photo_upload_issuer(
    is_production: bool,
) -> Result<Arc<dyn photo_upload::ListingPhotoUploadUrlIssuer>, StartupError> {
    build_photo_upload_issuer_from_config_result(
        is_production,
        photo_upload::ListingPhotoUploadConfig::from_env(),
    )
}

pub fn build_photo_upload_issuer_from_config_result(
    is_production: bool,
    config_result: Result<
        photo_upload::ListingPhotoUploadConfig,
        photo_upload::ListingPhotoUploadConfigError,
    >,
) -> Result<Arc<dyn photo_upload::ListingPhotoUploadUrlIssuer>, StartupError> {
    match config_result {
        Ok(config) => Ok(Arc::new(photo_upload::R2ListingPhotoUploadUrlIssuer::new(
            config,
        ))),
        Err(error) if is_production => Err(production_config_error(format!(
            "listing photo upload R2 config missing — production cannot issue mock upload URLs: {error}"
        ))),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "listing photo upload R2 config missing — upload URL issuing disabled in dev"
            );
            Ok(Arc::new(photo_upload::DisabledListingPhotoUploadUrlIssuer))
        }
    }
}

pub fn build_photo_object_verifier(
    is_production: bool,
) -> Result<Arc<dyn photo_upload::ListingPhotoObjectVerifier>, StartupError> {
    build_photo_object_verifier_from_config_result(
        is_production,
        photo_upload::ListingPhotoUploadConfig::from_env(),
    )
}

pub fn build_photo_object_verifier_from_config_result(
    is_production: bool,
    config_result: Result<
        photo_upload::ListingPhotoUploadConfig,
        photo_upload::ListingPhotoUploadConfigError,
    >,
) -> Result<Arc<dyn photo_upload::ListingPhotoObjectVerifier>, StartupError> {
    match config_result {
        Ok(config) => Ok(Arc::new(photo_upload::R2ListingPhotoObjectVerifier::new(
            config,
        ))),
        Err(error) if is_production => Err(production_config_error(format!(
            "listing photo object verifier R2 config missing: {error}"
        ))),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "listing photo object verifier R2 config missing - upload confirmation disabled in dev"
            );
            Ok(Arc::new(photo_upload::DisabledListingPhotoObjectVerifier))
        }
    }
}

pub fn build_verifier(dev_mode: bool, is_production: bool) -> Result<Arc<Verifier>, StartupError> {
    if dev_mode {
        if is_production {
            return Err(production_config_error(
                "AUTH_DEV_MODE=true is forbidden in production because it enables mock DEV.<sub> tokens",
            ));
        }
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

pub fn build_redis_pool_shared(
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

pub fn build_internal_auth_secret(is_production: bool) -> Result<Arc<str>, StartupError> {
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

pub fn build_building_reader(
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

#[cfg(test)]
mod tests {
    use auth::verifier::Verifier;

    use crate::photo_upload::ListingPhotoUploadConfigError;

    use super::{
        build_photo_object_verifier_from_config_result,
        build_photo_upload_issuer_from_config_result, build_verifier, required_env, StartupError,
    };

    #[test]
    fn required_env_returns_typed_error_when_missing() {
        const NAME: &str = "GONGZZANG_TEST_REQUIRED_ENV";
        std::env::remove_var(NAME);

        let result = required_env(NAME);

        assert!(matches!(result, Err(StartupError::MissingEnv { name }) if name == NAME));
    }

    #[test]
    fn production_rejects_auth_dev_mode() {
        let result = build_verifier(true, true);

        assert!(
            matches!(result, Err(StartupError::ProductionConfig { reason }) if reason.contains("AUTH_DEV_MODE"))
        );
    }

    #[test]
    fn non_production_allows_auth_dev_mode() {
        let result = build_verifier(true, false);

        assert!(result.is_ok(), "expected dev verifier");
        if let Ok(verifier) = result {
            assert!(matches!(verifier.as_ref(), Verifier::Dev));
        }
    }

    #[test]
    fn production_rejects_missing_listing_photo_upload_r2_config() {
        let result = build_photo_upload_issuer_from_config_result(
            true,
            Err(ListingPhotoUploadConfigError::MissingEnv(
                "LISTING_PHOTO_R2_BUCKET",
            )),
        );

        assert!(
            matches!(result, Err(StartupError::ProductionConfig { reason })
                if reason.contains("listing photo upload")
                    && reason.contains("LISTING_PHOTO_R2_BUCKET"))
        );
    }

    #[test]
    fn production_rejects_missing_listing_photo_object_verifier_r2_config() {
        let result = build_photo_object_verifier_from_config_result(
            true,
            Err(ListingPhotoUploadConfigError::MissingEnv(
                "LISTING_PHOTO_R2_BUCKET",
            )),
        );

        assert!(
            matches!(result, Err(StartupError::ProductionConfig { reason })
                if reason.contains("listing photo object verifier")
                    && reason.contains("LISTING_PHOTO_R2_BUCKET"))
        );
    }

    #[test]
    fn non_production_allows_disabled_listing_photo_object_verifier() {
        let result = build_photo_object_verifier_from_config_result(
            false,
            Err(ListingPhotoUploadConfigError::MissingEnv(
                "LISTING_PHOTO_R2_BUCKET",
            )),
        );

        assert!(result.is_ok());
    }
}
