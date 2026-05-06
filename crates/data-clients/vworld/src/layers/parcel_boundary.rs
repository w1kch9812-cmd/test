//! V-World `LP_PA_CBND_BUBUN` (연속지적도) 레이어 파서.
//!
//! 레이어 ID: `LP_PA_CBND_BUBUN` — PNU 기반 단일 필지 조회의 SSOT.
//!
//! 실 응답 properties (확인됨 2026-05-06):
//! ```json
//! {
//!   "pnu": "1168010100107370000",
//!   "jibun": "737 대",
//!   "bonbun": "737",
//!   "bubun": "",
//!   "addr": "서울특별시 강남구 역삼동 737",
//!   "jiga": "67300000",
//!   "gosi_year": "2025",
//!   "gosi_month": "01"
//! }
//! ```
//!
//! 면적/용도지역 **없음** — 이 레이어 자체가 제공 X. 그래서 [`Parcel.area`]와
//! [`Parcel.zoning`] 은 `Option`이며 본 파서가 항상 `None`으로 둠. 호출자가
//! 별도 호출(LT_C_UQ111 spatial intersect / 건축물대장 / PostGIS area)로 보강.
//!
//! [`Parcel.area`]: parcel_domain::entity::Parcel::area
//! [`Parcel.zoning`]: parcel_domain::entity::Parcel::zoning

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use chrono::{DateTime, Utc};
use parcel_domain::entity::{GosiYearMonth, Parcel};
use serde_json::Value;
use shared_kernel::address::JibunAddress;
use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;

use crate::envelope::{self, Outcome};
use crate::error::ParseError;
use crate::geometry::parse_geometry;

/// V-World `LP_PA_CBND_BUBUN` WFS GetFeature 응답 → 도메인 [`Parcel`] 변환.
///
/// 처리 순서:
/// 1. [`envelope::parse`] — status/error 분기 (NOT_FOUND → `Ok(None)`,
///    ERROR → [`ParseError::VWorldApi`])
/// 2. 첫 feature 추출 (다중 결과는 V-World가 PNU exact match면 1개만 반환)
/// 3. properties → `Parcel` 필드 매핑 (이 레이어가 제공하는 것만, 나머지 None)
///
/// # Errors
///
/// - envelope 단계 에러 → 그대로 전파
/// - properties 누락/형식 → [`ParseError::Malformed`]
/// - 도메인 invariant 위반 (PNU/주소/좌표) → [`ParseError::Domain`]
pub fn parse_parcel_boundary(
    raw: &Value,
    fetched_at: DateTime<Utc>,
) -> Result<Option<Parcel>, ParseError> {
    let features = match envelope::parse(raw)? {
        Outcome::Features(f) => f,
        Outcome::NotFound => return Ok(None),
    };

    let Some(first) = features.first() else {
        return Ok(None);
    };

    let props = first
        .get("properties")
        .ok_or_else(|| ParseError::Malformed("feature missing 'properties'".into()))?;
    let geometry = first
        .get("geometry")
        .ok_or_else(|| ParseError::Malformed("feature missing 'geometry'".into()))?;

    let pnu = parse_pnu(props)?;
    let admin = parse_admin_from_pnu(pnu.as_str())?;
    let jibun_address = parse_jibun_address(props)?;
    let land_use_type = parse_land_use_type_from_jibun(props);
    let (price, gosi) = parse_jiga(props)?;
    let geom = parse_geometry(geometry)?;

    Ok(Some(Parcel {
        pnu,
        admin,
        // 도로명: V-World 본 레이어 미제공 — 별도 API (NSDI/주소API).
        road_address: None,
        jibun_address,
        land_use_type,
        // 면적: 본 레이어 미제공. PostGIS 계산 또는 별도 호출이 채움.
        area: None,
        official_land_price_per_m2: price,
        gosi_year_month: gosi,
        // 용도지역: LT_C_UQ111 spatial intersect 별도 호출 필요.
        zoning: None,
        geom,
        fetched_at,
    }))
}

fn parse_pnu(props: &Value) -> Result<Pnu, ParseError> {
    let s = props
        .get("pnu")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("properties.pnu missing or not string".into()))?;
    Pnu::try_new(s.trim()).map_err(|e| ParseError::Domain(format!("invalid pnu '{s}': {e}")))
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
    // `addr` 가 풀주소 ("서울특별시 강남구 역삼동 737"), `jibun`은 지번+지목 ("737 대").
    let addr = props
        .get("addr")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("properties.addr missing".into()))?;
    JibunAddress::try_new(addr).map_err(|e| ParseError::Domain(format!("jibun address: {e}")))
}

/// `jibun` 마지막 토큰("737 대"의 "대") → [`LandUseType`].
///
/// V-World는 별도 `lndcgr_nm` 필드를 본 레이어에서 제공하지 않음 — `jibun` 한
/// 토큰만 있어 거기서 추출. 토큰이 없거나 매핑 불가면 `Other` (Honest fallback).
fn parse_land_use_type_from_jibun(props: &Value) -> LandUseType {
    let jibun = props.get("jibun").and_then(Value::as_str).unwrap_or("");
    let token = jibun.split_whitespace().next_back().unwrap_or("");
    match token.trim() {
        "대" => LandUseType::Building,
        "전" => LandUseType::Field,
        "답" => LandUseType::Paddy,
        "임야" | "임" => LandUseType::Forest,
        "공장용지" | "장" => LandUseType::FactorySite,
        "창고용지" | "창" => LandUseType::WarehouseSite,
        "도로" | "도" => LandUseType::Road,
        "공원" => LandUseType::Park,
        _ => LandUseType::Other,
    }
}

/// `jiga` (₩/m²) + `gosi_year`/`gosi_month` 묶어서 반환.
///
/// `jiga` 가 "0" 이거나 누락이면 `(None, None)` — 미고시 필지 (도로 등).
fn parse_jiga(
    props: &Value,
) -> Result<(Option<MoneyKrw>, Option<GosiYearMonth>), ParseError> {
    let jiga_str = props.get("jiga").and_then(Value::as_str).unwrap_or("");
    let jiga_num: i64 = jiga_str.trim().parse().unwrap_or(0);
    if jiga_num <= 0 {
        return Ok((None, None));
    }
    let price = MoneyKrw::try_new(jiga_num)
        .map_err(|e| ParseError::Domain(format!("jiga: {e}")))?;
    let gosi = parse_gosi_year_month(props)?;
    Ok((Some(price), Some(gosi)))
}

fn parse_gosi_year_month(props: &Value) -> Result<GosiYearMonth, ParseError> {
    let year_str = props
        .get("gosi_year")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("gosi_year missing (jiga present)".into()))?;
    let month_str = props
        .get("gosi_month")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("gosi_month missing (jiga present)".into()))?;
    let year: u16 = year_str
        .parse()
        .map_err(|e| ParseError::Domain(format!("gosi_year '{year_str}': {e}")))?;
    let month: u8 = month_str
        .parse()
        .map_err(|e| ParseError::Domain(format!("gosi_month '{month_str}': {e}")))?;
    if !(1..=12).contains(&month) {
        return Err(ParseError::Domain(format!(
            "gosi_month out of range: {month}"
        )));
    }
    Ok(GosiYearMonth { year, month })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;
    use shared_kernel::zoning::Zoning;
    use std::path::PathBuf;

    fn load_fixture(name: &str) -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
        serde_json::from_str(&raw).expect("valid JSON fixture")
    }

    // ── Real fixtures (recorded 2026-05-06) ────────────────────────

    #[test]
    fn real_gangnam_yeoksam_737_parses_correctly() {
        let raw = load_fixture("real_parcel_boundary_gangnam_yeoksam_737.json");
        let now = Utc::now();
        let parcel = parse_parcel_boundary(&raw, now).expect("ok").expect("Some");

        assert_eq!(parcel.pnu.as_str(), "1168010100107370000");
        assert_eq!(
            parcel.jibun_address.as_str(),
            "서울특별시 강남구 역삼동 737"
        );
        assert_eq!(parcel.land_use_type, LandUseType::Building); // jibun "737 대"
        assert_eq!(parcel.admin.sido.as_str(), "11");
        assert_eq!(parcel.admin.sigungu.as_str(), "11680");
        assert_eq!(parcel.admin.eupmyeondong.as_str(), "11680101");
        assert_eq!(
            parcel.official_land_price_per_m2,
            Some(MoneyKrw::try_new(67_300_000).unwrap())
        );
        let gosi = parcel.gosi_year_month.expect("Some");
        assert_eq!(gosi.year, 2025);
        assert_eq!(gosi.month, 1);
        // 본 레이어가 제공 안 하는 필드는 None — invariants:
        assert!(parcel.area.is_none());
        assert!(parcel.zoning.is_none());
        assert!(parcel.road_address.is_none());
        // geom: MultiPolygon, single member.
        assert_eq!(parcel.geom.polygon_count(), 1);
        assert_eq!(parcel.fetched_at, now);
    }

    #[test]
    fn real_jongno_cheongun_parses_correctly() {
        let raw = load_fixture("real_parcel_boundary_jongno_cheongun.json");
        let parcel = parse_parcel_boundary(&raw, Utc::now())
            .expect("ok")
            .expect("Some");
        assert_eq!(parcel.pnu.as_str(), "1111010100100010000");
        assert_eq!(parcel.admin.sido.as_str(), "11");
    }

    #[test]
    fn real_not_found_returns_none() {
        // status: NOT_FOUND, no `result` field.
        let raw = load_fixture("real_parcel_boundary_not_found.json");
        let result = parse_parcel_boundary(&raw, Utc::now()).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn real_error_envelope_returns_vworld_api_error() {
        // status: ERROR, error.code = INVALID_RANGE.
        let raw = load_fixture("real_error_invalid_range.json");
        let err = parse_parcel_boundary(&raw, Utc::now()).unwrap_err();
        match err {
            ParseError::VWorldApi { code, text } => {
                assert_eq!(code, "INVALID_RANGE");
                assert!(text.contains("attrFilter"));
            }
            other => panic!("expected VWorldApi, got {other:?}"),
        }
    }

    // ── Domain invariant 검증 (synthetic edge cases) ──────────────

    #[test]
    fn rejects_invalid_pnu_format() {
        // 19자리 미만 PNU.
        let raw = serde_json::json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [{
                    "geometry": { "type": "Polygon", "coordinates": [[
                        [126.0, 37.0], [127.0, 37.0], [127.0, 38.0], [126.0, 37.0]
                    ]]},
                    "properties": { "pnu": "INVALID", "addr": "서울 종로구 청운동", "jibun": "1 대" }
                }]}}
            }
        });
        let err = parse_parcel_boundary(&raw, Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Domain(_)));
    }

    #[test]
    fn jibun_factory_token_maps_to_factory_site() {
        let raw = serde_json::json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [{
                    "geometry": { "type": "Polygon", "coordinates": [[
                        [126.0, 37.0], [127.0, 37.0], [127.0, 38.0], [126.0, 37.0]
                    ]]},
                    "properties": {
                        "pnu": "1111010100100010000",
                        "addr": "테스트 주소",
                        "jibun": "100 공장용지"
                    }
                }]}}
            }
        });
        let parcel = parse_parcel_boundary(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.land_use_type, LandUseType::FactorySite);
    }

    #[test]
    fn unknown_jibun_token_falls_back_to_other() {
        let raw = serde_json::json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [{
                    "geometry": { "type": "Polygon", "coordinates": [[
                        [126.0, 37.0], [127.0, 37.0], [127.0, 38.0], [126.0, 37.0]
                    ]]},
                    "properties": {
                        "pnu": "1111010100100010000",
                        "addr": "테스트",
                        "jibun": "100 미지"
                    }
                }]}}
            }
        });
        let parcel = parse_parcel_boundary(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.land_use_type, LandUseType::Other);
    }

    #[test]
    fn jiga_zero_yields_no_price_no_gosi() {
        let raw = serde_json::json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [{
                    "geometry": { "type": "Polygon", "coordinates": [[
                        [126.0, 37.0], [127.0, 37.0], [127.0, 38.0], [126.0, 37.0]
                    ]]},
                    "properties": {
                        "pnu": "1111010100100010000",
                        "addr": "테스트",
                        "jibun": "100 도로",
                        "jiga": "0"
                    }
                }]}}
            }
        });
        let parcel = parse_parcel_boundary(&raw, Utc::now()).unwrap().unwrap();
        assert!(parcel.official_land_price_per_m2.is_none());
        assert!(parcel.gosi_year_month.is_none());
        assert_eq!(parcel.land_use_type, LandUseType::Road);
    }

    #[test]
    fn empty_features_returns_none() {
        let raw = serde_json::json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [] } }
            }
        });
        let result = parse_parcel_boundary(&raw, Utc::now()).expect("ok");
        assert!(result.is_none());
    }

    #[test]
    fn zoning_from_boundary_layer_is_always_none() {
        // 본 레이어는 zoning 미제공 — 별도 호출 필요. 명시적으로 None 보장.
        let raw = load_fixture("real_parcel_boundary_gangnam_yeoksam_737.json");
        let parcel = parse_parcel_boundary(&raw, Utc::now()).unwrap().unwrap();
        assert_eq!(parcel.zoning, None::<Zoning>);
    }
}
