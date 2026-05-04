//! V-World HTTP 통합 테스트 (SP4-ii) — wiremock 으로 fake V-World server.
//!
//! 6 시나리오:
//! 1. `fetch_by_pnu` happy path — 200 + 유효 JSON → Some(Parcel)
//! 2. `fetch_by_pnu` empty featureCollection → Ok(None) (PNU 미존재)
//! 3. `fetch_by_pnu` 5xx 재시도 후 모두 실패 → `ReaderError::Fetch`
//! 4. `fetch_by_pnu` malformed JSON → `ReaderError::Parse`
//! 5. `fetch_by_pnu` circuit open after threshold failures
//! 6. `fetch_markers_in_bbox` returns deferred error (honest failure)

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use circuit_breaker::Policy;
use parcel_domain::errors::ReaderError;
use parcel_domain::reader::ParcelReader;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::pnu::Pnu;
use vworld_client::{NoOpRawCapture, VWorldClient, VWorldConfig, VWorldParcelReader};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn sample_response() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "result": {
                "featureCollection": {
                    "features": [
                        {
                            "geometry": {
                                "type": "Polygon",
                                "coordinates": [[
                                    [126.97, 37.56],
                                    [126.98, 37.56],
                                    [126.98, 37.57],
                                    [126.97, 37.57],
                                    [126.97, 37.56]
                                ]]
                            },
                            "properties": {
                                "pnu": "1111010100100010000",
                                "addr": "서울특별시 종로구 청운동 1-1",
                                "lndcgr_nm": "공장용지",
                                "lndpcl_ar": 1500.0,
                                "uq_nm": "일반공업지역"
                            }
                        }
                    ]
                }
            }
        }
    })
}

fn empty_response() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "result": { "featureCollection": { "features": [] } }
        }
    })
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
async fn fetch_by_pnu_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_response()))
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1111010100100010000").unwrap();
    let parcel = reader
        .fetch_by_pnu(&pnu)
        .await
        .expect("ok")
        .expect("Some parcel");

    assert_eq!(parcel.pnu.as_str(), "1111010100100010000");
    assert!((parcel.area.as_f64() - 1500.0).abs() < 0.01);
}

#[tokio::test]
async fn fetch_by_pnu_empty_returns_none() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(ResponseTemplate::new(200).set_body_json(empty_response()))
        .mount(&server)
        .await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new("1111010100100010000").unwrap();
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
async fn fetch_by_pnu_circuit_opens_after_threshold() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/req/data"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    // threshold 3 — 호출 1번당 retry 포함 2번 실패 기록 → 2번 호출이면 open
    let policy = Policy {
        max_retries: 1,
        open_threshold: 3,
        ..fast_policy()
    };
    let reader = build_reader(&server, policy);
    let pnu = Pnu::try_new("1111010100100010000").unwrap();

    // 첫 호출 — 503 ×2 → recent_failures = 2
    let _ = reader.fetch_by_pnu(&pnu).await;
    // 둘째 호출 — 503 ×2 → recent_failures = 4 → Open 으로 전이 후 즉시 차단
    let _ = reader.fetch_by_pnu(&pnu).await;
    // 셋째 호출 — circuit 이 Open 이라 즉시 Fetch error (네트워크 호출 안 함)
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
