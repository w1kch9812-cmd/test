//! FU 41 — 실 data.go.kr `BldRgstHubService/getBrTitleInfo` 응답 fixture 검증.
//!
//! 2026-05-04 호출 (역삼동 본번 sweep 9건 → 5 fixture, 6 케이스):
//! - `mainPurpsCd` 5자리 표준 코드 (`01000` ~ `29000`)
//! - `strctCd` 2자리 표준 코드 (`11`, `21`, `42`)
//!
//! 본 테스트는 [`parse_building_title`] 의 Cd primary 매핑이 실제 응답에서
//! 도메인 enum 으로 정확히 변환됨을 검증.
//!
//! Fixture 파일 명명: `real_<설명>_<주코드>[_<부코드>].json`
//! - `real_office_gangnam_finance_14000.json` — 강남파이낸스센터, 업무시설(14000) + SRC(42)
//! - `real_education_10000.json` — FUTURE VALUE CAMPUS, 교육연구시설(10000) + RC(21)
//! - `real_apartment_02000.json` — 역삼동한스빌라텔, 공동주택(02000) + RC(21)
//! - `real_house_brick_01000.json` — 단독주택(01000) + 벽돌(11)
//! - `real_kunrin1_house_03000_01000.json` — 2 items: 제1종근린(03000) + 단독주택(01000)

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::collections::HashSet;

use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::structure_code::BuildingStructureCode;
use chrono::Utc;
use data_go_kr_client::building_register::parser::parse_building_title;
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use serde_json::Value;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

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

fn load_fixture(name: &str) -> Value {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
    serde_json::from_str(&raw).expect("valid JSON fixture")
}

/// fixture 의 첫 item 으로부터 PNU 19자리 합성 (sigunguCd + bjdongCd + platGbCd + bun + ji).
fn pnu_from_first_item(raw: &Value) -> Pnu {
    let item = &raw["response"]["body"]["items"]["item"][0];
    let s = format!(
        "{}{}{}{}{}",
        item["sigunguCd"].as_str().expect("sigunguCd"),
        item["bjdongCd"].as_str().expect("bjdongCd"),
        item["platGbCd"].as_str().expect("platGbCd"),
        item["bun"].as_str().expect("bun"),
        item["ji"].as_str().expect("ji"),
    );
    Pnu::try_new(&s).expect("valid PNU")
}

#[test]
fn real_office_gangnam_finance_maps_cd_14000_to_office_and_42_to_src() {
    let raw = load_fixture("real_office_gangnam_finance_14000.json");
    let pnu = pnu_from_first_item(&raw);

    let buildings = parse_building_title(&raw, &pnu, &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Office);
    assert_eq!(
        b.structure_code,
        BuildingStructureCode::SteelReinforcedConcrete
    );
    assert_eq!(b.building_name.as_deref(), Some("강남파이낸스센터"));
    // 실 API 응답: mainPurpsCdNm "업무시설", strctCdNm "철골철근콘크리트구조"
    let item = &raw["response"]["body"]["items"]["item"][0];
    assert_eq!(item["mainPurpsCd"].as_str(), Some("14000"));
    assert_eq!(item["strctCd"].as_str(), Some("42"));
}

#[test]
fn real_education_maps_cd_10000_to_educational_and_21_to_rc() {
    let raw = load_fixture("real_education_10000.json");
    let pnu = pnu_from_first_item(&raw);

    let buildings = parse_building_title(&raw, &pnu, &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Educational);
    assert_eq!(b.structure_code, BuildingStructureCode::ReinforcedConcrete);

    let item = &raw["response"]["body"]["items"]["item"][0];
    assert_eq!(item["mainPurpsCd"].as_str(), Some("10000"));
    assert_eq!(item["mainPurpsCdNm"].as_str(), Some("교육연구시설"));
}

#[test]
fn real_apartment_maps_cd_02000_to_multi_house() {
    let raw = load_fixture("real_apartment_02000.json");
    let pnu = pnu_from_first_item(&raw);

    let buildings = parse_building_title(&raw, &pnu, &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::MultiHouse);
    assert_eq!(b.building_name.as_deref(), Some("역삼동한스빌라텔"));

    let item = &raw["response"]["body"]["items"]["item"][0];
    assert_eq!(item["mainPurpsCd"].as_str(), Some("02000"));
    assert_eq!(item["mainPurpsCdNm"].as_str(), Some("공동주택"));
}

#[test]
fn real_house_brick_maps_cd_01000_to_single_house_and_11_to_brick() {
    let raw = load_fixture("real_house_brick_01000.json");
    let pnu = pnu_from_first_item(&raw);

    let buildings = parse_building_title(&raw, &pnu, &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 1);
    let b = &buildings[0];
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::SingleHouse);
    assert_eq!(b.structure_code, BuildingStructureCode::Brick);

    let item = &raw["response"]["body"]["items"]["item"][0];
    assert_eq!(item["mainPurpsCd"].as_str(), Some("01000"));
    assert_eq!(item["strctCd"].as_str(), Some("11"));
}

#[test]
fn real_kunrin1_house_two_items_map_to_retail_and_single_house() {
    // 역삼동 741 — 2 items: 03000 제1종근린생활시설 + 01000 단독주택
    let raw = load_fixture("real_kunrin1_house_03000_01000.json");
    let pnu = pnu_from_first_item(&raw);

    let buildings = parse_building_title(&raw, &pnu, &sample_polygon(), Utc::now()).expect("ok");
    assert_eq!(buildings.len(), 2);

    let purposes: HashSet<_> = buildings.iter().map(|b| b.main_purpose_code).collect();
    assert!(purposes.contains(&BuildingPurposeCode::Retail));
    assert!(purposes.contains(&BuildingPurposeCode::SingleHouse));
}

#[test]
fn real_kunrin_label_is_je_jong_form_not_unified() {
    // 실 API 검증: 한글 라벨이 "근린생활시설" 단일 X — "제1종근린생활시설" / "제2종근린생활시설"
    // 분리. 우리 spec/plan 의 추정 매핑이 실제와 다름을 입증 (Cd primary 가 SSS 정답인 이유).
    let raw = load_fixture("real_kunrin1_house_03000_01000.json");
    let items = raw["response"]["body"]["items"]["item"]
        .as_array()
        .expect("array");

    let kunrin = items
        .iter()
        .find(|i| i["mainPurpsCd"] == "03000")
        .expect("03000 item");
    assert_eq!(
        kunrin["mainPurpsCdNm"].as_str(),
        Some("제1종근린생활시설"),
        "실 API 가 분리된 표기를 사용 — Cd primary 매핑이 SSS 정답"
    );
}
