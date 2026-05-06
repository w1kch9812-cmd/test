//! V-World HTTP 통합 테스트 — wiremock 으로 fake V-World server.
//!
//! Fixture 출처: 모두 `tests/fixtures/real_*.json` (실 V-World 호출 캡처).
//! Hand-crafted fixture는 root cause R1 — 절대 추가 금지 ([ADR 0015](../../../docs/adr/0015-v-world-acl-rearchitecture.md)).
//!
//! 6 시나리오:
//! 1. `fetch_by_pnu` happy path — 200 + real OK fixture → Some(Parcel)
//! 2. `fetch_by_pnu` empty featureCollection → Ok(None) (real NOT_FOUND fixture)
//! 3. `fetch_by_pnu` 5xx 재시도 후 모두 실패 → `ReaderError::Fetch`
//! 4. `fetch_by_pnu` malformed JSON → `ReaderError::Parse`
//! 5. `fetch_by_pnu` V-World ERROR envelope → `ReaderError::Parse` (코드 보존)
//! 6. `fetch_by_pnu` circuit open after threshold failures
//! 7. `fetch_markers_in_bbox` returns deferred error (honest failure)

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic, clippy::doc_markdown)]

use std::path::PathBuf;
use std::sync::Arc;

use circuit_breaker::Policy;
use parcel_domain::errors::ReaderError;
use parcel_domain::reader::ParcelReader;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::pnu::Pnu;
use vworld_client::{NoOpRawCapture, VWorldClient, VWorldConfig, VWorldParcelReader};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn load_fixture(name: &str) -> serde_json::Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let raw = std::fs::read_to_string(&p).expect("fixture exists");
    serde_json::from_str(&raw).expect("valid JSON")
}

/// 통합 테스트용 빠른 정책 — 짧은 timeout, retry 1, 빠른 backoff.
const fn fast_policy() -> Policy {
    Policy {
        timeout_ms: 2_000,
        max_retries: 1,
        retry_base_ms: 10,
        open_threshold: 3,
        open_window_ms: 60_000,
        open_cooldown_ms: 60_000,
    }
}

fn build_reader(server: &MockServer, policy: Policy) -> VWorldParcelReader {
    let config = VWorldConfig {
        api_key: "test-key".to_owned(),
        domain: "localhost".to_owned(),
        base_url: server.uri(),
    };
    let client = Arc::new(VWorldClient::with_policy(config, policy));
    VWorldParcelReader::new(client, Arc::new(NoOpRawCapture::new()))
}

#[tokio::test]
async fn fetch_by_pnu_happy_path_real_fixture() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(load_fixture("real_parcel_boundary_gangnam_yeoksam_737.json")),
        )
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1168010100107370000").unwrap();
    let parcel = reader
        .fetch_by_pnu(&pnu)
        .await
        .expect("ok")
        .expect("Some parcel");

    assert_eq!(parcel.pnu.as_str(), "1168010100107370000");
    // Real fixture 의 jiga = 67_300_000.
    assert_eq!(
        parcel.official_land_price_per_m2.unwrap().as_i64(),
        67_300_000
    );
    // 본 레이어가 미제공인 필드 invariants:
    assert!(parcel.area.is_none());
    assert!(parcel.zoning.is_none());
    // MultiPolygon geometry.
    assert_eq!(parcel.geom.polygon_count(), 1);
}

#[tokio::test]
async fn fetch_by_pnu_not_found_returns_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(load_fixture("real_parcel_boundary_not_found.json")),
        )
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("9999999999999999999").unwrap();
    let result = reader.fetch_by_pnu(&pnu).await.expect("ok");
    assert!(result.is_none());
}

#[tokio::test]
async fn fetch_by_pnu_5xx_returns_fetch_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1111010100100010000").unwrap();
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    assert!(matches!(err, ReaderError::Fetch(_)));
}

#[tokio::test]
async fn fetch_by_pnu_malformed_response_returns_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"unexpected": "shape"})),
        )
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1111010100100010000").unwrap();
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    assert!(matches!(err, ReaderError::Parse(_)));
}

#[tokio::test]
async fn fetch_by_pnu_vworld_error_envelope_propagates() {
    // Real fixture 이지만 잘못된 layer/attrFilter 조합 (INVALID_RANGE) — 에러 envelope.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(load_fixture("real_error_invalid_range.json")),
        )
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1168010100107370000").unwrap();
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    match err {
        ReaderError::Parse(msg) => {
            assert!(msg.contains("INVALID_RANGE"), "msg: {msg}");
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[tokio::test]
async fn fetch_by_pnu_circuit_opens_after_threshold() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let policy = Policy {
        max_retries: 1,
        open_threshold: 3,
        ..fast_policy()
    };
    let reader = build_reader(&server, policy);
    let pnu = Pnu::try_new("1111010100100010000").unwrap();

    let _ = reader.fetch_by_pnu(&pnu).await;
    let _ = reader.fetch_by_pnu(&pnu).await;
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    match err {
        ReaderError::Fetch(msg) => assert!(msg.contains("circuit open"), "msg: {msg}"),
        other => panic!("expected Fetch(circuit open), got {other:?}"),
    }
}

#[tokio::test]
async fn fetch_markers_in_bbox_returns_deferred_error() {
    let server = MockServer::start().await;
    let reader = build_reader(&server, fast_policy());
    let bbox = BoundingBox::try_new_wgs84(126.9, 37.4, 127.1, 37.6).unwrap();
    let err = reader.fetch_markers_in_bbox(&bbox).await.unwrap_err();
    match err {
        ReaderError::Fetch(msg) => assert!(msg.contains("SP4-iii"), "msg: {msg}"),
        other => panic!("expected Fetch(SP4-iii), got {other:?}"),
    }
}
