#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::await_holding_lock,  // env-mutating tests 는 process-global 이라 lock-held await 필요
)]

use super::{cloudflare_purge, ArtifactSpec, CdnPurgeOutcome, PromoteError};
use crate::test_support::GLOBAL_ENV_LOCK as ENV_LOCK;

fn clear_cdn_env() {
    for k in [
        "CLOUDFLARE_API_TOKEN",
        "CLOUDFLARE_ZONE_ID",
        "R2_PUBLIC_URL_BASE",
        "ETL_ENVIRONMENT",
    ] {
        std::env::remove_var(k);
    }
}

fn fixture_prefix() -> String {
    ["gold", "v3", "parcels"].join("/")
}

/// Round 4 #5 — CDN config 누락 + ETL_ENVIRONMENT != production = `SkippedDevMode`.
#[tokio::test]
async fn cloudflare_purge_skips_silently_in_dev_mode() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "local");
    let outcome = cloudflare_purge("gold/manifest.json")
        .await
        .expect("dev mode skip");
    assert_eq!(outcome, CdnPurgeOutcome::SkippedDevMode);
    clear_cdn_env();
}

/// Round 4 #5 — CDN config 누락 + ETL_ENVIRONMENT=production = fail-fast (silent path 0).
#[tokio::test]
async fn cloudflare_purge_fails_fast_in_production_when_config_missing() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "production");
    let err = cloudflare_purge("gold/manifest.json")
        .await
        .expect_err("production mode missing-config = fail-fast");
    match err {
        PromoteError::CdnPurgeMissingConfig { missing } => {
            assert!(
                missing.contains("CLOUDFLARE_API_TOKEN"),
                "missing detail must include token: {missing}"
            );
            assert!(missing.contains("CLOUDFLARE_ZONE_ID"));
            assert!(missing.contains("R2_PUBLIC_URL_BASE"));
        }
        other => panic!("expected CdnPurgeMissingConfig, got: {other:?}"),
    }
    clear_cdn_env();
}

/// Round 5 P1 — cleanup-manifest-backups subcommand 의 keep=0 거부.
/// (실수로 전체 backup chain 삭제 차단.)
#[tokio::test]
async fn cleanup_rejects_zero_keep() {
    use crate::r2_upload::R2Config;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    // ListObjects mock — keep=0 abort 가 list 전에 발생해야 하므로 mock 실제 호출 X.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            "<?xml version=\"1.0\"?><ListBucketResult><Name>x</Name></ListBucketResult>",
        ))
        .mount(&server)
        .await;

    let cfg = R2Config {
        account_id: "fake".into(),
        access_key: "fake".into(),
        secret_key: "fake".into(),
        bucket: "test-bucket".into(),
        bronze_prefix: "bronze".into(),
        gold_prefix: "gold".into(),
    };
    let uploader = crate::r2_upload::R2Uploader::with_endpoint_override(cfg, server.uri());
    let err = super::cleanup_manifest_backups(&uploader, 0)
        .await
        .expect_err("keep=0 must be rejected");
    assert!(matches!(err, PromoteError::InvalidCleanupKeep));
}

/// Round 5 P0 — promote 의 pre-flight 가 production env 에서 CDN config 누락을
/// *manifest 만지기 전* 차단. 이전 path 는 publish 후 step 5 에서 검출했음.
#[test]
fn preflight_blocks_promotion_in_production_when_cdn_missing() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "production");
    let err = super::preflight_cdn_config()
        .expect_err("production + missing CDN config = pre-flight abort");
    match err {
        PromoteError::CdnPurgeMissingConfig { missing } => {
            assert!(missing.contains("CLOUDFLARE_API_TOKEN"), "{missing}");
            assert!(missing.contains("CLOUDFLARE_ZONE_ID"), "{missing}");
            assert!(missing.contains("R2_PUBLIC_URL_BASE"), "{missing}");
        }
        other => panic!("expected CdnPurgeMissingConfig, got {other:?}"),
    }
    clear_cdn_env();
}

/// Round 5 P0 — dev/staging env 에서는 pre-flight 가 silent OK (config 누락 허용,
/// step 5 의 `SkippedDevMode` 가 자연 path).
#[test]
fn preflight_passes_in_dev_mode_even_when_cdn_missing() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "local");
    super::preflight_cdn_config().expect("dev mode pre-flight = silent OK");
    clear_cdn_env();
}

/// Round 5 P0 — production env 에 모든 config 가 set 되면 pre-flight 통과.
#[test]
fn preflight_passes_in_production_when_cdn_config_complete() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "production");
    std::env::set_var("CLOUDFLARE_API_TOKEN", "fake-token");
    std::env::set_var("CLOUDFLARE_ZONE_ID", "fake-zone");
    std::env::set_var("R2_PUBLIC_URL_BASE", "https://r2.example.com");
    super::preflight_cdn_config().expect("complete config = pre-flight OK");
    clear_cdn_env();
}

/// Round 4 #5 — production mode 인데 *부분* config (1개만 누락) → 같은 fail-fast.
#[tokio::test]
async fn cloudflare_purge_fails_fast_in_production_when_partial_config() {
    let _guard = ENV_LOCK.lock().expect("env mutex");
    clear_cdn_env();
    std::env::set_var("ETL_ENVIRONMENT", "production");
    std::env::set_var("CLOUDFLARE_API_TOKEN", "fake-token");
    std::env::set_var("CLOUDFLARE_ZONE_ID", "fake-zone");
    // R2_PUBLIC_URL_BASE 만 누락.
    let err = cloudflare_purge("gold/manifest.json")
        .await
        .expect_err("partial config = fail-fast");
    match err {
        PromoteError::CdnPurgeMissingConfig { missing } => {
            assert!(missing.contains("R2_PUBLIC_URL_BASE"), "{missing}");
            assert!(!missing.contains("CLOUDFLARE_API_TOKEN"), "{missing}");
        }
        other => panic!("expected CdnPurgeMissingConfig, got: {other:?}"),
    }
    clear_cdn_env();
}

/// Round 4 #6 — `PromoteError::CdnPurge` 의 `body_read_error` 필드가 typed.
/// body read 가 성공했으면 None, 실패했으면 Some(에러 메시지).
#[test]
fn cdn_purge_error_body_read_error_field_default_is_none() {
    let err = PromoteError::CdnPurge {
        status: 502,
        body: "Bad Gateway".into(),
        body_read_error: None,
    };
    let display = format!("{err}");
    assert!(display.contains("body=Bad Gateway"), "{display}");
    assert!(display.contains("body_read_error=None"), "{display}");
}

#[test]
fn cdn_purge_error_preserves_body_read_error() {
    let err = PromoteError::CdnPurge {
        status: 503,
        body: String::new(),
        body_read_error: Some("io: connection reset".into()),
    };
    let display = format!("{err}");
    assert!(
        display.contains("connection reset"),
        "body_read_error must propagate: {display}"
    );
}

/// P0 typed gate (Codex Round 3 발견 fix): staging spec round-trip.
/// `write_staging_spec` 가 직렬화한 JSON 이 `ArtifactSpec` 으로 1:1 round-trip.
#[test]
fn artifact_spec_round_trips_typed() {
    use super::BuildLineage;
    use chrono::TimeZone;
    let key_prefix = fixture_prefix();
    let spec = ArtifactSpec {
        key_prefix,
        pmtiles_bytes: 1_234_567,
        pmtiles_sha256: "abc123".into(),
        row_count: Some(1_400_000_000),
        flat_tile_count: 800_000,
        flat_tiles_total_bytes: 8_000_000_000,
        lineage: BuildLineage {
            tippecanoe_version: "2.79.0".into(),
            git_sha: "deadbeef".into(),
            built_at: chrono::Utc.with_ymd_and_hms(2026, 5, 8, 12, 0, 0).unwrap(),
            bronze_inputs: vec![],
            source_srs: "EPSG:5186".into(),
            layer_name: "parcels".into(),
            build_environment: "dev".into(),
            source_license: None,
            source_url: None,
            correlation_id: None,
        },
    };
    let json = serde_json::to_vec_pretty(&spec).expect("serialize");
    let back: ArtifactSpec = serde_json::from_slice(&json).expect("deserialize");
    assert_eq!(back.key_prefix, spec.key_prefix);
    assert_eq!(back.pmtiles_bytes, spec.pmtiles_bytes);
    assert_eq!(back.flat_tile_count, spec.flat_tile_count);
    assert_eq!(back.row_count, spec.row_count);
    assert_eq!(back.lineage.source_srs, "EPSG:5186");
}

/// P0 typed gate: 누락 필드는 `unwrap_or_default()` 로 통과 안 되고 거부됨.
/// `serde_json::Value` + `as_u64().unwrap_or(0)` 의 trick 이 이전엔 silent 0 으로 통과시킴.
#[test]
fn artifact_spec_rejects_missing_required_field() {
    // `pmtiles_bytes` 누락 — 이전 path 에선 `unwrap_or(0)` 로 0 반환.
    let bad_json = serde_json::json!({
        "key_prefix": fixture_prefix(),
        "pmtiles_sha256": "abc",
        "row_count": null,
        "flat_tile_count": 100,
        "flat_tiles_total_bytes": 200,
        "lineage": {
            "tippecanoe_version": "2.79.0",
            "git_sha": "x",
            "built_at": "2026-05-08T00:00:00Z",
            "bronze_inputs": [],
            "source_srs": "EPSG:5186",
            "layer_name": "parcels",
            "build_environment": "dev",
        }
    });
    let result: Result<ArtifactSpec, _> = serde_json::from_value(bad_json);
    assert!(
        result.is_err(),
        "missing pmtiles_bytes must be rejected by serde, but got: {result:?}"
    );
}

/// P0 typed gate: 잘못된 타입 (string vs u64) 도 거부.
#[test]
fn artifact_spec_rejects_wrong_type() {
    let bad_json = serde_json::json!({
        "key_prefix": fixture_prefix(),
        "pmtiles_bytes": "not-a-number", // 잘못된 타입
        "pmtiles_sha256": "abc",
        "row_count": null,
        "flat_tile_count": 100,
        "flat_tiles_total_bytes": 200,
        "lineage": {
            "tippecanoe_version": "2.79.0",
            "git_sha": "x",
            "built_at": "2026-05-08T00:00:00Z",
            "bronze_inputs": [],
            "source_srs": "EPSG:5186",
            "layer_name": "parcels",
            "build_environment": "dev",
        }
    });
    let result: Result<ArtifactSpec, _> = serde_json::from_value(bad_json);
    assert!(result.is_err(), "wrong-type pmtiles_bytes must be rejected");
}

#[test]
fn staging_key_format() {
    use crate::r2_upload::R2Config;
    use sp9_base_layer_config::Version;
    let cfg = R2Config {
        account_id: "fake".into(),
        access_key: "fake".into(),
        secret_key: "fake".into(),
        bucket: "bucket".into(),
        bronze_prefix: "bronze".into(),
        gold_prefix: "gold".into(),
    };
    let v = Version::new("v3").expect("test version");
    assert_eq!(
        cfg.staging_spec_key(&v, "parcels"),
        "gold/staging/v3/parcels.spec.json"
    );
}
