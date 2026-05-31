//! Import Platform Core anchor artifact files into Gongzzang read models.

#![forbid(unsafe_code)]
#![allow(clippy::disallowed_types)]

use std::process::ExitCode;

use db::platform_core_anchor::{
    find_pending_anchor_import_event_ids, import_anchor_rows, mark_inbox_event_failed,
    mark_inbox_event_processed, mark_inbox_event_processing, PlatformCoreAnchorImport,
    PlatformCoreAnchorImportReport,
};
use sqlx::{postgres::PgPoolOptions, PgPool};

#[path = "platform_core_anchor_import/config.rs"]
mod config;
#[path = "platform_core_anchor_import/error.rs"]
mod error;
#[path = "platform_core_anchor_import/lock.rs"]
mod lock;
#[path = "platform_core_anchor_import/source.rs"]
mod source;
#[path = "platform_core_anchor_import/util.rs"]
mod util;

#[path = "../platform_core_anchor_import.rs"]
mod platform_core_anchor_import;

use config::{required_env, ImportSource, ImporterConfig};
use error::AnchorImporterError;
use lock::acquire_optional_event_import_lock;
use platform_core_anchor_import::{parse_anchor_manifest, parse_anchor_rows};
use source::load_import_source;
use util::{truncate_failure_reason, verify_sha256, verify_size_bytes};

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

#[cfg(test)]
#[path = "platform_core_anchor_import/tests.rs"]
mod tests;
