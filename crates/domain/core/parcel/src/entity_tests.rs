//! `Parcel` Aggregate 테스트.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::Parcel;
use chrono::Utc;
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use shared_kernel::address::{JibunAddress, RoadAddress};
use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::zoning::Zoning;

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

fn sample_admin() -> AdminDivision {
    AdminDivision::try_new(
        SidoCode::try_new("11").unwrap(),
        SigunguCode::try_new("11110").unwrap(),
        EupmyeondongCode::try_new("11110101").unwrap(),
    )
    .unwrap()
}

#[test]
fn parcel_constructs_from_r2_data() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: Some(RoadAddress::try_new("서울 종로구 청운동 123").unwrap()),
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::Building,
        area: AreaM2::try_new(250.0).unwrap(),
        official_land_price_per_m2: Some(MoneyKrw::try_new(5_000_000).unwrap()),
        zoning: Zoning::Residential,
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    assert_eq!(parcel.land_use_type, LandUseType::Building);
    assert_eq!(parcel.zoning, Zoning::Residential);
}

#[test]
fn parcel_optional_fields_none() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: None,
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::Forest,
        area: AreaM2::try_new(1000.0).unwrap(),
        official_land_price_per_m2: None,
        zoning: Zoning::Green,
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    assert!(parcel.road_address.is_none());
    assert!(parcel.official_land_price_per_m2.is_none());
}

#[test]
fn parcel_serde_roundtrip() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: None,
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::FactorySite,
        area: AreaM2::try_new(2500.0).unwrap(),
        official_land_price_per_m2: Some(MoneyKrw::try_new(8_000_000).unwrap()),
        zoning: Zoning::Industrial,
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    let json = serde_json::to_string(&parcel).expect("serialize");
    let back: Parcel = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parcel, back);
}

#[test]
fn parcel_clone_preserves_fields() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: None,
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::WarehouseSite,
        area: AreaM2::try_new(500.0).unwrap(),
        official_land_price_per_m2: None,
        zoning: Zoning::Industrial,
        geom: sample_polygon(),
        fetched_at: Utc::now(),
    };
    let cloned = parcel.clone();
    assert_eq!(parcel, cloned);
}
