#![allow(clippy::expect_used, clippy::panic)]

use chrono::{DateTime, Utc};

use super::config::{import_source_from_env_values, ImportSource, ImportSourceEnvValues};
use super::error::AnchorImporterError;
use super::lock::event_import_lock_key;
use super::source::{event_artifact_config_from_payload, resolve_artifact_object_url};
use super::util::{truncate_failure_reason, verify_sha256};
use super::{DEFAULT_BATCH_LIMIT, MAX_FAILURE_REASON_LEN};

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
    let config =
        import_source_from_env_values(ImportSourceEnvValues::default()).expect("batch source");

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
        &reqwest::Url::parse("https://platform-core.example.com/artifacts/anchors/manifest.json")
            .expect("fixture manifest URL must be valid"),
        "gold/parcel-marker-anchors/shard-000001.jsonl",
    )
    .expect("object url");

    assert_eq!(
        url.as_str(),
        "https://platform-core.example.com/artifacts/anchors/gold/parcel-marker-anchors/shard-000001.jsonl"
    );
}
