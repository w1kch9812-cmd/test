//! data.go.kr `getBrTitleInfo` JSON → 도메인 `Vec<Building>` 변환 (ACL).
//!
//! 응답 구조 (`docs/data-sources/data-go-kr.md` § 응답 예시):
//! ```json
//! {
//!   "response": {
//!     "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
//!     "body": {
//!       "items": {
//!         "item": [ { "bldNm": "○○동", "mainPurpsCdNm": "공장", ... } ]
//!       },
//!       "totalCount": "1",
//!       "pageNo": "1",
//!       "numOfRows": "100"
//!     }
//!   }
//! }
//! ```
//!
//! 본 모듈은 위 raw JSON 을 도메인 [`Building`] 으로 변환. 외부 스키마가
//! 도메인에 누출되지 않도록 (Anti-Corruption Layer).
//!
//! 매핑:
//! - `bldNm` → `building_name` (Option, 빈 문자열 → None)
//! - `mainPurpsCdNm` 한글 → [`BuildingPurposeCode`] (factory/warehouse/...)
//! - `strctCdNm` 한글 → [`BuildingStructureCode`] (steel/concrete/...)
//! - `totArea` 문자열 → f64 → [`AreaM2`]
//! - `grndFlrCnt`/`ugrndFlrCnt` 문자열 → u8
//! - `heit` 문자열 → f64 (Option, "0"/빈 → None)
//! - `useAprDay` `YYYYMMDD` → [`NaiveDate`] (Option, 8자리 아니면 None)
//! - `geom`: data.go.kr 응답에 없음 → 호출 측이 V-World 폴리곤 주입
//!
//! [`Building`]: building_domain::entity::Building
//! [`BuildingPurposeCode`]: building_domain::purpose_code::BuildingPurposeCode
//! [`BuildingStructureCode`]: building_domain::structure_code::BuildingStructureCode
//! [`AreaM2`]: shared_kernel::area::AreaM2
//! [`NaiveDate`]: chrono::NaiveDate

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use building_domain::entity::Building;
use building_domain::purpose_code::BuildingPurposeCode;
use building_domain::structure_code::BuildingStructureCode;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::Value;
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;

use crate::error::ParseError;

/// `getBrTitleInfo` 응답 → `Vec<Building>`.
///
/// `polygon` 은 호출 측 (`DataGoKrBuildingReader`) 이 V-World 에서 받아 주입 —
/// data.go.kr 응답에 폴리곤 없음 (spec § 3.3). 모든 building 이 같은 PNU 의
/// 동일 필지 폴리곤을 공유 (FU 40 까지 approximation).
///
/// 빈 `items` → `Ok(vec![])`. 단일 건물은 객체, 다수는 배열 — `serde_json::Value`
/// 다형 처리.
///
/// # Errors
///
/// - `resultCode != "00"` → [`ParseError::ApiError`]
/// - JSON 구조 mismatch → [`ParseError::Malformed`]
/// - 필수 필드 파싱/도메인 invariant 실패 → [`ParseError::Domain`]
pub fn parse_building_title(
    raw: &Value,
    pnu: &Pnu,
    polygon: &PolygonSrid,
    fetched_at: DateTime<Utc>,
) -> Result<Vec<Building>, ParseError> {
    // 1) header.resultCode 검증 — "00" 외 모두 ApiError.
    let header = raw.pointer("/response/header").ok_or_else(|| {
        ParseError::Malformed("missing /response/header".into())
    })?;
    let result_code = header
        .get("resultCode")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("header.resultCode missing".into()))?;
    if result_code != "00" {
        let msg = header
            .get("resultMsg")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        return Err(ParseError::ApiError {
            code: result_code.to_owned(),
            msg,
        });
    }

    // 2) body.items.item 추출 — 단일 객체 / 배열 / 빈 / 누락 처리.
    let items_node = raw.pointer("/response/body/items");
    let item_node = match items_node {
        // items 가 빈 문자열 ("") 인 경우 — data.go.kr 가 결과 0 일 때 자주 보냄.
        Some(Value::String(s)) if s.is_empty() => return Ok(vec![]),
        // items 가 null / 아예 없음 — 결과 0 으로 간주.
        None | Some(Value::Null) => return Ok(vec![]),
        Some(items) => items.get("item"),
    };

    let raw_items: Vec<&Value> = match item_node {
        Some(Value::Array(arr)) => arr.iter().collect(),
        Some(obj @ Value::Object(_)) => vec![obj],
        // item 이 없거나 null → 빈 배열.
        Some(Value::Null) | None => return Ok(vec![]),
        Some(other) => {
            return Err(ParseError::Malformed(format!(
                "body.items.item unexpected type: {other:?}"
            )));
        }
    };

    let mut buildings = Vec::with_capacity(raw_items.len());
    for item in raw_items {
        buildings.push(parse_single(item, pnu, polygon, fetched_at)?);
    }
    Ok(buildings)
}

fn parse_single(
    item: &Value,
    pnu: &Pnu,
    polygon: &PolygonSrid,
    fetched_at: DateTime<Utc>,
) -> Result<Building, ParseError> {
    let building_name = parse_optional_string(item, "bldNm");
    let main_purpose_code = parse_purpose(item)?;
    let structure_code = parse_structure(item)?;
    let total_floor_area_m2 = parse_total_area(item)?;
    let ground_floors = parse_floor_count(item, "grndFlrCnt")?;
    let underground_floors = parse_floor_count(item, "ugrndFlrCnt")?;
    let height_m = parse_optional_height(item);
    let use_approval_date = parse_optional_use_apr_day(item);

    Ok(Building {
        pnu: pnu.clone(),
        building_name,
        main_purpose_code,
        structure_code,
        total_floor_area_m2,
        ground_floors,
        underground_floors,
        height_m,
        use_approval_date,
        geom: polygon.clone(),
        fetched_at,
    })
}

fn parse_optional_string(item: &Value, field: &str) -> Option<String> {
    item.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// data.go.kr `mainPurpsCdNm` (한글) → 도메인 `BuildingPurposeCode`.
///
/// FU 41 — 28+ 개의 한글 라벨 매핑표. 현재는 산업용 핵심 + 흔한 케이스만
/// 명시 매핑하고 나머지는 `Other` fallback. 명시 매핑 외 라벨이 들어와도
/// `ParseError` 가 아니라 `Other` 로 흡수 — 외부 스키마 확장에 견고.
fn parse_purpose(item: &Value) -> Result<BuildingPurposeCode, ParseError> {
    let label = item
        .get("mainPurpsCdNm")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.mainPurpsCdNm missing".into()))?
        .trim();
    Ok(match label {
        "단독주택" => BuildingPurposeCode::SingleHouse,
        "공동주택" | "다세대주택" | "다가구주택" | "아파트" | "연립주택" => {
            BuildingPurposeCode::MultiHouse
        }
        "공장" => BuildingPurposeCode::Factory,
        "창고" | "창고시설" => BuildingPurposeCode::Warehouse,
        "업무시설" | "사무소" => BuildingPurposeCode::Office,
        "판매시설" | "근린생활시설" => BuildingPurposeCode::Retail,
        "지식산업센터" => BuildingPurposeCode::KnowledgeIndustryCenter,
        "물류시설" | "물류창고" => BuildingPurposeCode::LogisticsCenter,
        "교육연구시설" => BuildingPurposeCode::Educational,
        _ => BuildingPurposeCode::Other,
    })
}

/// data.go.kr `strctCdNm` (한글) → 도메인 `BuildingStructureCode`.
///
/// FU 41 — 매핑표 확장. `mainPurpsCdNm` 와 동일 fallback 정책.
fn parse_structure(item: &Value) -> Result<BuildingStructureCode, ParseError> {
    let label = item
        .get("strctCdNm")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.strctCdNm missing".into()))?
        .trim();
    Ok(match label {
        "철근콘크리트구조" | "철근콘크리트" => BuildingStructureCode::ReinforcedConcrete,
        "철골구조" | "철골" => BuildingStructureCode::Steel,
        "철골철근콘크리트구조" | "철골철근콘크리트" | "SRC구조" => {
            BuildingStructureCode::SteelReinforcedConcrete
        }
        "벽돌구조" | "벽돌조" => BuildingStructureCode::Brick,
        "블록구조" | "블록조" => BuildingStructureCode::Block,
        "목구조" | "목조" => BuildingStructureCode::Wood,
        "경량철골구조" | "경량철골조" => BuildingStructureCode::LightSteel,
        _ => BuildingStructureCode::Other,
    })
}

/// `totArea` — 문자열 → f64 → `AreaM2`. 0 이하 / NaN 모두 도메인 거부.
fn parse_total_area(item: &Value) -> Result<AreaM2, ParseError> {
    let raw = item
        .get("totArea")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.totArea missing or not string".into()))?;
    let value: f64 = raw
        .trim()
        .parse()
        .map_err(|e| ParseError::Domain(format!("totArea '{raw}' not f64: {e}")))?;
    AreaM2::try_new(value).map_err(|e| ParseError::Domain(format!("totArea: {e}")))
}

/// `grndFlrCnt` / `ugrndFlrCnt` — 문자열 → u8. 음수 / 비숫자 → Domain error.
/// data.go.kr 가 종종 빈 문자열 보냄 → 0 으로 fallback.
fn parse_floor_count(item: &Value, field: &str) -> Result<u8, ParseError> {
    let Some(raw) = item.get(field).and_then(Value::as_str) else {
        return Ok(0);
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(0);
    }
    trimmed
        .parse::<u8>()
        .map_err(|e| ParseError::Domain(format!("{field} '{raw}' not u8: {e}")))
}

/// `heit` — 문자열 → f64. "0" 또는 비숫자 → None (선택 필드).
fn parse_optional_height(item: &Value) -> Option<f64> {
    let raw = item.get("heit").and_then(Value::as_str)?.trim();
    if raw.is_empty() {
        return None;
    }
    let value: f64 = raw.parse().ok()?;
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    Some(value)
}

/// `useAprDay` — `YYYYMMDD` → `NaiveDate`. 8자리 아니거나 invalid 면 None.
fn parse_optional_use_apr_day(item: &Value) -> Option<NaiveDate> {
    let raw = item.get("useAprDay").and_then(Value::as_str)?.trim();
    if raw.len() != 8 {
        return None;
    }
    NaiveDate::parse_from_str(raw, "%Y%m%d").ok()
}

#[cfg(test)]
mod tests {
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
        let buildings =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), now).expect("ok");

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
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
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
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
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
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
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
        let err =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
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
        let err =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("header")));
    }

    #[test]
    fn parse_unknown_purpose_falls_back_to_other() {
        let mut item = factory_item();
        item["mainPurpsCdNm"] = serde_json::json!("의료시설"); // 매핑표 외 라벨
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
        assert_eq!(buildings[0].main_purpose_code, BuildingPurposeCode::Other);
    }

    #[test]
    fn parse_invalid_total_area_returns_domain_error() {
        let mut item = factory_item();
        item["totArea"] = serde_json::json!("not-a-number");
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let err =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Domain(s) if s.contains("totArea")));
    }

    #[test]
    fn parse_negative_total_area_returns_domain_error() {
        let mut item = factory_item();
        item["totArea"] = serde_json::json!("-100.0");
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let err =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).unwrap_err();
        assert!(matches!(err, ParseError::Domain(_)));
    }

    #[test]
    fn parse_zero_height_becomes_none() {
        let mut item = factory_item();
        item["heit"] = serde_json::json!("0");
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
        assert!(buildings[0].height_m.is_none());
    }

    #[test]
    fn parse_invalid_use_apr_day_becomes_none() {
        let mut item = factory_item();
        item["useAprDay"] = serde_json::json!("99999999"); // invalid YYYYMMDD
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
        assert!(buildings[0].use_approval_date.is_none());
    }

    #[test]
    fn parse_empty_building_name_becomes_none() {
        let mut item = factory_item();
        item["bldNm"] = serde_json::json!("");
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
        assert!(buildings[0].building_name.is_none());
    }

    #[test]
    fn parse_empty_floor_count_becomes_zero() {
        let mut item = factory_item();
        item["ugrndFlrCnt"] = serde_json::json!("");
        let raw = ok_response(&serde_json::json!({ "item": item }));
        let buildings = parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now())
            .expect("ok");
        assert_eq!(buildings[0].underground_floors, 0);
    }
}
