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

#[test]
fn building_constructs_from_r2_data() {
    let b = Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        building_name: Some("샘플 공장".to_owned()),
        main_purpose_code: BuildingPurposeCode::Factory,
        structure_code: BuildingStructureCode::ReinforcedConcrete,
        total_floor_area_m2: AreaM2::try_new(5000.0).unwrap(),
        ground_floors: 5,
        underground_floors: 1,
        height_m: Some(20.5),
        use_approval_date: Some(NaiveDate::from_ymd_opt(2020, 5, 15).unwrap()),
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    assert_eq!(b.main_purpose_code, BuildingPurposeCode::Factory);
    assert_eq!(b.structure_code, BuildingStructureCode::ReinforcedConcrete);
    assert_eq!(b.ground_floors, 5);
    assert_eq!(b.underground_floors, 1);
    assert_eq!(b.height_m, Some(20.5));
}

#[test]
fn building_optional_fields_none() {
    let b = Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        building_name: None,
        main_purpose_code: BuildingPurposeCode::Other,
        structure_code: BuildingStructureCode::Other,
        total_floor_area_m2: AreaM2::try_new(100.0).unwrap(),
        ground_floors: 1,
        underground_floors: 0,
        height_m: None,
        use_approval_date: None,
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    assert!(b.building_name.is_none());
    assert!(b.height_m.is_none());
    assert!(b.use_approval_date.is_none());
}

#[test]
fn building_serde_roundtrip() {
    let b = Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        building_name: Some("Test".to_owned()),
        main_purpose_code: BuildingPurposeCode::Warehouse,
        structure_code: BuildingStructureCode::Steel,
        total_floor_area_m2: AreaM2::try_new(2500.0).unwrap(),
        ground_floors: 3,
        underground_floors: 0,
        height_m: Some(15.0),
        use_approval_date: Some(NaiveDate::from_ymd_opt(2018, 12, 31).unwrap()),
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    let json = serde_json::to_string(&b).expect("serialize");
    let back: Building = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(b, back);
}

#[test]
fn building_clone_preserves_fields() {
    let b = Building {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        building_name: Some("지식산업센터 A동".to_owned()),
        main_purpose_code: BuildingPurposeCode::KnowledgeIndustryCenter,
        structure_code: BuildingStructureCode::SteelReinforcedConcrete,
        total_floor_area_m2: AreaM2::try_new(50000.0).unwrap(),
        ground_floors: 15,
        underground_floors: 3,
        height_m: Some(60.0),
        use_approval_date: Some(NaiveDate::from_ymd_opt(2022, 3, 1).unwrap()),
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    let cloned = b.clone();
    assert_eq!(b, cloned);
}
