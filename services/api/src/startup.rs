use std::env;
use std::sync::Arc;

use auth::jwks_cache::JwksCache;
use auth::verifier::{JwtVerifier, Verifier};
use deadpool_redis::{Config as RedisCfg, Runtime as RedisRt};
use parcel_lookup::{NoOpParcelInfoLookup, ParcelInfoLookup};
use sqlx::postgres::PgPoolOptions;

use crate::building_reader;
use crate::photo_upload;
use crate::platform_core_parcel_lookup;
use crate::routes;
use auth::platform_core_service::{PlatformCoreServiceAuth, PlatformCoreServiceAuthMetadataConfig};

/// Dev-only building reader fallback.
/// Production requires Platform Core Catalog because canonical building data is
/// no longer owned by the Gongzzang API service.
struct NoOpBuildingRegisterReader;

impl routes::buildings::BuildingRegisterReader for NoOpBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        _pnu: &'a shared_kernel::pnu::Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<
                        Vec<routes::buildings::BuildingRegisterRecord>,
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

pub fn build_parcel_lookup(is_production: bool) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
        is_production,
        optional_env("PLATFORM_CORE_API_BASE_URL"),
        optional_env("PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE"),
        optional_env("PLATFORM_CORE_SERVICE_TOKEN"),
        platform_core_service_auth_metadata_from_env(),
    )
}

#[cfg(test)]
fn build_parcel_lookup_from_platform_core_base_url(
    is_production: bool,
    base_url: Option<String>,
    service_token: Option<String>,
) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
        is_production,
        base_url,
        None,
        service_token,
        PlatformCoreServiceAuthMetadataConfig::default(),
    )
}

fn build_parcel_lookup_from_platform_core_base_url_with_service_auth_metadata(
    is_production: bool,
    base_url: Option<String>,
    workload_identity_token_file: Option<String>,
    service_token: Option<String>,
    service_auth_metadata: PlatformCoreServiceAuthMetadataConfig,
) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    if let Some(base_url) = base_url {
        let auth = build_platform_core_service_auth(
            is_production,
            workload_identity_token_file,
            service_token,
            service_auth_metadata,
        )?;
        tracing::info!(
            "parcel_lookup: Platform Core Catalog live (/catalog/v1/parcels/by-pnu/:pnu)"
        );
        return platform_core_parcel_lookup::build_platform_core_parcel_info_lookup(
            &base_url, auth,
        )
        .map_err(|error| {
            production_config_error(format!(
                "PLATFORM_CORE_API_BASE_URL invalid for parcel_lookup: {error}"
            ))
        });
    }
    build_noop_parcel_lookup(is_production)
}

fn build_noop_parcel_lookup(
    is_production: bool,
) -> Result<Arc<dyn ParcelInfoLookup>, StartupError> {
    if is_production {
        return Err(production_config_error(
            "PLATFORM_CORE_API_BASE_URL must be set for parcel_lookup because Platform Core owns catalog parcel data",
        ));
    }
    tracing::warn!(
        "parcel_lookup: PLATFORM_CORE_API_BASE_URL missing - NoOp empty result (dev only)"
    );
    Ok(Arc::new(NoOpParcelInfoLookup::new()))
}

fn build_platform_core_service_auth(
    is_production: bool,
    workload_identity_token_file: Option<String>,
    service_token: Option<String>,
    service_auth_metadata: PlatformCoreServiceAuthMetadataConfig,
) -> Result<Option<PlatformCoreServiceAuth>, StartupError> {
    if let Some(token_file) = workload_identity_token_file {
        if is_production && service_token.is_some() {
            return Err(production_config_error(
                "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE and PLATFORM_CORE_SERVICE_TOKEN must not both be set in production",
            ));
        }
        return PlatformCoreServiceAuth::new_from_workload_identity_token_file(token_file.as_str())
            .map(Some)
            .map_err(|error| production_config_error(error.to_string()));
    }
    if let Some(token) = service_token {
        return PlatformCoreServiceAuth::new_for_environment(
            &token,
            service_auth_metadata,
            is_production,
        )
        .map(Some)
        .map_err(|error| production_config_error(error.to_string()));
    }
    if is_production {
        return Err(production_config_error(
            "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE or PLATFORM_CORE_SERVICE_TOKEN must be set because Platform Core APIs require service-to-service auth",
        ));
    }
    tracing::warn!(
        "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE and PLATFORM_CORE_SERVICE_TOKEN missing - Platform Core calls are unauthenticated in dev only"
    );
    Ok(None)
}

fn platform_core_service_auth_metadata_from_env() -> PlatformCoreServiceAuthMetadataConfig {
    PlatformCoreServiceAuthMetadataConfig {
        scope: optional_env("PLATFORM_CORE_SERVICE_TOKEN_SCOPE"),
        issued_at: optional_env("PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT"),
        expires_at: optional_env("PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT"),
        rotation_owner: optional_env("PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER"),
    }
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

pub fn build_photo_download_issuer(
    is_production: bool,
) -> Result<Arc<dyn photo_upload::ListingPhotoDownloadUrlIssuer>, StartupError> {
    build_photo_download_issuer_from_config_result(
        is_production,
        photo_upload::ListingPhotoUploadConfig::from_env(),
    )
}

pub fn build_photo_download_issuer_from_config_result(
    is_production: bool,
    config_result: Result<
        photo_upload::ListingPhotoUploadConfig,
        photo_upload::ListingPhotoUploadConfigError,
    >,
) -> Result<Arc<dyn photo_upload::ListingPhotoDownloadUrlIssuer>, StartupError> {
    match config_result {
        Ok(config) => Ok(Arc::new(
            photo_upload::R2ListingPhotoDownloadUrlIssuer::new(config),
        )),
        Err(error) if is_production => Err(production_config_error(format!(
            "listing photo download R2 config missing: {error}"
        ))),
        Err(error) => {
            tracing::warn!(
                error = %error,
                "listing photo download R2 config missing - photo download disabled in dev"
            );
            Ok(Arc::new(
                photo_upload::DisabledListingPhotoDownloadUrlIssuer,
            ))
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

#[allow(clippy::disallowed_types)] // JWKS client is owned by the auth boundary, not Catalog integration.
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
) -> Result<Arc<dyn routes::buildings::BuildingRegisterReader>, StartupError> {
    build_building_reader_from_platform_core_base_url_with_service_auth_metadata(
        is_production,
        optional_env("PLATFORM_CORE_API_BASE_URL"),
        optional_env("PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE"),
        optional_env("PLATFORM_CORE_SERVICE_TOKEN"),
        platform_core_service_auth_metadata_from_env(),
    )
}

#[cfg(test)]
fn build_building_reader_from_platform_core_base_url(
    is_production: bool,
    base_url: Option<String>,
    service_token: Option<String>,
) -> Result<Arc<dyn routes::buildings::BuildingRegisterReader>, StartupError> {
    build_building_reader_from_platform_core_base_url_with_service_auth_metadata(
        is_production,
        base_url,
        None,
        service_token,
        PlatformCoreServiceAuthMetadataConfig::default(),
    )
}

fn build_building_reader_from_platform_core_base_url_with_service_auth_metadata(
    is_production: bool,
    base_url: Option<String>,
    workload_identity_token_file: Option<String>,
    service_token: Option<String>,
    service_auth_metadata: PlatformCoreServiceAuthMetadataConfig,
) -> Result<Arc<dyn routes::buildings::BuildingRegisterReader>, StartupError> {
    if let Some(base_url) = base_url {
        let auth = build_platform_core_service_auth(
            is_production,
            workload_identity_token_file,
            service_token,
            service_auth_metadata,
        )?;
        tracing::info!(
            "building_register: Platform Core Catalog live (/catalog/v1/parcels/by-pnu/:pnu/buildings)"
        );
        return building_reader::build_platform_core_building_register_reader(&base_url, auth)
            .map_err(|error| {
                production_config_error(format!(
                    "PLATFORM_CORE_API_BASE_URL invalid for building_register: {error}"
                ))
            });
    }
    if is_production {
        return Err(production_config_error(
            "PLATFORM_CORE_API_BASE_URL must be set for building_register because Platform Core owns catalog building data",
        ));
    }
    tracing::warn!(
        "building_register: PLATFORM_CORE_API_BASE_URL missing - NoOp empty list (dev only)"
    );
    Ok(Arc::new(NoOpBuildingRegisterReader))
}

#[cfg(test)]
mod tests;
