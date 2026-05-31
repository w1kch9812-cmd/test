#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::{parse_anchor_entry, parse_anchor_manifest, parse_anchor_rows, AnchorImportError};

const VALID_ENTRY: &str = r#"{"schema_version":"platform-core.parcel_marker_anchor_artifact_entry.v1","pnu":"1111010100100090000","anchor_lng":126.978,"anchor_lat":37.5665,"anchor_srid":"EPSG:4326","algorithm":"polylabel","algorithm_version":"postgis-st_maximuminscribedcircle-v1","source_geometry_checksum_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#;

#[test]
fn parses_anchor_entry_into_import_row() {
    let entry = parse_anchor_entry(VALID_ENTRY).expect("entry");

    assert_eq!(entry.pnu, "1111010100100090000");
    assert!((entry.anchor_lng - 126.978).abs() < f64::EPSILON);
    assert!((entry.anchor_lat - 37.5665).abs() < f64::EPSILON);
    assert_eq!(entry.algorithm, "polylabel");
    assert_eq!(
        entry.algorithm_version,
        "postgis-st_maximuminscribedcircle-v1"
    );
    assert_eq!(entry.source_geometry_checksum_sha256, "b".repeat(64));
}

#[test]
fn rejects_wrong_entry_srid() {
    let err =
        parse_anchor_entry(&VALID_ENTRY.replace("EPSG:4326", "EPSG:3857")).expect_err("wrong srid");

    assert_eq!(
        err,
        AnchorImportError::Contract("entry anchor_srid must be EPSG:4326")
    );
}

#[test]
fn rejects_invalid_pnu_shape() {
    let err = parse_anchor_entry(&VALID_ENTRY.replace("1111010100100090000", "bad-pnu"))
        .expect_err("invalid pnu");

    assert_eq!(err, AnchorImportError::Contract("entry pnu"));
}

#[test]
fn rejects_invalid_checksum_shape() {
    let err = parse_anchor_entry(&VALID_ENTRY.replace(&"b".repeat(64), "not-a-sha256-checksum"))
        .expect_err("invalid checksum");

    assert_eq!(
        err,
        AnchorImportError::Contract("entry source_geometry_checksum_sha256")
    );
}

#[test]
fn parses_anchor_manifest_and_validates_object_accounting() {
    let manifest = parse_anchor_manifest(&format!(
        r#"{{
                "schema_version":"platform-core.parcel_marker_anchor_artifact_manifest.v1",
                "artifact_version":"0196f0b0-3e01-7000-8000-000000000002",
                "source_snapshot_id":"iceberg:parcel-boundaries-snapshot-001",
                "source_table":"silver.parcel_boundaries",
                "source_srid":"EPSG:4326",
                "anchor_srid":"EPSG:4326",
                "algorithm":"polylabel",
                "algorithm_version":"postgis-st_maximuminscribedcircle-v1",
                "artifact_object_count":1,
                "artifact_row_count":2,
                "rejected_object_count":0,
                "rejected_row_count":0,
                "checksum_sha256":"{}",
                "objects":[
                    {{
                        "shard_id":"shard-000001",
                        "source_object_key":"silver/parcel-boundaries/shard-000001.jsonl",
                        "artifact_object_key":"gold/parcel-marker-anchors/shard-000001.jsonl",
                        "row_count":2,
                        "size_bytes":512,
                        "checksum_sha256":"{}"
                    }}
                ],
                "rejected_objects":[]
            }}"#,
        "a".repeat(64),
        "b".repeat(64)
    ))
    .expect("manifest");

    assert_eq!(manifest.artifact_row_count, 2);
    assert_eq!(manifest.objects.len(), 1);
    assert_eq!(
        manifest.objects[0].artifact_object_key,
        "gold/parcel-marker-anchors/shard-000001.jsonl"
    );
}

#[test]
fn rejects_manifest_when_artifact_row_count_does_not_match_objects() {
    let err = parse_anchor_manifest(&format!(
        r#"{{
                "schema_version":"platform-core.parcel_marker_anchor_artifact_manifest.v1",
                "artifact_version":"0196f0b0-3e01-7000-8000-000000000002",
                "source_snapshot_id":"iceberg:parcel-boundaries-snapshot-001",
                "source_table":"silver.parcel_boundaries",
                "source_srid":"EPSG:4326",
                "anchor_srid":"EPSG:4326",
                "algorithm":"polylabel",
                "algorithm_version":"postgis-st_maximuminscribedcircle-v1",
                "artifact_object_count":1,
                "artifact_row_count":3,
                "rejected_object_count":0,
                "rejected_row_count":0,
                "checksum_sha256":"{}",
                "objects":[
                    {{
                        "shard_id":"shard-000001",
                        "source_object_key":"silver/parcel-boundaries/shard-000001.jsonl",
                        "artifact_object_key":"gold/parcel-marker-anchors/shard-000001.jsonl",
                        "row_count":2,
                        "size_bytes":512,
                        "checksum_sha256":"{}"
                    }}
                ],
                "rejected_objects":[]
            }}"#,
        "a".repeat(64),
        "b".repeat(64)
    ))
    .expect_err("row count mismatch");

    assert_eq!(
        err,
        AnchorImportError::Contract("manifest artifact_row_count")
    );
}

#[test]
fn parses_anchor_jsonl_rows_and_rejects_row_count_mismatch() {
    let rows =
        parse_anchor_rows(&format!("{VALID_ENTRY}\n{VALID_ENTRY}\n"), 2).expect("anchor rows");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].pnu, "1111010100100090000");

    let err =
        parse_anchor_rows(&format!("{VALID_ENTRY}\n"), 2).expect_err("anchor row count mismatch");

    assert_eq!(err, AnchorImportError::Contract("object row_count"));
}
