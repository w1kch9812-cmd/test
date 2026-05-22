#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use serde_json::json;
use sp9_base_layer_config::{R2PublicBase, Version};
use std::io::Write;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn test_config(bucket: &str) -> R2Config {
    R2Config {
        account_id: "fake-account".into(),
        access_key: "fake-access".into(),
        secret_key: "fake-secret".into(),
        bucket: bucket.into(),
        bronze_prefix: "bronze".into(),
        gold_prefix: "gold".into(),
    }
}

#[test]
fn endpoint_url_uses_account_id() {
    let cfg = test_config("any");
    assert_eq!(
        cfg.endpoint_url(),
        "https://fake-account.r2.cloudflarestorage.com"
    );
}

// P2: R2Config key layout SSOT property tests.
// 이 테스트들이 곧 "key layout 이 변경되면 컴파일러 차단" 보장.
// URL 변경 = ADR + 이 테스트 갱신 = backward-compatibility gate.

fn v(s: &str) -> Version {
    Version::new(s).expect("test version must be valid")
}

fn pub_base(s: &str) -> R2PublicBase {
    R2PublicBase::new(s).expect("test public base must be valid")
}

#[test]
fn gold_layer_prefix_layout() {
    let cfg = test_config("bucket");
    assert_eq!(
        cfg.gold_layer_prefix(&v("v3"), "parcels"),
        "gold/v3/parcels"
    );
}

#[test]
fn tilejson_key_layout() {
    let cfg = test_config("bucket");
    assert_eq!(
        cfg.tilejson_key(&v("v3"), "parcels"),
        "gold/v3/parcels.json"
    );
}

#[test]
fn manifest_key_layout() {
    let cfg = test_config("bucket");
    assert_eq!(cfg.manifest_key(), "gold/manifest.json");
}

#[test]
fn manifest_backup_key_layout() {
    let cfg = test_config("bucket");
    assert_eq!(cfg.manifest_backup_key(&v("v2")), "gold/manifest.v2.json");
}

#[test]
fn staging_spec_key_layout() {
    let cfg = test_config("bucket");
    assert_eq!(
        cfg.staging_spec_key(&v("v3"), "admin"),
        "gold/staging/v3/admin.spec.json"
    );
}

#[test]
fn tiles_url_template_with_trailing_slash() {
    let cfg = test_config("bucket");
    let url = cfg.tiles_url_template(&pub_base("https://r2.example.com/"), &v("v3"), "parcels");
    assert_eq!(
        url,
        "https://r2.example.com/gold/v3/parcels/{z}/{x}/{y}.pbf"
    );
}

#[test]
fn tiles_url_template_without_trailing_slash() {
    let cfg = test_config("bucket");
    let url = cfg.tiles_url_template(&pub_base("https://r2.example.com"), &v("v3"), "admin");
    assert_eq!(url, "https://r2.example.com/gold/v3/admin/{z}/{x}/{y}.pbf");
}

#[test]
fn key_helpers_round_trip_coverage() {
    // 모든 helper 가 gold_prefix 를 일관되게 prefix 로 사용하는지 확인.
    // gold_prefix 변경 시 모든 key 가 한꺼번에 변경됨을 보장.
    let cfg = R2Config {
        account_id: "fake".into(),
        access_key: "fake".into(),
        secret_key: "fake".into(),
        bucket: "bucket".into(),
        bronze_prefix: "bronze".into(),
        gold_prefix: "custom-gold".into(),
    };
    let ver = v("v1");
    let prefix = cfg.gold_layer_prefix(&ver, "parcels");
    assert!(
        prefix.starts_with("custom-gold/"),
        "gold_prefix must be respected"
    );
    assert!(
        cfg.manifest_key().starts_with("custom-gold/"),
        "manifest must use gold_prefix"
    );
    assert!(
        cfg.staging_spec_key(&ver, "parcels")
            .starts_with("custom-gold/"),
        "staging key must use gold_prefix"
    );
}

#[tokio::test]
async fn put_object_file_sends_body_and_headers() {
    let server = MockServer::start().await;

    // path-style: /bucket/key  (force_path_style = true)
    Mock::given(method("PUT"))
        .and(path("/test-bucket/bronze/2026-05/parcel.shp.zip"))
        .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"deadbeef\""))
        .expect(1)
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(b"PK\x03\x04 fake zip body")
        .expect("write tmp");

    uploader
        .put_object_file(
            "bronze/2026-05/parcel.shp.zip",
            tmp.path(),
            "application/zip",
        )
        .await
        .expect("upload");

    // wiremock 의 `expect(1)` 가 drop 시 검증 → 통과하면 PUT 1회 받음.
}

/// P0 (Codex Round 3): `concurrency: 0` 은 fail-fast — `buffer_unordered(0)`
/// 가 stream 정지 시키는 silent failure 차단.
#[tokio::test]
async fn put_directory_rejects_zero_concurrency() {
    let server = MockServer::start().await;
    let cfg = test_config("test-bucket");
    let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
    let tmp = tempfile::tempdir().expect("tempdir");
    let err = uploader
        .put_directory(tmp.path(), "gold/v1/parcels", 0)
        .await
        .expect_err("concurrency=0 must reject");
    assert!(matches!(err, UploadError::InvalidConcurrency));
}

/// 회귀 테스트 — Codex stop-time review 발견 (Round 2 hotfix):
/// breaker wrap 이 first publish promote 를 깨뜨림. `try_get_object_bytes` 가
/// `NoSuchKey` 를 `Ok(None)` 으로 흡수해야 (1) typed `Option` 분기 + (2) breaker
/// failure window 누적 0.
#[tokio::test]
async fn try_get_object_bytes_returns_none_on_no_such_key() {
    let server = MockServer::start().await;
    // S3 NoSuchKey 응답 — 정확한 wire format (status 404 + AWS XML body).
    Mock::given(method("GET"))
        .and(path("/test-bucket/gold/manifest.json"))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message>\
                 <Key>gold/manifest.json</Key><RequestId>test</RequestId></Error>",
        ))
        .expect(1)
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

    let result = uploader
        .try_get_object_bytes("gold/manifest.json")
        .await
        .expect("NoSuchKey must be Ok(None), not Err");
    assert!(result.is_none(), "expected None for NoSuchKey, got Some");
}

/// 회귀 테스트 — `NoSuchKey` 가 breaker failure 로 카운트되지 않아야 함
/// (반복되는 expected miss 가 circuit open 트리거하면 first-publish 가 영구 차단됨).
#[tokio::test]
async fn try_get_object_bytes_no_such_key_does_not_open_breaker() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404).set_body_string(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <Error><Code>NoSuchKey</Code><Message>not found</Message></Error>",
        ))
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    let mut uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
    // 매우 낮은 threshold — 만약 NoSuchKey 가 실패로 카운트되면 1번에 open.
    uploader.policy = circuit_breaker::Policy {
        timeout_ms: 1_000,
        max_retries: 0,
        retry_base_ms: 1,
        open_threshold: 1,
        open_window_ms: 60_000,
        open_cooldown_ms: 60_000,
    };

    // 5번 연속 NoSuchKey — open 안 되어야 함.
    for _ in 0..5 {
        let r = uploader.try_get_object_bytes("missing.json").await;
        assert!(matches!(r, Ok(None)), "expected Ok(None), got: {r:?}");
    }
}

#[tokio::test]
async fn put_object_json_serializes_pretty() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .and(path("/test-bucket/gold/manifest.json"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

    let payload = json!({"current_version": "v1", "artifacts": []});
    uploader
        .put_object_json("gold/manifest.json", &payload, "no-cache, max-age=0")
        .await
        .expect("upload");
}

#[tokio::test]
async fn put_object_file_propagates_5xx() {
    let server = MockServer::start().await;

    Mock::given(method("PUT"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(
                "<Error><Code>InternalError</Code><Message>oops</Message></Error>",
            ),
        )
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(b"x").expect("write");

    let err = uploader
        .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
        .await
        .expect_err("should fail");
    // T2 — breaker wrap: inner error 가 MaxRetriesExceeded 로 전파 → `Breaker` variant.
    // op 식별자 + 원본 stderr (`InternalError`) 가 detail 에 보존됨을 검증.
    match err {
        UploadError::Breaker { op, detail } => {
            assert_eq!(op, "r2.put_object_file");
            assert!(
                detail.contains("InternalError") || detail.contains("put_object"),
                "breaker detail must preserve inner PutObject context: {detail}"
            );
        }
        other => panic!("expected Breaker variant, got: {other:?}"),
    }
}

#[tokio::test]
async fn breaker_opens_after_repeated_500_failures() {
    // T2 회귀 — circuit-breaker 가 R2 systemic 장애 시 fast-fail 를 보장.
    // open_threshold=5 (`Policy::r2_default`) 라 max_retries 1 + 의도적 5xx 가
    // 누적되어 open 으로 전이.
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(
            ResponseTemplate::new(500).set_body_string(
                "<Error><Code>InternalError</Code><Message>oops</Message></Error>",
            ),
        )
        .mount(&server)
        .await;

    let cfg = test_config("test-bucket");
    // production-like policy 와 비슷하지만 timeout 짧게 + cooldown 짧게.
    let mut uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
    uploader.policy = circuit_breaker::Policy {
        timeout_ms: 1_000,
        max_retries: 0,
        retry_base_ms: 1,
        open_threshold: 3,
        open_window_ms: 60_000,
        open_cooldown_ms: 60_000,
    };

    let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
    tmp.write_all(b"x").expect("write");

    // 3회 실패 누적 → 4회째 호출은 즉시 Open 으로 거부.
    for _ in 0..3 {
        let _ = uploader
            .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
            .await;
    }
    let err = uploader
        .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
        .await
        .expect_err("breaker should be open");
    match err {
        UploadError::Breaker { detail, .. } => {
            assert!(
                detail.contains("circuit open"),
                "expected open-state detail, got: {detail}"
            );
        }
        other => panic!("expected Breaker(Open), got: {other:?}"),
    }
}
