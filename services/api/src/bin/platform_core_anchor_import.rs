//! Import Platform Core anchor artifact files into Gongzzang read models.

#![forbid(unsafe_code)]
#![allow(clippy::disallowed_types)]

use std::{env, fs, process::ExitCode};

use chrono::{DateTime, Utc};
use circuit_breaker::{execute, Breaker, Policy};
use db::platform_core_anchor::{
    find_inbox_event_payload, find_pending_anchor_import_event_ids, import_anchor_rows,
    mark_inbox_event_failed, mark_inbox_event_processed, mark_inbox_event_processing,
    PlatformCoreAnchorImport, PlatformCoreAnchorImportReport,
};
use sha2::{Digest, Sha256};
use sqlx::{pool::PoolConnection, postgres::PgPoolOptions, PgPool, Postgres};
use thiserror::Error;

#[path = "../platform_core_anchor_import.rs"]
mod platform_core_anchor_import;

use platform_core_anchor_import::{parse_anchor_manifest, parse_anchor_rows, AnchorImportError};

const DATABASE_URL_ENV: &str = "DATABASE_URL";
const MANIFEST_PATH_ENV: &str = "PLATFORM_CORE_ANCHOR_MANIFEST_PATH";
const OBJECT_PATHS_ENV: &str = "PLATFORM_CORE_ANCHOR_OBJECT_PATHS";
const ANCHOR_SNAPSHOT_ID_ENV: &str = "PLATFORM_CORE_ANCHOR_SNAPSHOT_ID";
const SOURCE_GEOMETRY_VERSION_ENV: &str = "PLATFORM_CORE_SOURCE_GEOMETRY_VERSION";
const PUBLISHED_AT_ENV: &str = "PLATFORM_CORE_ANCHOR_PUBLISHED_AT";
const EVENT_ID_ENV: &str = "PLATFORM_CORE_EVENT_ID";
const BATCH_LIMIT_ENV: &str = "PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT";
const DEFAULT_BATCH_LIMIT: i64 = 10;
const MAX_BATCH_LIMIT: i64 = 100;
const MAX_FAILURE_REASON_LEN: usize = 2_000;

#[tokio::main]
async fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt::try_init();

    match run().await {
        Ok(report) => {
            tracing::info!(
                upserted_anchor_count = report.upserted_anchor_count,
                refreshed_listing_projection_count = report.refreshed_listing_projection_count,
                "platform core anchor import completed"
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            tracing::error!(error = %error, "platform core anchor import failed");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<PlatformCoreAnchorImportReport, AnchorImporterError> {
    let database_url = required_env(DATABASE_URL_ENV)?;
    let config = ImporterConfig::from_env()?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;

    if let ImportSource::PendingInboxBatch { batch_limit } = &config.source {
        return run_pending_inbox_batch(&pool, *batch_limit).await;
    }

    run_single_import(&pool, &config).await
}

async fn run_single_import(
    pool: &PgPool,
    config: &ImporterConfig,
) -> Result<PlatformCoreAnchorImportReport, AnchorImporterError> {
    let event_lock = acquire_optional_event_import_lock(pool, config.event_id.as_deref()).await?;
    let result = run_with_event_status(pool, config).await;
    if let Some(lock) = event_lock {
        if let Err(release_error) = lock.release().await {
            if result.is_ok() {
                return Err(release_error);
            }
            tracing::error!(
                error = %release_error,
                "failed to release platform core event import advisory lock"
            );
        }
    }
    result
}

async fn run_pending_inbox_batch(
    pool: &PgPool,
    batch_limit: i64,
) -> Result<PlatformCoreAnchorImportReport, AnchorImporterError> {
    let event_ids = find_pending_anchor_import_event_ids(pool, batch_limit).await?;
    let mut report = PlatformCoreAnchorImportReport {
        upserted_anchor_count: 0,
        refreshed_listing_projection_count: 0,
    };
    let mut failed_count = 0_u64;

    for event_id in event_ids {
        let config = ImporterConfig {
            source: ImportSource::EventPayload,
            event_id: Some(event_id.clone()),
        };
        match run_single_import(pool, &config).await {
            Ok(event_report) => {
                report.upserted_anchor_count = report
                    .upserted_anchor_count
                    .saturating_add(event_report.upserted_anchor_count);
                report.refreshed_listing_projection_count = report
                    .refreshed_listing_projection_count
                    .saturating_add(event_report.refreshed_listing_projection_count);
            }
            Err(AnchorImporterError::InboxEventAlreadyLocked { .. }) => {
                tracing::info!(
                    event_id = %event_id,
                    "platform core anchor import event is already locked"
                );
            }
            Err(error) => {
                failed_count = failed_count.saturating_add(1);
                tracing::error!(
                    event_id = %event_id,
                    error = %error,
                    "platform core anchor import event failed in batch"
                );
            }
        }
    }

    if failed_count > 0 {
        return Err(AnchorImporterError::BatchImportFailed { failed_count });
    }

    Ok(report)
}

async fn run_with_event_status(
    pool: &PgPool,
    config: &ImporterConfig,
) -> Result<PlatformCoreAnchorImportReport, AnchorImporterError> {
    if let Some(event_id) = &config.event_id {
        let claimed = mark_inbox_event_processing(pool, event_id).await?;
        if !claimed {
            return Err(AnchorImporterError::InboxEventNotPending {
                event_id: event_id.clone(),
            });
        }
    }

    let result = run_import(pool, config).await;
    match result {
        Ok(report) => {
            if let Some(event_id) = &config.event_id {
                mark_inbox_event_processed(pool, event_id).await?;
            }
            Ok(report)
        }
        Err(error) => {
            if let Some(event_id) = &config.event_id {
                let reason = truncate_failure_reason(&error.to_string());
                if let Err(mark_error) = mark_inbox_event_failed(pool, event_id, &reason).await {
                    tracing::error!(
                        event_id = %event_id,
                        error = %mark_error,
                        "failed to mark platform core anchor import event as failed"
                    );
                }
            }
            Err(error)
        }
    }
}

async fn run_import(
    pool: &PgPool,
    config: &ImporterConfig,
) -> Result<PlatformCoreAnchorImportReport, AnchorImporterError> {
    let source = load_import_source(pool, config).await?;
    let manifest_text = String::from_utf8(source.manifest_bytes).map_err(|source| {
        AnchorImporterError::InvalidUtf8 {
            path: "manifest".to_owned(),
            source,
        }
    })?;
    let manifest = parse_anchor_manifest(&manifest_text)?;
    if source.object_bytes.len() != manifest.objects.len() {
        return Err(AnchorImporterError::ObjectCountMismatch {
            expected: manifest.objects.len(),
            actual: source.object_bytes.len(),
        });
    }

    let mut rows = Vec::new();
    for (artifact, object) in source.object_bytes.iter().zip(manifest.objects.iter()) {
        verify_size_bytes(
            artifact.bytes.len(),
            object.size_bytes,
            "object",
            &object.artifact_object_key,
        )?;
        verify_sha256(&artifact.bytes, &object.checksum_sha256, "object")?;
        let object_text = String::from_utf8(artifact.bytes.clone()).map_err(|source| {
            AnchorImporterError::InvalidUtf8 {
                path: artifact.label.clone(),
                source,
            }
        })?;
        rows.extend(parse_anchor_rows(&object_text, object.row_count)?);
    }
    if rows.len()
        != usize::try_from(manifest.artifact_row_count)
            .map_err(|_| AnchorImporterError::ArtifactRowCountOverflow)?
    {
        return Err(AnchorImporterError::ArtifactRowCountMismatch {
            expected: manifest.artifact_row_count,
            actual: rows.len(),
        });
    }

    let import = PlatformCoreAnchorImport {
        anchor_snapshot_id: source.anchor_snapshot_id,
        source_geometry_version: source.source_geometry_version,
        platform_core_updated_at: source.platform_core_updated_at,
        rows,
    };

    Ok(import_anchor_rows(pool, &import).await?)
}

struct ImporterConfig {
    source: ImportSource,
    event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ImportSource {
    Local {
        manifest_path: String,
        object_paths: Vec<String>,
        anchor_snapshot_id: String,
        source_geometry_version: String,
        platform_core_updated_at: DateTime<Utc>,
    },
    EventPayload,
    PendingInboxBatch {
        batch_limit: i64,
    },
}

impl ImporterConfig {
    fn from_env() -> Result<Self, AnchorImporterError> {
        let event_id = optional_env(EVENT_ID_ENV);
        let source = import_source_from_env_values(ImportSourceEnvValues {
            manifest_path: optional_env(MANIFEST_PATH_ENV),
            object_paths: optional_env(OBJECT_PATHS_ENV),
            anchor_snapshot_id: optional_env(ANCHOR_SNAPSHOT_ID_ENV),
            source_geometry_version: optional_env(SOURCE_GEOMETRY_VERSION_ENV),
            published_at: optional_env(PUBLISHED_AT_ENV),
            event_id: event_id.clone(),
            batch_limit: optional_env(BATCH_LIMIT_ENV),
        })?;

        Ok(Self { source, event_id })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ImportSourceEnvValues {
    manifest_path: Option<String>,
    object_paths: Option<String>,
    anchor_snapshot_id: Option<String>,
    source_geometry_version: Option<String>,
    published_at: Option<String>,
    event_id: Option<String>,
    batch_limit: Option<String>,
}

fn import_source_from_env_values(
    values: ImportSourceEnvValues,
) -> Result<ImportSource, AnchorImporterError> {
    let local_mode = values.manifest_path.is_some()
        || values.object_paths.is_some()
        || values.anchor_snapshot_id.is_some()
        || values.source_geometry_version.is_some()
        || values.published_at.is_some();
    if local_mode {
        return Ok(ImportSource::Local {
            manifest_path: required_config_value(values.manifest_path, MANIFEST_PATH_ENV)?,
            object_paths: object_paths_from_value(&required_config_value(
                values.object_paths,
                OBJECT_PATHS_ENV,
            )?)?,
            anchor_snapshot_id: required_config_value(
                values.anchor_snapshot_id,
                ANCHOR_SNAPSHOT_ID_ENV,
            )?,
            source_geometry_version: required_config_value(
                values.source_geometry_version,
                SOURCE_GEOMETRY_VERSION_ENV,
            )?,
            platform_core_updated_at: parse_rfc3339_utc(&required_config_value(
                values.published_at,
                PUBLISHED_AT_ENV,
            )?)?,
        });
    }

    if values.event_id.is_some() {
        return Ok(ImportSource::EventPayload);
    }

    Ok(ImportSource::PendingInboxBatch {
        batch_limit: parse_batch_limit(values.batch_limit.as_deref())?,
    })
}

fn required_config_value(
    value: Option<String>,
    name: &'static str,
) -> Result<String, AnchorImporterError> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or(AnchorImporterError::MissingEnv { name })
}

fn parse_batch_limit(value: Option<&str>) -> Result<i64, AnchorImporterError> {
    let Some(value) = value else {
        return Ok(DEFAULT_BATCH_LIMIT);
    };
    let parsed = value
        .trim()
        .parse::<i64>()
        .map_err(|_| AnchorImporterError::InvalidBatchLimit)?;
    if (1..=MAX_BATCH_LIMIT).contains(&parsed) {
        return Ok(parsed);
    }

    Err(AnchorImporterError::InvalidBatchLimit)
}

fn required_env(name: &'static str) -> Result<String, AnchorImporterError> {
    env::var(name)
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
        .ok_or(AnchorImporterError::MissingEnv { name })
}

fn optional_env(name: &'static str) -> Option<String> {
    env::var(name)
        .map(|value| value.trim().to_owned())
        .ok()
        .filter(|value| !value.is_empty())
}

fn object_paths_from_value(value: &str) -> Result<Vec<String>, AnchorImporterError> {
    let paths = value
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Err(AnchorImporterError::MissingEnv {
            name: OBJECT_PATHS_ENV,
        });
    }
    Ok(paths)
}

struct LoadedImportSource {
    manifest_bytes: Vec<u8>,
    object_bytes: Vec<ArtifactBytes>,
    anchor_snapshot_id: String,
    source_geometry_version: String,
    platform_core_updated_at: DateTime<Utc>,
}

struct ArtifactBytes {
    label: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EventArtifactConfig {
    anchor_snapshot_id: String,
    source_geometry_version: String,
    platform_core_updated_at: DateTime<Utc>,
    artifact_manifest_url: reqwest::Url,
    artifact_checksum_sha256: String,
}

async fn load_import_source(
    pool: &PgPool,
    config: &ImporterConfig,
) -> Result<LoadedImportSource, AnchorImporterError> {
    match &config.source {
        ImportSource::Local {
            manifest_path,
            object_paths,
            anchor_snapshot_id,
            source_geometry_version,
            platform_core_updated_at,
        } => load_local_import_source(
            manifest_path,
            object_paths,
            anchor_snapshot_id,
            source_geometry_version,
            *platform_core_updated_at,
        ),
        ImportSource::EventPayload => {
            let event_id = config
                .event_id
                .as_deref()
                .ok_or(AnchorImporterError::MissingEnv { name: EVENT_ID_ENV })?;
            load_event_payload_import_source(pool, event_id).await
        }
        ImportSource::PendingInboxBatch { .. } => Err(AnchorImporterError::BatchSourceInSingleRun),
    }
}

fn load_local_import_source(
    manifest_path: &str,
    object_paths: &[String],
    anchor_snapshot_id: &str,
    source_geometry_version: &str,
    platform_core_updated_at: DateTime<Utc>,
) -> Result<LoadedImportSource, AnchorImporterError> {
    let manifest_bytes =
        fs::read(manifest_path).map_err(|source| AnchorImporterError::ReadFile {
            path: manifest_path.to_owned(),
            source,
        })?;
    let object_bytes = object_paths
        .iter()
        .map(|path| {
            let bytes = fs::read(path).map_err(|source| AnchorImporterError::ReadFile {
                path: path.clone(),
                source,
            })?;
            Ok(ArtifactBytes {
                label: path.clone(),
                bytes,
            })
        })
        .collect::<Result<Vec<_>, AnchorImporterError>>()?;

    Ok(LoadedImportSource {
        manifest_bytes,
        object_bytes,
        anchor_snapshot_id: anchor_snapshot_id.to_owned(),
        source_geometry_version: source_geometry_version.to_owned(),
        platform_core_updated_at,
    })
}

async fn load_event_payload_import_source(
    pool: &PgPool,
    event_id: &str,
) -> Result<LoadedImportSource, AnchorImporterError> {
    let payload = find_inbox_event_payload(pool, event_id)
        .await?
        .ok_or_else(|| AnchorImporterError::InboxEventNotFound {
            event_id: event_id.to_owned(),
        })?;
    let event_config = event_artifact_config_from_payload(&payload)?;
    let artifact_http = ArtifactHttpClient::new()?;

    let manifest_bytes = artifact_http
        .fetch_bytes(&event_config.artifact_manifest_url)
        .await?;
    verify_sha256(
        &manifest_bytes,
        &event_config.artifact_checksum_sha256,
        "manifest",
    )?;
    let manifest_text = String::from_utf8(manifest_bytes.clone()).map_err(|source| {
        AnchorImporterError::InvalidUtf8 {
            path: event_config.artifact_manifest_url.to_string(),
            source,
        }
    })?;
    let manifest = parse_anchor_manifest(&manifest_text)?;
    let mut object_bytes = Vec::new();
    for object in &manifest.objects {
        let object_url = resolve_artifact_object_url(
            &event_config.artifact_manifest_url,
            &object.artifact_object_key,
        )?;
        object_bytes.push(ArtifactBytes {
            label: object_url.to_string(),
            bytes: artifact_http.fetch_bytes(&object_url).await?,
        });
    }

    Ok(LoadedImportSource {
        manifest_bytes,
        object_bytes,
        anchor_snapshot_id: event_config.anchor_snapshot_id,
        source_geometry_version: event_config.source_geometry_version,
        platform_core_updated_at: event_config.platform_core_updated_at,
    })
}

fn event_artifact_config_from_payload(
    payload: &serde_json::Value,
) -> Result<EventArtifactConfig, AnchorImporterError> {
    let anchor_snapshot_id = payload_string(payload, "anchor_snapshot_id")?;
    let source_geometry_version = payload_string(payload, "source_geometry_version")?;
    let artifact_manifest_url_value = payload_string(payload, "artifact_manifest_url")?;
    let artifact_manifest_url =
        reqwest::Url::parse(&artifact_manifest_url_value).map_err(|_| {
            AnchorImporterError::InvalidEventPayload {
                field: "artifact_manifest_url",
            }
        })?;
    if artifact_manifest_url.scheme() != "https" {
        return Err(AnchorImporterError::InvalidEventPayload {
            field: "artifact_manifest_url",
        });
    }
    let artifact_checksum_sha256 = payload_string(payload, "artifact_checksum_sha256")?;
    validate_sha256_hex(&artifact_checksum_sha256)?;
    let platform_core_updated_at = parse_rfc3339_utc(&payload_string(payload, "published_at")?)?;

    Ok(EventArtifactConfig {
        anchor_snapshot_id,
        source_geometry_version,
        platform_core_updated_at,
        artifact_manifest_url,
        artifact_checksum_sha256,
    })
}

fn payload_string(
    payload: &serde_json::Value,
    field: &'static str,
) -> Result<String, AnchorImporterError> {
    payload
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(AnchorImporterError::InvalidEventPayload { field })
}

fn validate_sha256_hex(value: &str) -> Result<(), AnchorImporterError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(());
    }

    Err(AnchorImporterError::InvalidEventPayload {
        field: "artifact_checksum_sha256",
    })
}

fn resolve_artifact_object_url(
    manifest_url: &reqwest::Url,
    artifact_object_key: &str,
) -> Result<reqwest::Url, AnchorImporterError> {
    if artifact_object_key.trim() != artifact_object_key
        || artifact_object_key.is_empty()
        || artifact_object_key.starts_with('/')
        || artifact_object_key.contains('\\')
        || artifact_object_key.contains("..")
    {
        return Err(AnchorImporterError::InvalidArtifactObjectKey {
            object_key: artifact_object_key.to_owned(),
        });
    }

    manifest_url.join(artifact_object_key).map_err(|_| {
        AnchorImporterError::InvalidArtifactObjectKey {
            object_key: artifact_object_key.to_owned(),
        }
    })
}

struct ArtifactHttpClient {
    client: reqwest::Client,
    breaker: Breaker,
    policy: Policy,
}

impl ArtifactHttpClient {
    fn new() -> Result<Self, AnchorImporterError> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|source| AnchorImporterError::HttpClient { source })?;
        Ok(Self {
            client,
            breaker: Breaker::new(),
            policy: Policy::platform_core_default(),
        })
    }

    async fn fetch_bytes(&self, url: &reqwest::Url) -> Result<Vec<u8>, AnchorImporterError> {
        execute(
            &self.breaker,
            &self.policy,
            "platform_core.anchor_artifact.fetch",
            || {
                let client = self.client.clone();
                let url = url.clone();
                async move { fetch_artifact_bytes_once(&client, &url).await }
            },
        )
        .await
        .map_err(|source| AnchorImporterError::FetchArtifactCircuit {
            url: url.to_string(),
            error: source.to_string(),
        })
    }
}

async fn fetch_artifact_bytes_once(
    client: &reqwest::Client,
    url: &reqwest::Url,
) -> Result<Vec<u8>, AnchorImporterError> {
    let response = client.get(url.clone()).send().await.map_err(|source| {
        AnchorImporterError::FetchArtifact {
            url: url.to_string(),
            source,
        }
    })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AnchorImporterError::FetchArtifactStatus {
            url: url.to_string(),
            status,
        });
    }

    Ok(response
        .bytes()
        .await
        .map_err(|source| AnchorImporterError::FetchArtifact {
            url: url.to_string(),
            source,
        })?
        .to_vec())
}

fn parse_rfc3339_utc(value: &str) -> Result<DateTime<Utc>, AnchorImporterError> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}

fn truncate_failure_reason(reason: &str) -> String {
    reason.chars().take(MAX_FAILURE_REASON_LEN).collect()
}

struct EventImportLock {
    event_id: String,
    key: i64,
    connection: PoolConnection<Postgres>,
}

impl EventImportLock {
    async fn release(mut self) -> Result<(), AnchorImporterError> {
        let released: bool = sqlx::query_scalar("select pg_advisory_unlock($1)")
            .bind(self.key)
            .fetch_one(&mut *self.connection)
            .await?;
        if released {
            return Ok(());
        }

        Err(AnchorImporterError::EventImportLockReleaseFailed {
            event_id: self.event_id,
        })
    }
}

async fn acquire_optional_event_import_lock(
    pool: &PgPool,
    event_id: Option<&str>,
) -> Result<Option<EventImportLock>, AnchorImporterError> {
    let Some(event_id) = event_id else {
        return Ok(None);
    };

    let key = event_import_lock_key(event_id);
    let mut connection = pool.acquire().await?;
    let acquired: bool = sqlx::query_scalar("select pg_try_advisory_lock($1)")
        .bind(key)
        .fetch_one(&mut *connection)
        .await?;
    if !acquired {
        return Err(AnchorImporterError::InboxEventAlreadyLocked {
            event_id: event_id.to_owned(),
        });
    }

    Ok(Some(EventImportLock {
        event_id: event_id.to_owned(),
        key,
        connection,
    }))
}

fn event_import_lock_key(event_id: &str) -> i64 {
    let digest = Sha256::digest(event_id.as_bytes());
    i64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ])
}

fn verify_size_bytes(
    actual: usize,
    expected: u64,
    label: &'static str,
    object_key: &str,
) -> Result<(), AnchorImporterError> {
    if u64::try_from(actual).map_err(|_| AnchorImporterError::ArtifactObjectSizeOverflow)?
        == expected
    {
        return Ok(());
    }

    Err(AnchorImporterError::SizeMismatch {
        label,
        object_key: object_key.to_owned(),
        expected,
        actual,
    })
}

fn verify_sha256(
    bytes: &[u8],
    expected: &str,
    label: &'static str,
) -> Result<(), AnchorImporterError> {
    let digest = Sha256::digest(bytes);
    let actual = digest
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            use std::fmt::Write as _;
            let _ = write!(&mut output, "{byte:02x}");
            output
        });
    if actual == expected {
        return Ok(());
    }

    Err(AnchorImporterError::ChecksumMismatch {
        label,
        expected: expected.to_owned(),
        actual,
    })
}

#[derive(Debug, Error)]
enum AnchorImporterError {
    #[error("{name} must be set")]
    MissingEnv { name: &'static str },
    #[error("failed to read {path}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("artifact file is not UTF-8 JSONL: {path}")]
    InvalidUtf8 {
        path: String,
        #[source]
        source: std::string::FromUtf8Error,
    },
    #[error("manifest object count mismatch: expected {expected}, got {actual}")]
    ObjectCountMismatch { expected: usize, actual: usize },
    #[error("platform core event inbox row was not found: event_id={event_id}")]
    InboxEventNotFound { event_id: String },
    #[error("platform core event inbox row is not pending_import: event_id={event_id}")]
    InboxEventNotPending { event_id: String },
    #[error("platform core event inbox row is already locked for import: event_id={event_id}")]
    InboxEventAlreadyLocked { event_id: String },
    #[error("failed to release platform core event import advisory lock: event_id={event_id}")]
    EventImportLockReleaseFailed { event_id: String },
    #[error("invalid platform core event payload field: {field}")]
    InvalidEventPayload { field: &'static str },
    #[error("invalid platform core anchor artifact object key: {object_key}")]
    InvalidArtifactObjectKey { object_key: String },
    #[error("PLATFORM_CORE_ANCHOR_IMPORT_BATCH_LIMIT must be between 1 and 100")]
    InvalidBatchLimit,
    #[error("pending inbox batch source cannot be loaded by a single import run")]
    BatchSourceInSingleRun,
    #[error("platform core anchor import batch failed for {failed_count} event(s)")]
    BatchImportFailed { failed_count: u64 },
    #[error("failed to build artifact HTTP client")]
    HttpClient {
        #[source]
        source: reqwest::Error,
    },
    #[error("failed to fetch platform core artifact: {url}")]
    FetchArtifact {
        url: String,
        #[source]
        source: reqwest::Error,
    },
    #[error("platform core artifact fetch failed: {url} returned {status}")]
    FetchArtifactStatus {
        url: String,
        status: reqwest::StatusCode,
    },
    #[error("platform core artifact fetch circuit failed: {url}: {error}")]
    FetchArtifactCircuit { url: String, error: String },
    #[error("manifest artifact row count is too large for this process")]
    ArtifactRowCountOverflow,
    #[error("manifest object byte size is too large for this process")]
    ArtifactObjectSizeOverflow,
    #[error("{label} artifact size mismatch for {object_key}: expected {expected}, got {actual}")]
    SizeMismatch {
        label: &'static str,
        object_key: String,
        expected: u64,
        actual: usize,
    },
    #[error("{label} artifact checksum mismatch")]
    ChecksumMismatch {
        label: &'static str,
        expected: String,
        actual: String,
    },
    #[error("manifest artifact row count mismatch: expected {expected}, got {actual}")]
    ArtifactRowCountMismatch { expected: u64, actual: usize },
    #[error("invalid RFC3339 timestamp")]
    Timestamp(#[from] chrono::ParseError),
    #[error(transparent)]
    Anchor(#[from] AnchorImportError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Repository(#[from] listing_domain::repository::RepoError),
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use super::*;

    #[test]
    fn verifies_sha256_digest_for_artifact_bytes() {
        verify_sha256(
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "object",
        )
        .expect("checksum");
    }

    #[test]
    fn rejects_sha256_digest_mismatch_for_artifact_bytes() {
        let error = verify_sha256(b"abc", &"0".repeat(64), "object").expect_err("mismatch");

        assert!(matches!(
            error,
            AnchorImporterError::ChecksumMismatch {
                label: "object",
                ..
            }
        ));
    }

    #[test]
    fn truncates_failure_reason_on_char_boundary() {
        let reason = format!("{}\u{ac00}", "a".repeat(MAX_FAILURE_REASON_LEN));

        assert_eq!(
            truncate_failure_reason(&reason),
            "a".repeat(MAX_FAILURE_REASON_LEN)
        );
    }

    #[test]
    fn derives_stable_signed_event_import_lock_key() {
        assert_eq!(
            event_import_lock_key("0196f0b0-3e01-7000-8000-000000000005"),
            7_950_551_788_526_988_078
        );
    }

    #[test]
    fn selects_event_payload_source_when_local_artifact_paths_are_absent() {
        let config = import_source_from_env_values(ImportSourceEnvValues {
            event_id: Some("0196f0b0-3e01-7000-8000-000000000006".to_owned()),
            ..ImportSourceEnvValues::default()
        })
        .expect("event payload source");

        assert!(matches!(config, ImportSource::EventPayload));
    }

    #[test]
    fn selects_pending_inbox_batch_source_when_no_single_event_or_local_paths_are_set() {
        let config = import_source_from_env_values(ImportSourceEnvValues::default())
            .expect("pending inbox batch source");

        assert!(matches!(
            config,
            ImportSource::PendingInboxBatch {
                batch_limit: DEFAULT_BATCH_LIMIT
            }
        ));
    }

    #[test]
    fn parses_pending_inbox_batch_limit_from_env() {
        let config = import_source_from_env_values(ImportSourceEnvValues {
            batch_limit: Some("25".to_owned()),
            ..ImportSourceEnvValues::default()
        })
        .expect("pending inbox batch source");

        assert!(matches!(
            config,
            ImportSource::PendingInboxBatch { batch_limit: 25 }
        ));
    }

    #[test]
    fn derives_remote_artifact_config_from_event_payload() {
        let config = event_artifact_config_from_payload(&serde_json::json!({
            "anchor_snapshot_id": "anchor-snapshot-20260528T120000Z",
            "source_geometry_version": "silver.parcel_boundaries@20260528",
            "artifact_manifest_url": "https://platform-core.example.com/artifacts/anchors/manifest.json",
            "artifact_checksum_sha256": "a".repeat(64),
            "published_at": "2026-05-28T12:00:00Z"
        }))
        .expect("event artifact config");

        assert_eq!(
            config.anchor_snapshot_id,
            "anchor-snapshot-20260528T120000Z"
        );
        assert_eq!(
            config.source_geometry_version,
            "silver.parcel_boundaries@20260528"
        );
        assert_eq!(
            config.artifact_manifest_url.as_str(),
            "https://platform-core.example.com/artifacts/anchors/manifest.json"
        );
        assert_eq!(config.artifact_checksum_sha256, "a".repeat(64));
        assert_eq!(
            config.platform_core_updated_at,
            "2026-05-28T12:00:00Z"
                .parse::<DateTime<Utc>>()
                .expect("fixture timestamp must be valid RFC3339")
        );
    }

    #[test]
    fn resolves_object_url_relative_to_manifest_directory() {
        let url = resolve_artifact_object_url(
            &reqwest::Url::parse(
                "https://platform-core.example.com/artifacts/anchors/manifest.json",
            )
            .expect("fixture manifest URL must be valid"),
            "gold/parcel-marker-anchors/shard-000001.jsonl",
        )
        .expect("object url");

        assert_eq!(
            url.as_str(),
            "https://platform-core.example.com/artifacts/anchors/gold/parcel-marker-anchors/shard-000001.jsonl"
        );
    }
}
