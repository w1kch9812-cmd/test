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
    let header = raw
        .pointer("/response/header")
        .ok_or_else(|| ParseError::Malformed("missing /response/header".into()))?;
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
        // Building 엔티티 SSOT 확장 (2026-05-08): rich reader 는 panel 추가 필드 None 으로 둠.
        // 신규 필드 채우기는 SP4 후속 (FU 41+ — Cd primary mapping).
        // mgmBldrgstPk: 실 응답이 JSON number — string 변환 필요 (panel reader 의 검증된 패턴).
        mgm_bldrgst_pk: parse_id_as_string(item, "mgmBldrgstPk"),
        plat_plc: parse_optional_string(item, "platPlc"),
        building_name,
        main_purpose_code,
        structure_code,
        plat_area_m2: None,
        arch_area_m2: None,
        building_coverage_ratio: None,
        total_floor_area_m2,
        floor_area_ratio: None,
        ground_floors,
        underground_floors,
        height_m,
        passenger_elevators: None,
        emergency_elevators: None,
        indoor_self_parking: None,
        outdoor_self_parking: None,
        annex_building_count: None,
        annex_building_area_m2: None,
        permit_date: None,
        construction_start_date: None,
        use_approval_date,
        geom: Some(polygon.clone()),
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

/// ID 필드 (mgmBldrgstPk 등) — JSON number 또는 string 모두 String 으로 변환.
/// 실 응답이 number 형식 (예: `1024112777`) → docs 가이드와 다름. 둘 다 처리.
/// 빈/null/누락 → 빈 문자열 (caller 가 unwrap_or_default 패턴 가정).
fn parse_id_as_string(item: &Value, field: &str) -> String {
    match item.get(field) {
        Some(Value::String(s)) => s.trim().to_owned(),
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

/// data.go.kr 응답 → 도메인 `BuildingPurposeCode`.
///
/// FU 41 — Cd primary + CdNm fallback 하이브리드 (2026-05-04 실 API 검증).
///
/// 1. **Cd primary**: `mainPurpsCd` 5자리 표준 코드 (건축법 시행령 별표1 의 29분류) →
///    [`map_purpose_cd`]. 정부 표준이라 법령 개정 외 안 바뀜
/// 2. **CdNm fallback**: 코드 미상 / 비매핑 시 `mainPurpsCdNm` 한글 라벨 →
///    [`map_purpose_label`]. 산업 도메인 특수 분류 (지식산업센터 / 물류시설 등) 흡수
/// 3. **Other 안전망**: 둘 다 미매핑이면 `Other` (외부 schema 확장에 견고)
///
/// Fixture 기반 검증: `tests/fixtures/real_*.json` (5건, 6 케이스).
fn parse_purpose(item: &Value) -> Result<BuildingPurposeCode, ParseError> {
    // 1) Cd primary
    if let Some(cd) = item.get("mainPurpsCd").and_then(Value::as_str) {
        let trimmed = cd.trim();
        if !trimmed.is_empty() {
            if let Some(domain) = map_purpose_cd(trimmed) {
                return Ok(domain);
            }
        }
    }
    // 2) CdNm fallback (mainPurpsCdNm 누락 = Malformed)
    let label = item
        .get("mainPurpsCdNm")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.mainPurpsCdNm missing".into()))?
        .trim();
    Ok(map_purpose_label(label))
}

/// `mainPurpsCd` 5자리 표준 코드 → 산업 도메인 enum (None = 비매핑 → CdNm fallback).
///
/// 건축법 시행령 별표1 의 29분류 중 산업 부동산 핵심 분류만 명시 매핑.
/// 비산업 분류 (의료/숙박/문화/종교/등) 는 `None` 반환 → 호출 측에서 한글 fallback
/// 시도 → 그것도 미매핑이면 `Other`.
fn map_purpose_cd(cd: &str) -> Option<BuildingPurposeCode> {
    match cd {
        "01000" => Some(BuildingPurposeCode::SingleHouse),
        "02000" => Some(BuildingPurposeCode::MultiHouse),
        "03000" | "04000" | "07000" => Some(BuildingPurposeCode::Retail),
        "10000" => Some(BuildingPurposeCode::Educational),
        "14000" => Some(BuildingPurposeCode::Office),
        "17000" => Some(BuildingPurposeCode::Factory),
        "18000" => Some(BuildingPurposeCode::Warehouse),
        _ => None,
    }
}

/// `mainPurpsCdNm` 한글 라벨 → 산업 도메인 enum (fallback).
///
/// Cd primary 매핑 외 케이스 흡수:
/// - 산업 특수 분류 (지식산업센터 / 물류시설) — 별도 mainPurpsCd 없음
/// - 행정 표기 변형 ("아파트", "사무소" 등)
/// - 미매핑 → `Other`
fn map_purpose_label(label: &str) -> BuildingPurposeCode {
    match label {
        "단독주택" => BuildingPurposeCode::SingleHouse,
        "공동주택" | "다세대주택" | "다가구주택" | "아파트" | "연립주택" => {
            BuildingPurposeCode::MultiHouse
        }
        "공장" => BuildingPurposeCode::Factory,
        "창고" | "창고시설" => BuildingPurposeCode::Warehouse,
        "업무시설" | "사무소" => BuildingPurposeCode::Office,
        "제1종근린생활시설" | "제2종근린생활시설" | "근린생활시설" | "판매시설" => {
            BuildingPurposeCode::Retail
        }
        "지식산업센터" | "지식산업센터(아파트형공장)" | "아파트형공장" => {
            BuildingPurposeCode::KnowledgeIndustryCenter
        }
        "물류시설" | "물류창고" | "물류터미널" => BuildingPurposeCode::LogisticsCenter,
        "교육연구시설" => BuildingPurposeCode::Educational,
        _ => BuildingPurposeCode::Other,
    }
}

/// data.go.kr 응답 → 도메인 `BuildingStructureCode`.
///
/// FU 41 — Cd primary + CdNm fallback (`parse_purpose` 와 동일 정책).
fn parse_structure(item: &Value) -> Result<BuildingStructureCode, ParseError> {
    // 1) Cd primary
    if let Some(cd) = item.get("strctCd").and_then(Value::as_str) {
        let trimmed = cd.trim();
        if !trimmed.is_empty() {
            if let Some(domain) = map_structure_cd(trimmed) {
                return Ok(domain);
            }
        }
    }
    // 2) CdNm fallback
    let label = item
        .get("strctCdNm")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("item.strctCdNm missing".into()))?
        .trim();
    Ok(map_structure_label(label))
}

/// `strctCd` 2자리 표준 코드 → 도메인 enum.
///
/// 검증된 코드만 명시 매핑 (실 API 5건 fixture). 미검증 코드는 `None` →
/// 한글 fallback 안전망. 미매핑 표기 변형 은 `map_structure_label` 가 흡수.
fn map_structure_cd(cd: &str) -> Option<BuildingStructureCode> {
    match cd {
        "11" => Some(BuildingStructureCode::Brick),
        "21" => Some(BuildingStructureCode::ReinforcedConcrete),
        "42" => Some(BuildingStructureCode::SteelReinforcedConcrete),
        _ => None,
    }
}

/// `strctCdNm` 한글 라벨 → 도메인 enum (fallback).
fn map_structure_label(label: &str) -> BuildingStructureCode {
    match label {
        "철근콘크리트구조" | "철근콘크리트" => {
            BuildingStructureCode::ReinforcedConcrete
        }
        "철골구조" | "철골" => BuildingStructureCode::Steel,
        "철골철근콘크리트구조" | "철골철근콘크리트" | "SRC구조" => {
            BuildingStructureCode::SteelReinforcedConcrete
        }
        "벽돌구조" | "벽돌조" => BuildingStructureCode::Brick,
        "블록구조" | "블록조" => BuildingStructureCode::Block,
        "목구조" | "목조" => BuildingStructureCode::Wood,
        "경량철골구조" | "경량철골조" => BuildingStructureCode::LightSteel,
        _ => BuildingStructureCode::Other,
    }
}

/// `totArea` → f64 → `AreaM2`. 0 이하 / NaN 모두 도메인 거부.
///
/// 실 API 응답이 number 또는 string 둘 다 가능 — JSON spec 의 `Number` 타입 직접 가능 +
/// 일부 정부 endpoint 는 `_type=json` 이어도 정수만 string 으로 wrap. 둘 다 처리.
fn parse_total_area(item: &Value) -> Result<AreaM2, ParseError> {
    let value = read_f64_field(item, "totArea")?
        .ok_or_else(|| ParseError::Malformed("item.totArea missing or empty".into()))?;
    AreaM2::try_new(value).map_err(|e| ParseError::Domain(format!("totArea: {e}")))
}

/// `grndFlrCnt` / `ugrndFlrCnt` → u8. 음수 / 비숫자 → Domain error. 빈/누락 → 0 fallback.
///
/// 실 API 가 number (`45`) 또는 string (`"45"`) 둘 다 보냄 — 둘 다 처리.
fn parse_floor_count(item: &Value, field: &str) -> Result<u8, ParseError> {
    let Some(node) = item.get(field) else {
        return Ok(0);
    };
    match node {
        Value::Null => Ok(0),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(0)
            } else {
                trimmed
                    .parse::<u8>()
                    .map_err(|e| ParseError::Domain(format!("{field} '{s}' not u8: {e}")))
            }
        }
        Value::Number(n) => n
            .as_u64()
            .and_then(|v| u8::try_from(v).ok())
            .ok_or_else(|| ParseError::Domain(format!("{field} '{n}' not u8"))),
        other => Err(ParseError::Domain(format!(
            "{field} unexpected type: {other:?}"
        ))),
    }
}

/// `heit` → f64. "0" / 비숫자 / 빈 → None (선택 필드). number/string 둘 다 처리.
fn parse_optional_height(item: &Value) -> Option<f64> {
    let value = read_f64_field(item, "heit").ok().flatten()?;
    if !value.is_finite() || value <= 0.0 {
        None
    } else {
        Some(value)
    }
}

/// 정부 API 가 number 또는 string 둘 다로 보내는 숫자 필드 → f64.
///
/// - `Value::Number` → `as_f64()`
/// - `Value::String` → `parse::<f64>()` (빈 문자열 → `Ok(None)`)
/// - `Value::Null` / 누락 → `Ok(None)`
/// - 그 외 타입 → `ParseError::Domain`
///
/// 호출 측이 누락 vs 0 vs invalid 분기 결정. 본 함수는 unwrap 안 함.
fn read_f64_field(item: &Value, field: &str) -> Result<Option<f64>, ParseError> {
    let Some(node) = item.get(field) else {
        return Ok(None);
    };
    match node {
        Value::Null => Ok(None),
        Value::Number(n) => Ok(n.as_f64()),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                trimmed
                    .parse::<f64>()
                    .map(Some)
                    .map_err(|e| ParseError::Domain(format!("{field} '{s}' not f64: {e}")))
            }
        }
        other => Err(ParseError::Domain(format!(
            "{field} unexpected type: {other:?}"
        ))),
    }
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
        let buildings =
            parse_building_title(&raw, &sample_pnu(), &sample_polygon(), Utc::now()).expect("ok");
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
}
