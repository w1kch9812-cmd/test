use std::fs;

use chrono::{DateTime, Utc};
use circuit_breaker::{execute, Breaker, Policy};
use db::platform_core_anchor::find_inbox_event_payload;
use sqlx::PgPool;

use super::config::{ImportSource, ImporterConfig};
use super::error::AnchorImporterError;
use super::platform_core_anchor_import::parse_anchor_manifest;
use super::util::{parse_rfc3339_utc, verify_sha256};
use super::EVENT_ID_ENV;

pub struct LoadedImportSource {
    pub manifest_bytes: Vec<u8>,
    pub object_bytes: Vec<ArtifactBytes>,
    pub anchor_snapshot_id: String,
    pub source_geometry_version: String,
    pub platform_core_updated_at: DateTime<Utc>,
}

pub struct ArtifactBytes {
    pub label: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventArtifactConfig {
    pub anchor_snapshot_id: String,
    pub source_geometry_version: String,
    pub platform_core_updated_at: DateTime<Utc>,
    pub artifact_manifest_url: reqwest::Url,
    pub artifact_checksum_sha256: String,
}

pub async fn load_import_source(
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

pub fn event_artifact_config_from_payload(
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

pub fn resolve_artifact_object_url(
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
