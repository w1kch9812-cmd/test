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
        // Building 엔티티 SSOT — 23 필드 *모두* 채움 (panel + rich 합치 후, Codex
        // round 8 catch: "API building response loses parsed panel data" → 단일 parser
        // 가 모든 필드 수집).
        // mgmBldrgstPk: 실 응답이 JSON number — string 변환 (Codex round 7 fixture).
        mgm_bldrgst_pk: parse_id_as_string(item, "mgmBldrgstPk"),
        plat_plc: parse_optional_string(item, "platPlc"),
        building_name,
        main_purpose_code,
        structure_code,
        plat_area_m2: parse_optional_area_m2(item, "platArea"),
        arch_area_m2: parse_optional_area_m2(item, "archArea"),
        building_coverage_ratio: parse_optional_positive_f64(item, "bcRat"),
        total_floor_area_m2,
        floor_area_ratio: parse_optional_positive_f64(item, "vlRat"),
        ground_floors,
        underground_floors,
        height_m,
        passenger_elevators: parse_optional_u32(item, "rideUseElvtCnt"),
        emergency_elevators: parse_optional_u32(item, "emgenUseElvtCnt"),
        indoor_self_parking: parse_optional_u32(item, "indrAutoUtcnt"),
        outdoor_self_parking: parse_optional_u32(item, "oudrAutoUtcnt"),
        annex_building_count: parse_optional_u32(item, "atchBldCnt"),
        annex_building_area_m2: parse_optional_area_m2(item, "atchBldArea"),
        permit_date: parse_optional_yyyymmdd_date(item, "pmsDay"),
        construction_start_date: parse_optional_yyyymmdd_date(item, "stcnsDay"),
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

/// `Option<AreaM2>` — number / string 둘 다 처리, 0 이하 / NaN / 누락 → `None`.
/// `bcRat = 0` 같은 산업 매물 측면 "측정값 없음" 도 None 으로 정규화.
fn parse_optional_area_m2(item: &Value, field: &str) -> Option<AreaM2> {
    let value = read_f64_field(item, field).ok().flatten()?;
    if value <= 0.0 || !value.is_finite() {
        return None;
    }
    AreaM2::try_new(value).ok()
}

/// `Option<f64>` — 양수 + 유한 만 통과. 0 / 음수 / NaN / 빈 / 누락 → `None`.
fn parse_optional_positive_f64(item: &Value, field: &str) -> Option<f64> {
    let value = read_f64_field(item, field).ok().flatten()?;
    if value > 0.0 && value.is_finite() {
        Some(value)
    } else {
        None
    }
}

/// `Option<u32>` — number / string 둘 다 처리. 0 도 *유의미한 값* 으로 보존
/// (예: `atchBldCnt = 0` = "부속건축물 없음"). 음수 / 비숫자 / 빈 / 누락 → `None`.
fn parse_optional_u32(item: &Value, field: &str) -> Option<u32> {
    match item.get(field)? {
        Value::Number(n) => n.as_u64().and_then(|v| u32::try_from(v).ok()),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<u32>().ok()
            }
        }
        _ => None,
    }
}

/// `YYYYMMDD` 8자리 → `NaiveDate`. 빈 / 길이 mismatch / invalid date → `None`.
fn parse_optional_yyyymmdd_date(item: &Value, field: &str) -> Option<NaiveDate> {
    let raw = item.get(field).and_then(Value::as_str)?.trim();
    if raw.len() != 8 || !raw.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    NaiveDate::parse_from_str(raw, "%Y%m%d").ok()
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
mod tests;
