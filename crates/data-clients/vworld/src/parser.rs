//! V-World JSON → 도메인 `Parcel` 변환 (Anti-Corruption Layer).
//!
//! V-World WFS GetFeature 응답 구조 (`docs/data-sources/v-world.md` § 요청 예시):
//! ```json
//! {
//!   "response": {
//!     "result": {
//!       "featureCollection": {
//!         "features": [
//!           {
//!             "geometry": { "type": "Polygon", "coordinates": [[[lng,lat], ...]] },
//!             "properties": {
//!               "pnu": "1111010100100010000",
//!               "jibun": "1-1",
//!               "addr": "서울특별시 종로구 청운동",
//!               "lndcgr_nm": "대",
//!               "lndpcl_ar": 250.0,
//!               "uq_nm": "주거지역"
//!             }
//!           }
//!         ]
//!       }
//!     }
//!   }
//! }
//! ```
//!
//! 본 모듈은 위 raw JSON 을 도메인 [`Parcel`] 로 변환. 외부 스키마가 도메인에
//! 누출되지 않도록 (Anti-Corruption Layer).
//!
//! 매핑:
//! - `properties.pnu` → [`Pnu::try_new`]
//! - `geometry.coordinates[0]` (외곽 ring) → [`PolygonSrid::try_new_wgs84`]
//! - `properties.lndpcl_ar` → [`AreaM2::try_new`]
//! - `properties.lndcgr_nm` → [`LandUseType`] (한글 지목 → enum)
//! - `properties.uq_nm` → [`Zoning`] (한글 용도지역 → enum)
//! - `pnu[0..10]` → [`AdminDivision`] (시도 2 + 시군구 5 + 읍면동 8 — 단, 8자리는
//!   읍면동 8자리, 즉 `pnu[0..2] / pnu[0..5] / pnu[0..8]`)
//! - `properties.addr` → [`JibunAddress`]
//! - `road_address` → V-World 응답에 항상 포함되지 않음 → `None`
//!
//! [`Pnu::try_new`]: shared_kernel::pnu::Pnu::try_new
//! [`PolygonSrid::try_new_wgs84`]: shared_kernel::geometry::PolygonSrid::try_new_wgs84
//! [`AreaM2::try_new`]: shared_kernel::area::AreaM2::try_new
//! [`LandUseType`]: shared_kernel::land_use_type::LandUseType
//! [`Zoning`]: shared_kernel::zoning::Zoning
//! [`AdminDivision`]: shared_kernel::admin_division::AdminDivision
//! [`JibunAddress`]: shared_kernel::address::JibunAddress

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use chrono::{DateTime, Utc};
use geo_types::{Coord, LineString, Polygon as GeoPolygon};
use parcel_domain::entity::Parcel;
use serde_json::Value;
use shared_kernel::address::JibunAddress;
use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::pnu::Pnu;
use shared_kernel::zoning::Zoning;

use crate::error::ParseError;

/// V-World 응답에서 첫 번째 feature 를 도메인 [`Parcel`] 로 변환.
///
/// `Ok(None)` — featureCollection 이 비어 있을 때 (PNU 미존재).
/// `Ok(Some(parcel))` — 첫 feature 변환 성공.
/// `Err(ParseError)` — JSON 형식 깨짐 또는 도메인 invariant 위반.
pub fn parse_parcel(raw: &Value, fetched_at: DateTime<Utc>) -> Result<Option<Parcel>, ParseError> {
    let features = raw
        .pointer("/response/result/featureCollection/features")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ParseError::Malformed(
                "missing /response/result/featureCollection/features array".into(),
            )
        })?;

    let Some(first) = features.first() else {
        return Ok(None);
    };

    let props = first
        .get("properties")
        .ok_or_else(|| ParseError::Malformed("feature missing 'properties'".into()))?;

    let pnu_str = props
        .get("pnu")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("properties.pnu missing or not string".into()))?;
    let pnu = Pnu::try_new(pnu_str.trim())
        .map_err(|e| ParseError::Domain(format!("invalid pnu '{pnu_str}': {e}")))?;

    // Admin division — PNU 앞 8자리 (시도 2 + 시군구 5 + 읍면동 8 의 prefix).
    let admin = parse_admin_from_pnu(pnu_str)?;

    let jibun_address = parse_jibun_address(props)?;

    let land_use_type = parse_land_use_type(props)?;
    let zoning = parse_zoning(props)?;

    let area = parse_area(props)?;
    let polygon = parse_polygon(first)?;

    Ok(Some(Parcel {
        pnu,
        admin,
        road_address: None, // V-World 응답에 항상 없으므로 None — FU: 별도 API 필요
        jibun_address,
        land_use_type,
        area,
        official_land_price_per_m2: None, // V-World 본 응답엔 없음 (별도 레이어)
        zoning,
        geom: polygon,
        fetched_at,
    }))
}

fn parse_admin_from_pnu(pnu: &str) -> Result<AdminDivision, ParseError> {
    if pnu.len() < 10 {
        return Err(ParseError::Domain(format!(
            "pnu too short for admin division: {pnu}"
        )));
    }
    let sido = SidoCode::try_new(&pnu[0..2])
        .map_err(|e| ParseError::Domain(format!("invalid sido in pnu: {e}")))?;
    let sigungu = SigunguCode::try_new(&pnu[0..5])
        .map_err(|e| ParseError::Domain(format!("invalid sigungu in pnu: {e}")))?;
    let eupmyeondong = EupmyeondongCode::try_new(&pnu[0..8])
        .map_err(|e| ParseError::Domain(format!("invalid eupmyeondong in pnu: {e}")))?;
    AdminDivision::try_new(sido, sigungu, eupmyeondong)
        .map_err(|e| ParseError::Domain(format!("admin division: {e}")))
}

fn parse_jibun_address(props: &Value) -> Result<JibunAddress, ParseError> {
    let addr = props
        .get("addr")
        .and_then(Value::as_str)
        .or_else(|| props.get("jibun").and_then(Value::as_str))
        .ok_or_else(|| ParseError::Malformed("properties.addr / .jibun missing".into()))?;
    JibunAddress::try_new(addr).map_err(|e| ParseError::Domain(format!("jibun address: {e}")))
}

/// V-World `lndcgr_nm` (한글 지목) → 도메인 `LandUseType`.
fn parse_land_use_type(props: &Value) -> Result<LandUseType, ParseError> {
    let lndcgr = props
        .get("lndcgr_nm")
        .and_then(Value::as_str)
        .unwrap_or("기타");
    Ok(match lndcgr.trim() {
        "대" => LandUseType::Building,
        "전" => LandUseType::Field,
        "답" => LandUseType::Paddy,
        "임야" => LandUseType::Forest,
        "공장용지" => LandUseType::FactorySite,
        "창고용지" => LandUseType::WarehouseSite,
        "도로" => LandUseType::Road,
        "공원" => LandUseType::Park,
        _ => LandUseType::Other,
    })
}

/// V-World `uq_nm` (한글 용도지역) → 도메인 `Zoning` (4 대분류).
fn parse_zoning(props: &Value) -> Result<Zoning, ParseError> {
    let uq = props.get("uq_nm").and_then(Value::as_str).unwrap_or("기타");
    let trimmed = uq.trim();
    Ok(if trimmed.contains("주거") {
        Zoning::Residential
    } else if trimmed.contains("상업") {
        Zoning::Commercial
    } else if trimmed.contains("공업") {
        Zoning::Industrial
    } else if trimmed.contains("녹지") {
        Zoning::Green
    } else {
        Zoning::Other
    })
}

fn parse_area(props: &Value) -> Result<AreaM2, ParseError> {
    let area_f64 = props
        .get("lndpcl_ar")
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            ParseError::Malformed("properties.lndpcl_ar missing or not number".into())
        })?;
    AreaM2::try_new(area_f64).map_err(|e| ParseError::Domain(format!("area: {e}")))
}

fn parse_polygon(feature: &Value) -> Result<PolygonSrid, ParseError> {
    let geom = feature
        .get("geometry")
        .ok_or_else(|| ParseError::Malformed("feature missing 'geometry'".into()))?;
    let geom_type = geom
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("geometry.type missing".into()))?;
    if geom_type != "Polygon" {
        return Err(ParseError::Malformed(format!(
            "expected geometry.type 'Polygon', got '{geom_type}'"
        )));
    }
    let coords = geom
        .get("coordinates")
        .and_then(Value::as_array)
        .ok_or_else(|| ParseError::Malformed("geometry.coordinates missing or not array".into()))?;
    let outer = coords
        .first()
        .and_then(Value::as_array)
        .ok_or_else(|| ParseError::Malformed("geometry.coordinates[0] missing".into()))?;

    let mut points: Vec<Coord<f64>> = Vec::with_capacity(outer.len());
    for pair in outer {
        let pair_arr = pair
            .as_array()
            .ok_or_else(|| ParseError::Malformed("coordinate pair not array".into()))?;
        let lng = pair_arr
            .first()
            .and_then(Value::as_f64)
            .ok_or_else(|| ParseError::Malformed("lng not f64".into()))?;
        let lat = pair_arr
            .get(1)
            .and_then(Value::as_f64)
            .ok_or_else(|| ParseError::Malformed("lat not f64".into()))?;
        points.push(Coord { x: lng, y: lat });
    }
    if points.len() < 4 {
        return Err(ParseError::Malformed(format!(
            "polygon ring needs ≥4 points (got {})",
            points.len()
        )));
    }
    let ring = LineString(points);
    let polygon = GeoPolygon::new(ring, vec![]);
    PolygonSrid::try_new_wgs84(polygon).map_err(|e| ParseError::Domain(format!("polygon: {e}")))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    fn sample_response() -> Value {
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
                                    "lndcgr_nm": "대",
                                    "lndpcl_ar": 250.5,
                                    "uq_nm": "제2종일반주거지역"
                                }
                            }
                        ]
                    }
                }
            }
        })
    }

    fn empty_response() -> Value {
        serde_json::json!({
            "response": {
                "result": {
                    "featureCollection": { "features": [] }
                }
            }
        })
    }

    #[test]
    fn parse_valid_parcel_json() {
        let raw = sample_response();
        let now = Utc::now();
        let parcel = parse_parcel(&raw, now).expect("ok").expect("Some parcel");

        assert_eq!(parcel.pnu.as_str(), "1111010100100010000");
        assert_eq!(parcel.land_use_type, LandUseType::Building);
        assert_eq!(parcel.zoning, Zoning::Residential);
        assert!((parcel.area.as_f64() - 250.5).abs() < 0.01);
        assert_eq!(parcel.admin.sido.as_str(), "11");
        assert_eq!(parcel.fetched_at, now);
        assert_eq!(
            parcel.jibun_address.as_str(),
            "서울특별시 종로구 청운동 1-1"
        );
        assert!(parcel.road_address.is_none());
    }

    #[test]
    fn parse_empty_feature_collection_returns_none() {
        let raw = empty_response();
        let result = parse_parcel(&raw, Utc::now()).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn parse_missing_pnu_returns_error() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["properties"]
            .as_object_mut()
            .unwrap()
            .remove("pnu");
        let err = parse_parcel(&raw, Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("pnu")));
    }

    #[test]
    fn parse_invalid_pnu_returns_domain_error() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["properties"]["pnu"] =
            serde_json::json!("INVALID");
        let err = parse_parcel(&raw, Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Domain(_)));
    }

    #[test]
    fn parse_malformed_geometry_returns_error() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["geometry"]["type"] =
            serde_json::json!("LineString");
        let err = parse_parcel(&raw, Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("Polygon")));
    }

    #[test]
    fn parse_industrial_zoning() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["properties"]["uq_nm"] =
            serde_json::json!("일반공업지역");
        let parcel = parse_parcel(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.zoning, Zoning::Industrial);
    }

    #[test]
    fn parse_factory_site_land_use() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["properties"]["lndcgr_nm"] =
            serde_json::json!("공장용지");
        let parcel = parse_parcel(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.land_use_type, LandUseType::FactorySite);
    }

    #[test]
    fn parse_unknown_zoning_falls_back_to_other() {
        let mut raw = sample_response();
        raw["response"]["result"]["featureCollection"]["features"][0]["properties"]["uq_nm"] =
            serde_json::json!("자연환경보전지역");
        let parcel = parse_parcel(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.zoning, Zoning::Other);
    }
}
