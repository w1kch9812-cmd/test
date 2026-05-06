//! data.go.kr 건축물대장 통합 테스트 (SP4-iii-a) — wiremock 으로 fake server.
//!
//! 6 시나리오:
//! 1. `fetch_by_pnu` happy path — 200 + 단일 건물 → `Vec[1]`
//! 2. `fetch_by_pnu` multi buildings — items.item 배열 3건물 → `Vec[3]`
//! 3. `fetch_by_pnu` empty items → `Vec[]` (V-World 호출 안 함)
//! 4. `fetch_by_pnu` 5xx 재시도 후 실패 → `ReaderError::Fetch`
//! 5. `fetch_by_pnu` malformed → `ReaderError::Parse`
//! 6. `fetch_by_pnu` circuit open after threshold failures
//!
//! 한 `MockServer` 가 data.go.kr (`/1613000/...`) + V-World (`/req/data`)
//! 두 path 를 동시에 처리. 분리하지 않은 이유 — `wiremock` path matcher 가
//! 충분.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::sync::Arc;

use building_domain::errors::ReaderError;
use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::reader::BuildingReader;
use building_domain::structure_code::BuildingStructureCode;
use circuit_breaker::Policy;
use data_go_kr_client::building_register::DataGoKrBuildingReader;
use data_go_kr_client::{DataGoKrClient, DataGoKrConfig};
use raw_capture_client::NoOpRawCapture;
use shared_kernel::pnu::Pnu;
use vworld_client::{VWorldClient, VWorldConfig};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const BR_PATH: &str = "/1613000/BldRgstHubService/getBrTitleInfo";
const VWORLD_PATH: &str = "/req/data";
const SAMPLE_PNU: &str = "1111010100100010000";

fn factory_item() -> serde_json::Value {
    serde_json::json!({
        "bldNm": "공장1동",
        "mainPurpsCdNm": "공장",
        "strctCdNm": "철골구조",
        "totArea": "1500.50",
        "grndFlrCnt": "3",
        "ugrndFlrCnt": "1",
        "heit": "12.5",
        "useAprDay": "20100315",
        "platPlc": "...",
        "mgmBldrgstPk": "12345678901234567"
    })
}

fn warehouse_item() -> serde_json::Value {
    serde_json::json!({
        "bldNm": "창고2동",
        "mainPurpsCdNm": "창고",
        "strctCdNm": "철근콘크리트",
        "totArea": "800.0",
        "grndFlrCnt": "2",
        "ugrndFlrCnt": "0",
        "heit": "8.0",
        "useAprDay": "20150601"
    })
}

fn office_item() -> serde_json::Value {
    serde_json::json!({
        "bldNm": "사무동",
        "mainPurpsCdNm": "업무시설",
        "strctCdNm": "철근콘크리트",
        "totArea": "2200.0",
        "grndFlrCnt": "5",
        "ugrndFlrCnt": "1",
        "heit": "20.0",
        "useAprDay": "20180420"
    })
}

fn br_response_single() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": {
                "items": { "item": factory_item() },
                "totalCount": "1", "pageNo": "1", "numOfRows": "100"
            }
        }
    })
}

fn br_response_multi() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": {
                "items": { "item": [factory_item(), warehouse_item(), office_item()] },
                "totalCount": "3", "pageNo": "1", "numOfRows": "100"
            }
        }
    })
}

fn br_response_empty() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": { "items": "", "totalCount": "0", "pageNo": "1", "numOfRows": "100" }
        }
    })
}

/// V-World `LP_PA_CBND_BUBUN` 응답 모양 (실 API 캡처 기반).
///
/// 본 테스트가 검증하는 건 `building_register` reader 가 V-World geom 합성
/// path 를 거치는지 — V-World 응답 자체 검증은 `vworld-client` crate 책임.
/// 그래서 fixture 는 envelope 통과 + 단일 `MultiPolygon` feature 의 최소형.
fn vworld_response() -> serde_json::Value {
    serde_json::json!({
        "response": {
            "service": { "name": "data", "version": "2.0", "operation": "GetFeature" },
            "status": "OK",
            "record": { "total": "1", "current": "1" },
            "result": {
                "featureCollection": {
                    "type": "FeatureCollection",
                    "features": [{
                        "type": "Feature",
                        "geometry": {
                            "type": "MultiPolygon",
                            "coordinates": [[[
                                [126.97, 37.56], [126.98, 37.56],
                                [126.98, 37.57], [126.97, 37.57],
                                [126.97, 37.56]
                            ]]]
                        },
                        "properties": {
                            "pnu": SAMPLE_PNU,
                            "jibun": "1-1 공장용지",
                            "addr": "서울특별시 종로구 청운동 1-1",
                            "jiga": "5000000",
                            "gosi_year": "2025",
                            "gosi_month": "01"
                        }
                    }]
                }
            }
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

fn build_reader(server: &MockServer, policy: Policy) -> DataGoKrBuildingReader {
    let dgk_cfg = DataGoKrConfig {
        service_key: "test-key".to_owned(),
        base_url: server.uri(),
    };
    let dgk_client = Arc::new(DataGoKrClient::with_policy(dgk_cfg, policy));

    let vw_cfg = VWorldConfig {
        api_key: "test-key".to_owned(),
        domain: "localhost".to_owned(),
        base_url: server.uri(),
    };
    let vw_client = Arc::new(VWorldClient::with_policy(vw_cfg, policy));

    DataGoKrBuildingReader::new(dgk_client, vw_client, Arc::new(NoOpRawCapture::new()))
}

async fn mount_vworld_ok(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path(VWORLD_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(vworld_response()))
        .mount(server)
        .await;
}

#[tokio::test]
async fn fetch_by_pnu_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(br_response_single()))
        .mount(&server)
        .await;
    mount_vworld_ok(&server).await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();
    let buildings = reader.fetch_by_pnu(&pnu).await.expect("ok");

    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.pnu.as_str(), SAMPLE_PNU);
    assert_eq!(b.building_name.as_deref(), Some("공장1동"));
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(b.structure_code, BuildingStructureCode::Steel);
    assert!((b.total_floor_area_m2.as_f64() - 1500.50).abs() < 0.001);
    assert_eq!(b.ground_floors, 3);
    assert_eq!(b.underground_floors, 1);
    assert!(b.height_m.is_some());
    assert!(b.use_approval_date.is_some());
}

#[tokio::test]
async fn fetch_by_pnu_multi_buildings() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(br_response_multi()))
        .mount(&server)
        .await;
    mount_vworld_ok(&server).await;

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();
    let buildings = reader.fetch_by_pnu(&pnu).await.expect("ok");

    assert_eq!(buildings.len(), 3);
    assert_eq!(buildings[0].main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(
        buildings[1].main_purpose_code,
        BuildingPurposeCode::Warehouse
    );
    assert_eq!(buildings[2].main_purpose_code, BuildingPurposeCode::Office);
    // 모두 같은 PNU + 같은 (합성된) geom 공유 — V-World 1회 호출.
    assert_eq!(buildings[0].pnu, buildings[1].pnu);
    assert_eq!(buildings[1].geom, buildings[2].geom);
}

#[tokio::test]
async fn fetch_by_pnu_empty_returns_empty_vec() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(br_response_empty()))
        .mount(&server)
        .await;
    // V-World mock 일부러 안 mount — empty 분기에서 호출 안 함을 검증.

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();
    let buildings = reader.fetch_by_pnu(&pnu).await.expect("ok");
    assert!(buildings.is_empty());
}

#[tokio::test]
async fn fetch_by_pnu_5xx_retries_then_fails() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    mount_vworld_ok(&server).await; // 도달 안 함.

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    assert!(matches!(err, ReaderError::Fetch(_)));
}

#[tokio::test]
async fn fetch_by_pnu_malformed_returns_parse_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({ "unexpected": "shape" })),
        )
        .mount(&server)
        .await;
    mount_vworld_ok(&server).await; // header 없으면 V-World 호출 전에 parse 실패.

    let reader = build_reader(&server, fast_policy());
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    // items_present 가 false → V-World 호출 안 함, parse_building_title 가
    // header 없음 detect → Malformed → Parse(...).
    assert!(matches!(err, ReaderError::Parse(_)));
}

#[tokio::test]
async fn fetch_by_pnu_circuit_opens_after_threshold() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(BR_PATH))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;
    mount_vworld_ok(&server).await;

    // threshold 3 — 호출 1번 = 503 ×2 (initial + retry) → 2 failures.
    // 두번째 호출 시 4 failures → open.
    let policy = Policy {
        max_retries: 1,
        open_threshold: 3,
        ..fast_policy()
    };
    let reader = build_reader(&server, policy);
    let pnu = Pnu::try_new(SAMPLE_PNU).unwrap();

    let _ = reader.fetch_by_pnu(&pnu).await;
    let _ = reader.fetch_by_pnu(&pnu).await;
    let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
    match err {
        ReaderError::Fetch(msg) => assert!(msg.contains("circuit open"), "msg: {msg}"),
        other => panic!("expected Fetch(circuit open), got {other:?}"),
    }
}
