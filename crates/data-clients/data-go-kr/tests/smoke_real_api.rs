//! data.go.kr `BldRgstHubService` 실 API smoke test (SP7-iii).
//!
//! 평소 `cargo test` 에서 빌드/실행 X — `#![cfg(feature = "real-api")]` + `#[ignore]`.
//! CI nightly cron (T6 의 `.github/workflows/api-drift-smoke-test.yml`) 또는
//! 로컬 검증:
//! ```bash
//! cargo test --features real-api -p data-go-kr-client \
//!     --test smoke_real_api -- --ignored --nocapture
//! ```
//!
//! 환경변수:
//! - `ODP_SERVICE_KEY` (필수) — data.go.kr 발급 키
//! - `GONGZZANG_DRIFT_TEST_PNU` (옵션, default `1168010100107370000` = 강남파이낸스센터)
//!   `simulate_failure` workflow input 시 `9999999999999999999` 로 override 됨
//!
//! 검증 (drift 검출):
//! 1. `BuildingRegisterClient::fetch_title_info` 가 실 API HTTP 응답 받음 (endpoint URL drift 검출)
//! 2. `parse_building_title` 통과 (schema drift 검출)
//! 3. mainPurpsCd 매핑: 강남파이낸스 = `BuildingPurposeCode::Office`
//! 4. strctCd 매핑: 강남파이낸스 = `BuildingStructureCode::SteelReinforcedConcrete`

#![cfg(feature = "real-api")]
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::structure_code::BuildingStructureCode;
use chrono::Utc;
use data_go_kr_client::building_register::parser::parse_building_title;
use data_go_kr_client::building_register::BuildingRegisterClient;
use data_go_kr_client::pnu_split::split;
use data_go_kr_client::{DataGoKrClient, DataGoKrConfig};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

fn dummy_polygon() -> PolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![])).expect("valid")
}

#[tokio::test]
#[ignore = "real API call — requires ODP_SERVICE_KEY; runs only in CI nightly cron (T6 워크플로우)"]
async fn smoke_data_go_kr_building_register_alive() {
    let key = std::env::var("ODP_SERVICE_KEY").expect("ODP_SERVICE_KEY required");

    let pnu_str = std::env::var("GONGZZANG_DRIFT_TEST_PNU")
        .unwrap_or_else(|_| "1168010100107370000".to_owned());
    let pnu = Pnu::try_new(&pnu_str).expect("valid PNU");

    let config = DataGoKrConfig {
        service_key: key,
        base_url: "https://apis.data.go.kr".to_owned(),
    };
    let client = DataGoKrClient::new(config);

    let br = BuildingRegisterClient::new(&client);
    let raw = br
        .fetch_title_info(split(&pnu))
        .await
        .expect("HTTP call should succeed (endpoint URL drift 의심?)");

    let buildings = parse_building_title(&raw, &pnu, &dummy_polygon(), Utc::now())
        .expect("parser should accept response (schema drift 의심?)");

    assert!(
        !buildings.is_empty(),
        "응답 0건 — endpoint drift 또는 PNU 잘못됨 (simulate_failure 의도된 fail?)"
    );

    // 강남파이낸스센터 검증 (default PNU = 1168010100107370000)
    if pnu.as_str() == "1168010100107370000" {
        let b = &buildings[0];
        assert_eq!(
            b.main_purpose_code,
            BuildingPurposeCode::Office,
            "mainPurpsCd 14000 → Office 매핑 검증"
        );
        assert_eq!(
            b.structure_code,
            BuildingStructureCode::SteelReinforcedConcrete,
            "strctCd 42 → SRC 매핑 검증"
        );
    }
}
