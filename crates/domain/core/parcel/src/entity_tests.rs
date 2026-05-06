//! `Parcel` Aggregate 테스트.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_cmp,
    clippy::unreadable_literal
)]

use super::{GosiYearMonth, Parcel};
use chrono::Utc;
use geo_types::{Coord, LineString, MultiPolygon as GeoMultiPolygon, Polygon as GeoPolygon};
use shared_kernel::address::{JibunAddress, RoadAddress};
use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::MultiPolygonSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::zoning::Zoning;

fn sample_multi_polygon() -> MultiPolygonSrid {
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    let polygon = GeoPolygon::new(exterior, vec![]);
    MultiPolygonSrid::try_new_wgs84(GeoMultiPolygon(vec![polygon])).expect("valid")
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
fn parcel_constructs_with_all_fields_present() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: Some(RoadAddress::try_new("서울 종로구 청운동 123").unwrap()),
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::Building,
        area: Some(AreaM2::try_new(250.0).unwrap()),
        official_land_price_per_m2: Some(MoneyKrw::try_new(5_000_000).unwrap()),
        gosi_year_month: Some(GosiYearMonth {
            year: 2025,
            month: 1,
        }),
        zoning: Some(Zoning::Residential),
        geom: sample_multi_polygon(),
        fetched_at: Utc::now(),
    };
    assert_eq!(parcel.land_use_type, LandUseType::Building);
    assert_eq!(parcel.zoning, Some(Zoning::Residential));
    assert_eq!(parcel.area.unwrap().as_f64(), 250.0);
    assert_eq!(parcel.gosi_year_month.unwrap().year, 2025);
}

#[test]
fn parcel_optional_fields_none() {
    // V-World LP_PA_CBND_BUBUN 단독 응답 시뮬레이션 — area/zoning은 None.
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: None,
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::Forest,
        area: None,
        official_land_price_per_m2: None,
        gosi_year_month: None,
        zoning: None,
        geom: sample_multi_polygon(),
        fetched_at: Utc::now(),
    };
    assert!(parcel.road_address.is_none());
    assert!(parcel.area.is_none());
    assert!(parcel.zoning.is_none());
    assert!(parcel.official_land_price_per_m2.is_none());
    assert!(parcel.gosi_year_month.is_none());
}

#[test]
fn parcel_serde_roundtrip() {
    let parcel = Parcel {
        pnu: Pnu::try_new("1111010100100010000").unwrap(),
        admin: sample_admin(),
        road_address: None,
        jibun_address: JibunAddress::try_new("서울 종로구 청운동 1-1").unwrap(),
        land_use_type: LandUseType::FactorySite,
        area: Some(AreaM2::try_new(2500.0).unwrap()),
        official_land_price_per_m2: Some(MoneyKrw::try_new(8_000_000).unwrap()),
        gosi_year_month: Some(GosiYearMonth {
            year: 2026,
            month: 1,
        }),
        zoning: Some(Zoning::Industrial),
        geom: sample_multi_polygon(),
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
        area: Some(AreaM2::try_new(500.0).unwrap()),
        official_land_price_per_m2: None,
        gosi_year_month: None,
        zoning: Some(Zoning::Industrial),
        geom: sample_multi_polygon(),
        fetched_at: Utc::now(),
    };
    let cloned = parcel.clone();
    assert_eq!(parcel, cloned);
}
