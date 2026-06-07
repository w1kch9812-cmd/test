//! 공짱 outbox publisher daemon — `outbox_event` row 를 폴링해 `Sink` 로 발행.
//!
//! 환경변수:
//! - `DATABASE_URL` (필수) — `Postgres` 접속 문자열
//! - `OUTBOX_POLL_INTERVAL_MS` (기본 1000) — tick 주기
//! - `OUTBOX_BATCH_SIZE` (기본 100) — tick 당 fetch limit
//! - `RUST_LOG` (기본 `info`) — `tracing-subscriber` env filter
//!
//! 종료 신호 (`SIGTERM` / `Ctrl+C`) 받으면 진행 중 tick 완료 후 graceful shutdown.

#![forbid(unsafe_code)]
// `main.rs`: init failure panic은 정답이라 expect/unwrap 허용해요.
// pedantic: `tokio::select!` 매크로 안 redundant_pub_crate / 비공개 main 의
// missing_panics_doc / cfg-gated future 의 redundant_async_block 등 false-positive 차단.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::redundant_pub_crate,
    clippy::redundant_async_block
)]

use std::env;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use auth::platform_core_service::{
    PlatformCoreServiceAuth, PlatformCoreServiceAuthMetadataConfig, PlatformCoreServiceCallPolicy,
};
use db::outbox::PgOutboxRepository;
use outbox_event_domain::repository::OutboxRepository;
use outbox_publisher::{tick, LoggingSink, Sink, SinkError};
use sqlx::postgres::PgPoolOptions;
use thiserror::Error;
use tokio::signal;
use tokio::time;
use tracing::{error, info, warn};

use crate::listing_photo_lakehouse::{
    ListingPhotoLakehouseSink, ListingPhotoR2ReadConfig, R2ListingPhotoObjectReader,
};
use crate::platform_core_lakehouse_registry::PlatformCoreLakehouseRegistryClient;

mod listing_photo_lakehouse;
mod platform_core_lakehouse_registry;

const OUTBOX_LAKEHOUSE_REGISTRY_ENABLED_ENV: &str = "OUTBOX_LAKEHOUSE_REGISTRY_ENABLED";
const PLATFORM_CORE_API_BASE_URL_ENV: &str = "PLATFORM_CORE_API_BASE_URL";
const PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE_ENV: &str =
    "PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE";
const PLATFORM_CORE_SERVICE_TOKEN_ENV: &str = "PLATFORM_CORE_SERVICE_TOKEN";
const PLATFORM_CORE_SERVICE_TOKEN_SCOPE_ENV: &str = "PLATFORM_CORE_SERVICE_TOKEN_SCOPE";
const PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT_ENV: &str = "PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT";
const PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT_ENV: &str = "PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT";
const PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER_ENV: &str =
    "PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let interval_ms: u64 = env::var("OUTBOX_POLL_INTERVAL_MS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1000);
    let batch_size: u32 = env::var("OUTBOX_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to Postgres");

    let repo: Arc<dyn OutboxRepository> = Arc::new(PgOutboxRepository::new(pool));
    let is_production = is_production_env();
    let sink = build_sink(is_production).expect("build outbox sink");

    info!(interval_ms, batch_size, "outbox publisher starting");

    let mut interval = time::interval(Duration::from_millis(interval_ms));
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            _ = interval.tick() => {
                match tick(repo.as_ref(), sink.as_ref(), batch_size).await {
                    Ok(report) if report.fetched > 0 => {
                        info!(
                            fetched = report.fetched,
                            published = report.published,
                            failed = report.failed,
                            "tick"
                        );
                    }
                    Ok(_) => {} // empty tick — silent (운영 spam 방지)
                    Err(e) => error!(error = %e, "tick failed"),
                }
            }
            () = shutdown_signal() => {
                info!("shutdown signal received — stopping");
                break;
            }
        }
    }
}

/// `SIGTERM` (Unix) / `Ctrl+C` 대기.
///
/// Windows 빌드는 `SIGTERM` 미지원 — `pending::<()>()` 로 대체해 `Ctrl+C` 만 동작.
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install ctrl-c handler");
    };
    #[cfg(unix)]
    let term = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = term => {}
    }
}

#[derive(Debug, Error)]
enum OutboxSinkConfigError {
    #[error("{name} must be set")]
    MissingEnv { name: &'static str },
    #[error("{name} must not be empty")]
    EmptyEnv { name: &'static str },
    #[error("{name} must be true, false, 1, or 0")]
    InvalidBoolEnv { name: &'static str },
    #[error("listing photo R2 read config: {0}")]
    ListingPhotoR2(#[from] listing_photo_lakehouse::ListingPhotoR2ReadConfigError),
    #[error("Platform Core service auth: {0}")]
    PlatformCoreServiceAuth(
        #[from] auth::platform_core_service::PlatformCoreServiceAuthConfigError,
    ),
    #[error("Platform Core Lakehouse Registry config: {0}")]
    LakehouseRegistryConfig(
        #[from] platform_core_lakehouse_registry::PlatformCoreLakehouseRegistryConfigError,
    ),
    #[error("{workload_identity_env} and {service_token_env} must not both be set in production")]
    AmbiguousPlatformCoreTokenSources {
        workload_identity_env: &'static str,
        service_token_env: &'static str,
    },
}

fn build_sink(is_production: bool) -> Result<Box<dyn Sink>, OutboxSinkConfigError> {
    if !lakehouse_registry_enabled(is_production)? {
        warn!(
            "outbox lakehouse registry sink disabled - listing photo media lineage is not registered"
        );
        return Ok(Box::new(LoggingSink::new()));
    }

    let reader = Arc::new(R2ListingPhotoObjectReader::new(
        ListingPhotoR2ReadConfig::from_env()?,
    ));
    let service_auth = build_worker_platform_core_service_auth(is_production)?;
    let registry = Arc::new(PlatformCoreLakehouseRegistryClient::new(
        &required_env(PLATFORM_CORE_API_BASE_URL_ENV)?,
        service_auth,
    )?);
    let listing_photo_sink = ListingPhotoLakehouseSink::new(reader, registry);
    Ok(Box::new(FanoutSink::new(
        LoggingSink::new(),
        listing_photo_sink,
    )))
}

fn lakehouse_registry_enabled(is_production: bool) -> Result<bool, OutboxSinkConfigError> {
    lakehouse_registry_enabled_value(
        is_production,
        optional_env(OUTBOX_LAKEHOUSE_REGISTRY_ENABLED_ENV).as_deref(),
    )
}

fn lakehouse_registry_enabled_value(
    is_production: bool,
    value: Option<&str>,
) -> Result<bool, OutboxSinkConfigError> {
    match value {
        None => Ok(is_production),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(_) => Err(OutboxSinkConfigError::InvalidBoolEnv {
            name: OUTBOX_LAKEHOUSE_REGISTRY_ENABLED_ENV,
        }),
    }
}

fn build_worker_platform_core_service_auth(
    is_production: bool,
) -> Result<PlatformCoreServiceAuth, OutboxSinkConfigError> {
    let workload_identity_token_file = optional_env(PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE_ENV);
    let service_token = optional_env(PLATFORM_CORE_SERVICE_TOKEN_ENV);
    let call_policy = PlatformCoreServiceCallPolicy::gongzzang_worker_lakehouse_registry_write();
    if let Some(token_file) = workload_identity_token_file {
        if is_production && service_token.is_some() {
            return Err(OutboxSinkConfigError::AmbiguousPlatformCoreTokenSources {
                workload_identity_env: PLATFORM_CORE_WORKLOAD_IDENTITY_TOKEN_FILE_ENV,
                service_token_env: PLATFORM_CORE_SERVICE_TOKEN_ENV,
            });
        }
        return PlatformCoreServiceAuth::new_from_workload_identity_token_file(token_file.as_str())
            .map(|auth| auth.with_call_policy(call_policy))
            .map_err(OutboxSinkConfigError::from);
    }
    let token = required_env(PLATFORM_CORE_SERVICE_TOKEN_ENV)?;
    PlatformCoreServiceAuth::new_for_environment_with_call_policy(
        &token,
        platform_core_service_auth_metadata_from_env(),
        is_production,
        call_policy,
    )
    .map_err(OutboxSinkConfigError::from)
}

fn platform_core_service_auth_metadata_from_env() -> PlatformCoreServiceAuthMetadataConfig {
    PlatformCoreServiceAuthMetadataConfig {
        scope: optional_env(PLATFORM_CORE_SERVICE_TOKEN_SCOPE_ENV),
        issued_at: optional_env(PLATFORM_CORE_SERVICE_TOKEN_ISSUED_AT_ENV),
        expires_at: optional_env(PLATFORM_CORE_SERVICE_TOKEN_EXPIRES_AT_ENV),
        rotation_owner: optional_env(PLATFORM_CORE_SERVICE_TOKEN_ROTATION_OWNER_ENV),
    }
}

fn is_production_env() -> bool {
    env::var("APP_ENV").as_deref() == Ok("production")
        || env::var("NODE_ENV").as_deref() == Ok("production")
}

fn required_env(name: &'static str) -> Result<String, OutboxSinkConfigError> {
    let value = env::var(name).map_err(|_| OutboxSinkConfigError::MissingEnv { name })?;
    let value = value.trim().to_owned();
    if value.is_empty() {
        return Err(OutboxSinkConfigError::EmptyEnv { name });
    }
    Ok(value)
}

fn optional_env(name: &'static str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Debug)]
struct FanoutSink<A, B> {
    first: A,
    second: B,
}

impl<A, B> FanoutSink<A, B> {
    const fn new(first: A, second: B) -> Self {
        Self { first, second }
    }
}

#[async_trait]
impl<A, B> Sink for FanoutSink<A, B>
where
    A: Sink + Send + Sync,
    B: Sink + Send + Sync,
{
    async fn publish(
        &self,
        event: &outbox_event_domain::entity::OutboxEvent,
    ) -> Result<(), SinkError> {
        self.first.publish(event).await?;
        self.second.publish(event).await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn lakehouse_registry_defaults_to_enabled_in_production() {
        assert!(lakehouse_registry_enabled_value(true, None).expect("enabled"));
    }

    #[test]
    fn lakehouse_registry_defaults_to_disabled_outside_production() {
        assert!(!lakehouse_registry_enabled_value(false, None).expect("disabled"));
    }
}
