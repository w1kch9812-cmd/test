//! `Building` Aggregate 테스트.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::Building;
use chrono::{NaiveDate, Utc};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

use crate::purpose_code::BuildingPurposeCode;
use crate::structure_code::BuildingStructureCode;

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

/// 모든 필드 채워진 sample — 새 필드 추가 시 본 helper 도 갱신.
fn sample_building_full() -> Building {
    Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        mgm_bldrgst_pk: "1024112777".to_owned(),
        plat_plc: Some("서울특별시 강남구 역삼동 737".to_owned()),
        building_name: Some("샘플 공장".to_owned()),
        main_purpose_code: BuildingPurposeCode::Factory,
        structure_code: BuildingStructureCode::ReinforcedConcrete,
        plat_area_m2: Some(AreaM2::try_new(13_156.7).unwrap()),
        arch_area_m2: Some(AreaM2::try_new(5_600.51).unwrap()),
        building_coverage_ratio: Some(42.5677),
        total_floor_area_m2: AreaM2::try_new(5000.0).unwrap(),
        floor_area_ratio: Some(995.1887),
        ground_floors: 5,
        underground_floors: 1,
        height_m: Some(20.5),
        passenger_elevators: Some(29),
        emergency_elevators: Some(2),
        indoor_self_parking: Some(1300),
        outdoor_self_parking: Some(12),
        annex_building_count: Some(0),
        annex_building_area_m2: None,
        permit_date: Some(NaiveDate::from_ymd_opt(1995, 5, 4).unwrap()),
        construction_start_date: Some(NaiveDate::from_ymd_opt(1995, 5, 13).unwrap()),
        use_approval_date: Some(NaiveDate::from_ymd_opt(2020, 5, 15).unwrap()),
        geom: Some(sample_polygon()),
        fetched_at: Utc::now(),
    }
}

/// 핵심 필드만 채운 minimal — `Option` 모두 None.
fn sample_building_minimal() -> Building {
    Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        mgm_bldrgst_pk: "M".to_owned(),
        plat_plc: None,
        building_name: None,
        main_purpose_code: BuildingPurposeCode::Other,
        structure_code: BuildingStructureCode::Other,
        plat_area_m2: None,
        arch_area_m2: None,
        building_coverage_ratio: None,
        total_floor_area_m2: AreaM2::try_new(100.0).unwrap(),
        floor_area_ratio: None,
        ground_floors: 1,
        underground_floors: 0,
        height_m: None,
        passenger_elevators: None,
        emergency_elevators: None,
        indoor_self_parking: None,
        outdoor_self_parking: None,
        annex_building_count: None,
        annex_building_area_m2: None,
        permit_date: None,
        construction_start_date: None,
        use_approval_date: None,
        geom: None,
        fetched_at: Utc::now(),
    }
}

#[test]
fn building_constructs_from_full_data() {
    let b = sample_building_full();
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(b.structure_code, BuildingStructureCode::ReinforcedConcrete);
    assert_eq!(b.ground_floors, 5);
    assert_eq!(b.underground_floors, 1);
    assert_eq!(b.height_m, Some(20.5));
    assert_eq!(b.mgm_bldrgst_pk, "1024112777");
    assert!(b.plat_area_m2.is_some());
    assert!(b.geom.is_some());
}

#[test]
fn building_optional_fields_none() {
    let b = sample_building_minimal();
    assert!(b.building_name.is_none());
    assert!(b.height_m.is_none());
    assert!(b.use_approval_date.is_none());
    assert!(b.plat_area_m2.is_none());
    assert!(b.geom.is_none(), "panel-only path 는 geom None");
}

#[test]
fn building_serde_roundtrip() {
    let b = sample_building_full();
    let json = serde_json::to_string(&b).expect("serialize");
    let back: Building = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(b, back);
}

#[test]
fn building_clone_preserves_fields() {
    let b = sample_building_full();
    let cloned = b.clone();
    assert_eq!(b, cloned);
}

#[test]
fn building_minimal_serde_roundtrip() {
    let b = sample_building_minimal();
    let json = serde_json::to_string(&b).expect("serialize");
    let back: Building = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(b, back);
}
