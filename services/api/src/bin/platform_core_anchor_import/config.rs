use std::env;

use chrono::{DateTime, Utc};

use super::error::AnchorImporterError;
use super::util::parse_rfc3339_utc;
use super::{
    ANCHOR_SNAPSHOT_ID_ENV, BATCH_LIMIT_ENV, DEFAULT_BATCH_LIMIT, EVENT_ID_ENV, MANIFEST_PATH_ENV,
    MAX_BATCH_LIMIT, OBJECT_PATHS_ENV, PUBLISHED_AT_ENV, SOURCE_GEOMETRY_VERSION_ENV,
};

pub struct ImporterConfig {
    pub source: ImportSource,
    pub event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportSource {
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
    pub(super) fn from_env() -> Result<Self, AnchorImporterError> {
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
pub struct ImportSourceEnvValues {
    pub manifest_path: Option<String>,
    pub object_paths: Option<String>,
    pub anchor_snapshot_id: Option<String>,
    pub source_geometry_version: Option<String>,
    pub published_at: Option<String>,
    pub event_id: Option<String>,
    pub batch_limit: Option<String>,
}

pub fn import_source_from_env_values(
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

pub fn required_env(name: &'static str) -> Result<String, AnchorImporterError> {
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
