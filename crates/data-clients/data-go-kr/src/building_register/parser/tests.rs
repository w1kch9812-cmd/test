#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use geo_types::{Coord, LineString, Polygon as GeoPolygon};

fn sample_polygon() -> PolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![])).expect("valid")
}

fn sample_pnu() -> Pnu {
    Pnu::try_new("1111010100100010000").expect("valid")
}

fn ok_response(items: &Value) -> Value {
    serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": {
                "items": items,
                "totalCount": "1",
                "pageNo": "1",
                "numOfRows": "100"
            }
        }
    })
}

fn factory_item() -> Value {
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

#[test]
fn parse_single_factory_happy_path() {
    let raw = ok_response(&serde_json::json!({ "item": factory_item() }));
    let now = Utc::now();
    let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), now).expect("ok");

    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.building_name.as_deref(), Some("공장1동"));
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(b.structure_code, BuildingStructureCode::Steel);
    assert!((b.total_floor_area_m2.as_f64() - 1500.50).abs() < 0.001);
    assert_eq!(b.ground_floors, 3);
    assert_eq!(b.underground_floors, 1);
    assert!((b.height_m.unwrap() - 12.5).abs() < 0.001);
    assert_eq!(
        b.use_approval_date,
        Some(NaiveDate::from_ymd_opt(2010, 3, 15).unwrap())
    );
    assert_eq!(b.fetched_at, now);
}

#[test]
fn parse_array_of_items() {
    let item2 = serde_json::json!({
        "bldNm": "창고2동",
        "mainPurpsCdNm": "창고",
        "strctCdNm": "철근콘크리트",
        "totArea": "800.0",
        "grndFlrCnt": "2",
        "ugrndFlrCnt": "0",
        "heit": "8.0",
        "useAprDay": "20150601"
    });
    let raw = ok_response(&serde_json::json!({
        "item": [factory_item(), item2]
    }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 2);
    assert_eq!(buildings[0].main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(
        buildings[1].main_purpose_code,
        BuildingPurposeCode::Warehouse
    );
    assert_eq!(
        buildings[1].structure_code,
        BuildingStructureCode::ReinforcedConcrete
    );
}

#[test]
fn parse_empty_items_string_returns_empty_vec() {
    let raw = serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": { "items": "", "totalCount": "0", "pageNo": "1", "numOfRows": "100" }
        }
    });
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert!(buildings.is_empty());
}

#[test]
fn parse_missing_items_returns_empty_vec() {
    let raw = serde_json::json!({
        "response": {
            "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
            "body": { "totalCount": "0" }
        }
    });
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert!(buildings.is_empty());
}

#[test]
fn parse_api_error_returns_api_error_variant() {
    let raw = serde_json::json!({
        "response": {
            "header": {
                "resultCode": "30",
                "resultMsg": "SERVICE KEY IS NOT REGISTERED ERROR."
            },
            "body": {}
        }
    });
    let err = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
    match err {
        ParseError::ApiError { code, msg } => {
            assert_eq!(code, "30");
            assert!(msg.contains("SERVICE KEY"));
        }
        other => panic!("expected ApiError, got {other:?}"),
    }
}

#[test]
fn parse_missing_header_returns_malformed() {
    let raw = serde_json::json!({ "response": { "body": {} } });
    let err = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
    assert!(matches!(err, ParseError::Malformed(s) if s.contains("header")));
}

#[test]
fn parse_unknown_purpose_falls_back_to_other() {
    let mut item = factory_item();
    item["mainPurpsCdNm"] = serde_json::json!("의료시설"); // 매핑표 외 라벨
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings[0].main_purpose_code, BuildingPurposeCode::Other);
}

#[test]
fn parse_invalid_total_area_returns_domain_error() {
    let mut item = factory_item();
    item["totArea"] = serde_json::json!("not-a-number");
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let err = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
    assert!(matches!(err, ParseError::Domain(s) if s.contains("totArea")));
}

#[test]
fn parse_negative_total_area_returns_domain_error() {
    let mut item = factory_item();
    item["totArea"] = serde_json::json!("-100.0");
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let err = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
    assert!(matches!(err, ParseError::Domain(_)));
}

#[test]
fn parse_zero_height_becomes_none() {
    let mut item = factory_item();
    item["heit"] = serde_json::json!("0");
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert!(buildings[0].height_m.is_none());
}

#[test]
fn parse_invalid_use_apr_day_becomes_none() {
    let mut item = factory_item();
    item["useAprDay"] = serde_json::json!("99999999"); // invalid YYYYMMDD
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert!(buildings[0].use_approval_date.is_none());
}

#[test]
fn parse_empty_building_name_becomes_none() {
    let mut item = factory_item();
    item["bldNm"] = serde_json::json!("");
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert!(buildings[0].building_name.is_none());
}

#[test]
fn parse_empty_floor_count_becomes_zero() {
    let mut item = factory_item();
    item["ugrndFlrCnt"] = serde_json::json!("");
    let raw = ok_response(&serde_json::json!({ "item": item }));
    let buildings =
        parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings[0].underground_floors, 0);
}

// ─── FU 41: Cd primary + CdNm fallback 하이브리드 매핑 ───
//
// 실 API 검증 (2026-05-04, 역삼동 본번 sweep 9건 → 5 fixture, 6 케이스):
// - mainPurpsCd 5자리 (`14000` 업무 / `02000` 공동주택 / `10000` 교육 / `01000` 단독 /
//   `03000` 제1종근린 / `04000` 제2종근린)
// - strctCd 2자리 (`11` 벽돌 / `21` RC / `42` SRC)

#[test]
fn map_purpose_cd_industrial_codes() {
    assert_eq!(map_purpose_cd("17000"), Some(BuildingPurposeCode::Factory));
    assert_eq!(
        map_purpose_cd("18000"),
        Some(BuildingPurposeCode::Warehouse)
    );
    assert_eq!(map_purpose_cd("14000"), Some(BuildingPurposeCode::Office));
    assert_eq!(
        map_purpose_cd("01000"),
        Some(BuildingPurposeCode::SingleHouse)
    );
    assert_eq!(
        map_purpose_cd("02000"),
        Some(BuildingPurposeCode::MultiHouse)
    );
    assert_eq!(
        map_purpose_cd("10000"),
        Some(BuildingPurposeCode::Educational)
    );
}

#[test]
fn map_purpose_cd_retail_collapses_three_codes() {
    // 03000 제1종근린 / 04000 제2종근린 / 07000 판매시설 모두 Retail
    assert_eq!(map_purpose_cd("03000"), Some(BuildingPurposeCode::Retail));
    assert_eq!(map_purpose_cd("04000"), Some(BuildingPurposeCode::Retail));
    assert_eq!(map_purpose_cd("07000"), Some(BuildingPurposeCode::Retail));
}

#[test]
fn map_purpose_cd_non_industrial_returns_none() {
    // 09000 의료 / 06000 종교 / 15000 숙박 / 13000 운동 → None (CdNm fallback 시도)
    assert_eq!(map_purpose_cd("09000"), None);
    assert_eq!(map_purpose_cd("06000"), None);
    assert_eq!(map_purpose_cd("15000"), None);
    assert_eq!(map_purpose_cd("13000"), None);
}

#[test]
fn map_purpose_cd_unknown_returns_none() {
    assert_eq!(map_purpose_cd("99999"), None);
    assert_eq!(map_purpose_cd(""), None);
    assert_eq!(map_purpose_cd("abc"), None);
}

#[test]
fn map_purpose_label_kunrin_split_variants() {
    // 실 API: "근린생활시설" 단일 X — "제1종/제2종근린생활시설" 분리 (검증됨)
    assert_eq!(
        map_purpose_label("제1종근린생활시설"),
        BuildingPurposeCode::Retail
    );
    assert_eq!(
        map_purpose_label("제2종근린생활시설"),
        BuildingPurposeCode::Retail
    );
    // legacy / 단일 표기 호환
    assert_eq!(
        map_purpose_label("근린생활시설"),
        BuildingPurposeCode::Retail
    );
    assert_eq!(map_purpose_label("판매시설"), BuildingPurposeCode::Retail);
}

#[test]
fn map_purpose_label_industrial_special() {
    // 별도 mainPurpsCd 없는 산업 도메인 분류 (CdNm fallback 으로만 검출)
    assert_eq!(
        map_purpose_label("지식산업센터"),
        BuildingPurposeCode::KnowledgeIndustryCenter
    );
    assert_eq!(
        map_purpose_label("지식산업센터(아파트형공장)"),
        BuildingPurposeCode::KnowledgeIndustryCenter
    );
    assert_eq!(
        map_purpose_label("아파트형공장"),
        BuildingPurposeCode::KnowledgeIndustryCenter
    );
    assert_eq!(
        map_purpose_label("물류시설"),
        BuildingPurposeCode::LogisticsCenter
    );
    assert_eq!(
        map_purpose_label("물류창고"),
        BuildingPurposeCode::LogisticsCenter
    );
    assert_eq!(
        map_purpose_label("물류터미널"),
        BuildingPurposeCode::LogisticsCenter
    );
}

#[test]
fn map_purpose_label_apartment_variants() {
    for label in ["공동주택", "다세대주택", "다가구주택", "아파트", "연립주택"]
    {
        assert_eq!(map_purpose_label(label), BuildingPurposeCode::MultiHouse);
    }
}

#[test]
fn map_purpose_label_unknown_returns_other() {
    assert_eq!(map_purpose_label("의료시설"), BuildingPurposeCode::Other);
    assert_eq!(map_purpose_label("종교시설"), BuildingPurposeCode::Other);
    assert_eq!(map_purpose_label("문화시설"), BuildingPurposeCode::Other);
    assert_eq!(map_purpose_label(""), BuildingPurposeCode::Other);
}

#[test]
fn parse_purpose_cd_primary_overrides_label() {
    // Cd "14000" (업무) + CdNm "공장" — Cd primary 우선 → Office
    let item = serde_json::json!({
        "mainPurpsCd": "14000",
        "mainPurpsCdNm": "공장"
    });
    assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Office);
}

#[test]
fn parse_purpose_cd_missing_falls_back_to_label() {
    // Cd 누락 → CdNm "공장" → Factory
    let item = serde_json::json!({ "mainPurpsCdNm": "공장" });
    assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Factory);
}

#[test]
fn parse_purpose_cd_unmapped_falls_back_to_label() {
    // Cd "99999" 비매핑 → CdNm "공장" → Factory
    let item = serde_json::json!({
        "mainPurpsCd": "99999",
        "mainPurpsCdNm": "공장"
    });
    assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Factory);
}

#[test]
fn parse_purpose_cd_non_industrial_falls_back_to_other() {
    // Cd "09000" 의료 (None) → CdNm "의료시설" (Other) → Other
    let item = serde_json::json!({
        "mainPurpsCd": "09000",
        "mainPurpsCdNm": "의료시설"
    });
    assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Other);
}

#[test]
fn parse_purpose_industrial_special_via_label_fallback() {
    // Cd 누락 + CdNm "지식산업센터" → KnowledgeIndustryCenter
    let item = serde_json::json!({ "mainPurpsCdNm": "지식산업센터" });
    assert_eq!(
        parse_purpose(&item).unwrap(),
        BuildingPurposeCode::KnowledgeIndustryCenter
    );
}

#[test]
fn parse_purpose_missing_label_returns_malformed() {
    // Cd 누락 + CdNm 도 누락 → Malformed
    let item = serde_json::json!({});
    let err = parse_purpose(&item).unwrap_err();
    assert!(matches!(err, ParseError::Malformed(s) if s.contains("mainPurpsCdNm")));
}

#[test]
fn parse_purpose_empty_cd_falls_back_to_label() {
    // Cd 빈 문자열 (정부 API 가 종종 반환) → label fallback
    let item = serde_json::json!({
        "mainPurpsCd": "  ",
        "mainPurpsCdNm": "공장"
    });
    assert_eq!(parse_purpose(&item).unwrap(), BuildingPurposeCode::Factory);
}

#[test]
fn map_structure_cd_verified_codes() {
    // 실 fixture 검증된 3 코드
    assert_eq!(map_structure_cd("11"), Some(BuildingStructureCode::Brick));
    assert_eq!(
        map_structure_cd("21"),
        Some(BuildingStructureCode::ReinforcedConcrete)
    );
    assert_eq!(
        map_structure_cd("42"),
        Some(BuildingStructureCode::SteelReinforcedConcrete)
    );
}

#[test]
fn map_structure_cd_unverified_returns_none() {
    // 미검증 코드 → None → CdNm fallback
    assert_eq!(map_structure_cd("31"), None);
    assert_eq!(map_structure_cd("99"), None);
    assert_eq!(map_structure_cd(""), None);
}

#[test]
fn map_structure_label_steel_variants() {
    assert_eq!(
        map_structure_label("철골구조"),
        BuildingStructureCode::Steel
    );
    assert_eq!(map_structure_label("철골"), BuildingStructureCode::Steel);
}

#[test]
fn map_structure_label_src_variants() {
    assert_eq!(
        map_structure_label("철골철근콘크리트구조"),
        BuildingStructureCode::SteelReinforcedConcrete
    );
    assert_eq!(
        map_structure_label("SRC구조"),
        BuildingStructureCode::SteelReinforcedConcrete
    );
}

#[test]
fn map_structure_label_unknown_returns_other() {
    assert_eq!(map_structure_label("조립식"), BuildingStructureCode::Other);
    assert_eq!(map_structure_label(""), BuildingStructureCode::Other);
}

#[test]
fn parse_structure_cd_primary_overrides_label() {
    // Cd "21" (RC) + CdNm "철골" — Cd primary 우선 → RC
    let item = serde_json::json!({
        "strctCd": "21",
        "strctCdNm": "철골"
    });
    assert_eq!(
        parse_structure(&item).unwrap(),
        BuildingStructureCode::ReinforcedConcrete
    );
}

#[test]
fn parse_structure_cd_missing_falls_back_to_label() {
    let item = serde_json::json!({ "strctCdNm": "철근콘크리트구조" });
    assert_eq!(
        parse_structure(&item).unwrap(),
        BuildingStructureCode::ReinforcedConcrete
    );
}
