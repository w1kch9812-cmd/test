//! Platform Core anchor artifact import contract parsing.

use serde::Deserialize;
use thiserror::Error;

pub use db::platform_core_anchor::AnchorArtifactRow;

const MANIFEST_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_manifest.v1";
const ENTRY_SCHEMA_VERSION: &str = "platform-core.parcel_marker_anchor_artifact_entry.v1";

/// Platform Core anchor artifact manifest after contract validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorArtifactManifest {
    /// Immutable artifact version.
    pub artifact_version: String,
    /// Platform Core source snapshot id.
    pub source_snapshot_id: String,
    /// Source table name.
    pub source_table: String,
    /// Source geometry SRID.
    pub source_srid: String,
    /// Anchor point SRID.
    pub anchor_srid: String,
    /// Anchor algorithm.
    pub algorithm: String,
    /// Anchor algorithm version.
    pub algorithm_version: String,
    /// Total accepted anchor rows.
    pub artifact_row_count: u64,
    /// Total rejected rows.
    pub rejected_row_count: u64,
    /// Manifest checksum declared by Platform Core.
    pub checksum_sha256: String,
    /// Accepted anchor object descriptors.
    pub objects: Vec<AnchorArtifactObject>,
    /// Rejected row object descriptors.
    pub rejected_objects: Vec<AnchorArtifactRejectObject>,
}

/// Accepted anchor JSONL object descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorArtifactObject {
    /// Shard identity.
    pub shard_id: String,
    /// Platform Core source object key.
    pub source_object_key: String,
    /// Platform Core anchor object key.
    pub artifact_object_key: String,
    /// Number of JSONL rows in the object.
    pub row_count: u64,
    /// Object byte size declared by Platform Core.
    pub size_bytes: u64,
    /// Object checksum declared by Platform Core.
    pub checksum_sha256: String,
}

/// Rejected anchor JSONL object descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnchorArtifactRejectObject {
    /// Shard identity.
    pub shard_id: String,
    /// Platform Core source object key.
    pub source_object_key: String,
    /// Platform Core rejected object key.
    pub rejected_object_key: String,
    /// Number of JSONL rows in the object.
    pub row_count: u64,
    /// Object byte size declared by Platform Core.
    pub size_bytes: u64,
    /// Object checksum declared by Platform Core.
    pub checksum_sha256: String,
}

/// Platform Core anchor artifact import contract error.
#[derive(Debug, Error)]
pub enum AnchorImportError {
    /// JSON syntax or shape error.
    #[error("invalid anchor artifact json: {0}")]
    Json(#[from] serde_json::Error),
    /// Contract field mismatch.
    #[error("anchor artifact contract mismatch: {0}")]
    Contract(&'static str),
}

impl PartialEq for AnchorImportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Contract(left), Self::Contract(right)) => left == right,
            (Self::Json(_), Self::Json(_)) => true,
            _ => false,
        }
    }
}

impl Eq for AnchorImportError {}

#[derive(Debug, Deserialize)]
struct AnchorArtifactEntry {
    schema_version: String,
    pnu: String,
    anchor_lng: f64,
    anchor_lat: f64,
    anchor_srid: String,
    algorithm: String,
    algorithm_version: String,
    source_geometry_checksum_sha256: String,
}

#[derive(Debug, Deserialize)]
struct RawAnchorArtifactManifest {
    schema_version: String,
    artifact_version: String,
    source_snapshot_id: String,
    source_table: String,
    source_srid: String,
    anchor_srid: String,
    algorithm: String,
    algorithm_version: String,
    artifact_object_count: u64,
    artifact_row_count: u64,
    rejected_object_count: u64,
    rejected_row_count: u64,
    checksum_sha256: String,
    objects: Vec<AnchorArtifactObject>,
    rejected_objects: Vec<AnchorArtifactRejectObject>,
}

impl<'de> Deserialize<'de> for AnchorArtifactObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            shard_id: String,
            source_object_key: String,
            artifact_object_key: String,
            row_count: u64,
            size_bytes: u64,
            checksum_sha256: String,
        }

        let raw = Raw::deserialize(deserializer)?;
        Ok(Self {
            shard_id: raw.shard_id,
            source_object_key: raw.source_object_key,
            artifact_object_key: raw.artifact_object_key,
            row_count: raw.row_count,
            size_bytes: raw.size_bytes,
            checksum_sha256: raw.checksum_sha256,
        })
    }
}

impl<'de> Deserialize<'de> for AnchorArtifactRejectObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Raw {
            shard_id: String,
            source_object_key: String,
            rejected_object_key: String,
            row_count: u64,
            size_bytes: u64,
            checksum_sha256: String,
        }

        let raw = Raw::deserialize(deserializer)?;
        Ok(Self {
            shard_id: raw.shard_id,
            source_object_key: raw.source_object_key,
            rejected_object_key: raw.rejected_object_key,
            row_count: raw.row_count,
            size_bytes: raw.size_bytes,
            checksum_sha256: raw.checksum_sha256,
        })
    }
}

/// Parse a Platform Core anchor artifact manifest.
///
/// # Errors
///
/// Returns [`AnchorImportError`] when the manifest JSON or accounting violates
/// the published Platform Core artifact contract.
pub fn parse_anchor_manifest(value: &str) -> Result<AnchorArtifactManifest, AnchorImportError> {
    let manifest: RawAnchorArtifactManifest = serde_json::from_str(value)?;
    validate_manifest_schema_version(&manifest.schema_version)?;
    validate_source_table(&manifest.source_table)?;
    validate_source_srid(&manifest.source_srid)?;
    validate_anchor_srid(&manifest.anchor_srid)?;
    validate_algorithm(&manifest.algorithm)?;
    validate_algorithm_version(&manifest.algorithm_version)?;
    validate_sha256(&manifest.checksum_sha256)?;
    validate_manifest_objects(&manifest)?;

    Ok(AnchorArtifactManifest {
        artifact_version: manifest.artifact_version,
        source_snapshot_id: manifest.source_snapshot_id,
        source_table: manifest.source_table,
        source_srid: manifest.source_srid,
        anchor_srid: manifest.anchor_srid,
        algorithm: manifest.algorithm,
        algorithm_version: manifest.algorithm_version,
        artifact_row_count: manifest.artifact_row_count,
        rejected_row_count: manifest.rejected_row_count,
        checksum_sha256: manifest.checksum_sha256,
        objects: manifest.objects,
        rejected_objects: manifest.rejected_objects,
    })
}

/// Parse one anchor JSONL object and verify the expected row count.
///
/// # Errors
///
/// Returns [`AnchorImportError`] when any line violates the entry contract or
/// the parsed row count differs from the manifest object descriptor.
pub fn parse_anchor_rows(
    value: &str,
    expected_row_count: u64,
) -> Result<Vec<AnchorArtifactRow>, AnchorImportError> {
    let mut rows = Vec::new();
    for line in value.lines() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(parse_anchor_entry(line)?);
    }

    if u64::try_from(rows.len()).map_err(|_| AnchorImportError::Contract("object row_count"))?
        != expected_row_count
    {
        return Err(AnchorImportError::Contract("object row_count"));
    }

    Ok(rows)
}

/// Parse one Platform Core anchor artifact JSONL entry.
///
/// # Errors
///
/// Returns [`AnchorImportError`] when the line is invalid JSON or violates the
/// Platform Core anchor artifact contract consumed by Gongzzang.
pub fn parse_anchor_entry(line: &str) -> Result<AnchorArtifactRow, AnchorImportError> {
    let entry: AnchorArtifactEntry = serde_json::from_str(line)?;

    validate_schema_version(&entry.schema_version)?;
    validate_pnu(&entry.pnu)?;
    validate_lng_lat(entry.anchor_lng, entry.anchor_lat)?;
    validate_anchor_srid(&entry.anchor_srid)?;
    validate_algorithm(&entry.algorithm)?;
    validate_algorithm_version(&entry.algorithm_version)?;
    validate_sha256(&entry.source_geometry_checksum_sha256)?;

    Ok(AnchorArtifactRow {
        pnu: entry.pnu,
        anchor_lng: entry.anchor_lng,
        anchor_lat: entry.anchor_lat,
        algorithm: entry.algorithm,
        algorithm_version: entry.algorithm_version,
        source_geometry_checksum_sha256: entry.source_geometry_checksum_sha256,
    })
}

fn validate_schema_version(value: &str) -> Result<(), AnchorImportError> {
    if value == ENTRY_SCHEMA_VERSION {
        return Ok(());
    }
    Err(AnchorImportError::Contract("entry schema_version"))
}

fn validate_manifest_schema_version(value: &str) -> Result<(), AnchorImportError> {
    if value == MANIFEST_SCHEMA_VERSION {
        return Ok(());
    }
    Err(AnchorImportError::Contract("manifest schema_version"))
}

fn validate_source_table(value: &str) -> Result<(), AnchorImportError> {
    if value == "silver.parcel_boundaries" {
        return Ok(());
    }
    Err(AnchorImportError::Contract("manifest source_table"))
}

fn validate_source_srid(value: &str) -> Result<(), AnchorImportError> {
    if value == "EPSG:4326" {
        return Ok(());
    }
    Err(AnchorImportError::Contract(
        "manifest source_srid must be EPSG:4326",
    ))
}

fn validate_pnu(value: &str) -> Result<(), AnchorImportError> {
    if value.len() == 19 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(());
    }
    Err(AnchorImportError::Contract("entry pnu"))
}

fn validate_lng_lat(anchor_lng: f64, anchor_lat: f64) -> Result<(), AnchorImportError> {
    if !anchor_lng.is_finite() || !(-180.0..=180.0).contains(&anchor_lng) {
        return Err(AnchorImportError::Contract("entry anchor_lng"));
    }
    if !anchor_lat.is_finite() || !(-90.0..=90.0).contains(&anchor_lat) {
        return Err(AnchorImportError::Contract("entry anchor_lat"));
    }
    Ok(())
}

fn validate_anchor_srid(value: &str) -> Result<(), AnchorImportError> {
    if value == "EPSG:4326" {
        return Ok(());
    }
    Err(AnchorImportError::Contract(
        "entry anchor_srid must be EPSG:4326",
    ))
}

fn validate_algorithm(value: &str) -> Result<(), AnchorImportError> {
    if value.len() <= 64
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Ok(());
    }
    Err(AnchorImportError::Contract("entry algorithm"))
}

fn validate_algorithm_version(value: &str) -> Result<(), AnchorImportError> {
    if (2..=128).contains(&value.len())
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b':' | b'-')
        })
    {
        return Ok(());
    }
    Err(AnchorImportError::Contract("entry algorithm_version"))
}

fn validate_sha256(value: &str) -> Result<(), AnchorImportError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Ok(());
    }
    Err(AnchorImportError::Contract(
        "entry source_geometry_checksum_sha256",
    ))
}

fn validate_manifest_objects(
    manifest: &RawAnchorArtifactManifest,
) -> Result<(), AnchorImportError> {
    if u64::try_from(manifest.objects.len())
        .map_err(|_| AnchorImportError::Contract("manifest artifact_object_count"))?
        != manifest.artifact_object_count
    {
        return Err(AnchorImportError::Contract(
            "manifest artifact_object_count",
        ));
    }
    if u64::try_from(manifest.rejected_objects.len())
        .map_err(|_| AnchorImportError::Contract("manifest rejected_object_count"))?
        != manifest.rejected_object_count
    {
        return Err(AnchorImportError::Contract(
            "manifest rejected_object_count",
        ));
    }

    let artifact_rows = manifest.objects.iter().try_fold(0_u64, |acc, object| {
        validate_object_descriptor(object)?;
        acc.checked_add(object.row_count)
            .ok_or(AnchorImportError::Contract("manifest artifact_row_count"))
    })?;
    if artifact_rows != manifest.artifact_row_count {
        return Err(AnchorImportError::Contract("manifest artifact_row_count"));
    }

    let rejected_rows = manifest
        .rejected_objects
        .iter()
        .try_fold(0_u64, |acc, object| {
            validate_reject_object_descriptor(object)?;
            acc.checked_add(object.row_count)
                .ok_or(AnchorImportError::Contract("manifest rejected_row_count"))
        })?;
    if rejected_rows != manifest.rejected_row_count {
        return Err(AnchorImportError::Contract("manifest rejected_row_count"));
    }

    Ok(())
}

fn validate_object_descriptor(object: &AnchorArtifactObject) -> Result<(), AnchorImportError> {
    validate_object_key(&object.source_object_key, "object source_object_key")?;
    validate_object_key(&object.artifact_object_key, "object artifact_object_key")?;
    validate_sha256(&object.checksum_sha256)
        .map_err(|_| AnchorImportError::Contract("object checksum_sha256"))?;
    Ok(())
}

fn validate_reject_object_descriptor(
    object: &AnchorArtifactRejectObject,
) -> Result<(), AnchorImportError> {
    validate_object_key(
        &object.source_object_key,
        "rejected_object source_object_key",
    )?;
    validate_object_key(
        &object.rejected_object_key,
        "rejected_object rejected_object_key",
    )?;
    validate_sha256(&object.checksum_sha256)
        .map_err(|_| AnchorImportError::Contract("rejected_object checksum_sha256"))?;
    Ok(())
}

fn validate_object_key(value: &str, label: &'static str) -> Result<(), AnchorImportError> {
    if value.trim() != value || value.is_empty() || value.starts_with('/') {
        return Err(AnchorImportError::Contract(label));
    }
    if value.contains('\\') || value.contains("..") {
        return Err(AnchorImportError::Contract(label));
    }
    Ok(())
}

#[cfg(test)]
#[path = "platform_core_anchor_import/tests.rs"]
mod tests;
