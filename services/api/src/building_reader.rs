//! `DataGoKrBuildingRegisterReader` — `routes::buildings::BuildingRegisterReader` 의
//! data.go.kr 라이브 구현체.
//!
//! `getBrTitleInfo` JSON 응답 → `Vec<BuildingItem>` (api 로컬 좁은 shape).
//!
//! `crates/data-clients/data-go-kr` 의 `DataGoKrBuildingReader` 와 다름 — 그쪽은
//! `building_domain::reader::BuildingReader` (rich `Building` entity + V-World 폴리곤
//! 합성) 을 구현하지만, 본 reader 는 *panel 응답용 좁은 subset* 만 채움. 라이브 시
//! `building_domain` 풀체인 (FU 40 `R2` `PMTiles`) 은 별도 도입.

use std::sync::Arc;

use chrono::Utc;
use data_go_kr_client::{building_register::BuildingRegisterClient, pnu_split, DataGoKrClient};
use raw_capture_client::RawCapture;
use serde_json::Value;
use shared_kernel::pnu::Pnu;
use tracing::warn;

use crate::routes::buildings::{BuildingItem, BuildingRegisterError, BuildingRegisterReader};

/// `parcel_external_data.source` CHECK 의 라벨. `migrations/30006_parcel_external_data.sql:13-19`
/// 의 enum-like 제약 (`data_go_kr_building`) 과 정확히 일치.
const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";

/// `BuildingRegisterReader` 의 data.go.kr 라이브 구현체.
///
/// `getBrTitleInfo` raw JSON 을 `RawCapture` 로 best-effort 보존 — `parcel_external_data`
/// (pnu, `data_go_kr_building`) UPSERT. 보존 실패는 warn 로그 + 응답 정상 진행 (SSOT 보호).
pub struct DataGoKrBuildingRegisterReader {
    client: Arc<DataGoKrClient>,
    raw_capture: Arc<dyn RawCapture>,
}

impl DataGoKrBuildingRegisterReader {
    /// 새 [`DataGoKrBuildingRegisterReader`].
    #[must_use]
    pub const fn new(client: Arc<DataGoKrClient>, raw_capture: Arc<dyn RawCapture>) -> Self {
        Self {
            client,
            raw_capture,
        }
    }
}

impl BuildingRegisterReader for DataGoKrBuildingRegisterReader {
    fn list_by_pnu<'a>(
        &'a self,
        pnu: &'a Pnu,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<BuildingItem>, BuildingRegisterError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let parts = pnu_split::split(pnu);
            let br = BuildingRegisterClient::new(&self.client);
            let raw = br
                .fetch_title_info(parts)
                .await
                .map_err(|e| Box::new(e) as BuildingRegisterError)?;

            // raw_capture best-effort — 보존 실패는 warn 후 정상 진행 (응답 자체는 OK).
            // AGENTS.md § 3 "raw 응답 보존" + audit 2026-05-08 round 2 (P2 ship-safety fix).
            if let Err(capture_err) = self
                .raw_capture
                .capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, Utc::now())
                .await
            {
                warn!(
                    pnu = %pnu.as_str(),
                    source = RAW_CAPTURE_SOURCE,
                    error = %capture_err,
                    "raw_capture failed — proceeding with parsed result"
                );
            }

            parse_items(&raw)
        })
    }
}

fn parse_items(raw: &Value) -> Result<Vec<BuildingItem>, BuildingRegisterError> {
    // header.resultCode 검증 — "00" 외 모두 ApiError.
    let result_code = raw
        .pointer("/response/header/resultCode")
        .and_then(Value::as_str)
        .ok_or("missing /response/header/resultCode")?;
    if result_code != "00" {
        let msg = raw
            .pointer("/response/header/resultMsg")
            .and_then(Value::as_str)
            .unwrap_or("");
        return Err(format!("data.go.kr resultCode={result_code} resultMsg={msg}").into());
    }

    // body.items: data.go.kr 가 결과 0 일 때 빈 문자열 / null / 누락 다양 — 모두 빈 vec.
    let items_node = raw.pointer("/response/body/items");
    let item_node = match items_node {
        Some(Value::String(s)) if s.is_empty() => return Ok(vec![]),
        None | Some(Value::Null) => return Ok(vec![]),
        Some(items) => items.get("item"),
    };

    // body.items.item: 단일 객체 / 배열 / 빈 / 누락 다형 처리.
    let raw_items: Vec<&Value> = match item_node {
        Some(Value::Array(arr)) => arr.iter().collect(),
        Some(obj @ Value::Object(_)) => vec![obj],
        Some(Value::Null) | None => return Ok(vec![]),
        Some(other) => {
            return Err(format!("body.items.item unexpected type: {other:?}").into());
        }
    };

    raw_items.iter().copied().map(parse_single_item).collect()
}

fn parse_single_item(item: &Value) -> Result<BuildingItem, BuildingRegisterError> {
    // mgmBldrgstPk: 실 응답은 JSON number (예: 1024112777), docs 의 "String 으로 받아라"
    // 가이드와 다름. fixture (`tests/fixtures/live_*.json`) 검증. number / string 모두 처리.
    let mgm_bldrgst_pk = parse_id_as_string(item, "mgmBldrgstPk")?;
    let bldg_nm = item
        .get("bldNm")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .unwrap_or_default();
    let main_purps_cd_nm = item
        .get("mainPurpsCdNm")
        .and_then(Value::as_str)
        .ok_or("item.mainPurpsCdNm missing")?
        .to_owned();
    // totArea: 실 응답은 JSON number (예: 212615.29), 일부 endpoint 는 string 으로 wrap.
    // 둘 다 처리 — 라이브 fixture (`live_2026-05-08_*.json`) 검증.
    let tot_area = parse_f64_field(item, "totArea")?;

    Ok(BuildingItem {
        // 식별자 / 위치
        mgm_bldrgst_pk,
        bldg_nm,
        plat_plc: parse_optional_string(item, "platPlc"),

        // 용도 / 구조
        main_purps_cd_nm,
        strct_cd_nm: parse_optional_string(item, "strctCdNm"),

        // 면적 / 비율
        plat_area: parse_optional_f64(item, "platArea"),
        arch_area: parse_optional_f64(item, "archArea"),
        bc_rat: parse_optional_f64(item, "bcRat"),
        tot_area,
        vl_rat: parse_optional_f64(item, "vlRat"),

        // 층수 / 높이
        grnd_flr_cnt: parse_optional_u32(item, "grndFlrCnt"),
        ugrnd_flr_cnt: parse_optional_u32(item, "ugrndFlrCnt"),
        heit: parse_optional_f64(item, "heit"),

        // 승강기
        ride_use_elvt_cnt: parse_optional_u32(item, "rideUseElvtCnt"),
        emgen_use_elvt_cnt: parse_optional_u32(item, "emgenUseElvtCnt"),

        // 주차장
        indr_auto_utcnt: parse_optional_u32(item, "indrAutoUtcnt"),
        oudr_auto_utcnt: parse_optional_u32(item, "oudrAutoUtcnt"),

        // 부속건축물
        atch_bld_cnt: parse_optional_u32(item, "atchBldCnt"),
        atch_bld_area: parse_optional_f64(item, "atchBldArea"),

        // 날짜 (YYYYMMDD)
        pms_day: parse_optional_yyyymmdd(item, "pmsDay"),
        stcns_day: parse_optional_yyyymmdd(item, "stcnsDay"),
        use_apr_day: parse_optional_yyyymmdd(item, "useAprDay"),
    })
}

/// 정부 API 가 ID 필드를 number 또는 string 둘 다 보내는 케이스 처리.
///
/// data.go.kr 의 `mgmBldrgstPk` 가 실 응답에서 JSON number (예: `1024112777`) 로 오는데
/// docs 가이드는 "String 으로 저장" 이라 두 형식 모두 수용. boolean / null / array 는 거부.
fn parse_id_as_string(item: &Value, field: &str) -> Result<String, BuildingRegisterError> {
    match item.get(field) {
        Some(Value::String(s)) => Ok(s.trim().to_owned()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        Some(other) => Err(format!("item.{field} unexpected type: {other:?}").into()),
        None => Err(format!("item.{field} missing").into()),
    }
}

/// 정부 API 가 숫자 필드를 number 또는 string 둘 다 보내는 케이스 처리.
///
/// data.go.kr 의 `totArea`/`platArea`/`bcRat`/`vlRat`/`heit` 등 모든 수치 필드는
/// 응답마다 number / string 변동. 빈 문자열 → "missing".
fn parse_f64_field(item: &Value, field: &str) -> Result<f64, BuildingRegisterError> {
    match item.get(field) {
        Some(Value::Number(n)) => n
            .as_f64()
            .ok_or_else(|| format!("item.{field} not f64-representable").into()),
        Some(Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                return Err(format!("item.{field} missing (empty string)").into());
            }
            t.parse::<f64>()
                .map_err(|e| format!("item.{field} parse: {e}").into())
        }
        Some(other) => Err(format!("item.{field} unexpected type: {other:?}").into()),
        None => Err(format!("item.{field} missing").into()),
    }
}

/// data.go.kr 의 string 필드 — 빈 / 공백 / null / 누락 모두 `None`.
///
/// 정부 API 가 빈 string 을 `" "` (단일 공백) 으로 보내는 패턴 처리.
fn parse_optional_string(item: &Value, field: &str) -> Option<String> {
    item.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// 옵션 f64 — number / string 둘 다 처리, 빈 / 0 / 누락 → `None`.
///
/// 0 은 의도적 fallthrough — `bcRat = 0` 같은 정상값이 있을 수 있으나, 산업 매물
/// 표시 측면에서 0 은 "측정값 없음" 으로 간주 (UI 가 "—" 로 노출).
fn parse_optional_f64(item: &Value, field: &str) -> Option<f64> {
    match item.get(field)? {
        Value::Number(n) => n.as_f64().filter(|v| *v > 0.0),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                t.parse::<f64>().ok().filter(|v| *v > 0.0)
            }
        }
        _ => None,
    }
}

/// 옵션 u32 — number / string 둘 다 처리, 음수 / 빈 / 누락 → `None`.
///
/// 0 은 *유의미한 값* 으로 보존 (예: `ugrndFlrCnt = 0` = "지하층 없음").
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

/// `YYYYMMDD` 8자리 string 만 `Some`. 그 외 (`" "` / 길이 mismatch / 누락) → `None`.
fn parse_optional_yyyymmdd(item: &Value, field: &str) -> Option<String> {
    item.get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    fn ok_response(item: &Value) -> Value {
        serde_json::json!({
            "response": {
                "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
                "body": { "items": { "item": item.clone() } }
            }
        })
    }

    #[test]
    fn parse_items_handles_single_object() {
        let raw = ok_response(&serde_json::json!({
            "mgmBldrgstPk": "12345678901234567",
            "bldNm": "공장1동",
            "mainPurpsCdNm": "공장",
            "totArea": "1500.50",
            "useAprDay": "20100315"
        }));
        let items = parse_items(&raw).expect("parse ok");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].mgm_bldrgst_pk, "12345678901234567");
        assert_eq!(items[0].bldg_nm, "공장1동");
        assert_eq!(items[0].main_purps_cd_nm, "공장");
        assert!((items[0].tot_area - 1500.50).abs() < f64::EPSILON);
        assert_eq!(items[0].use_apr_day.as_deref(), Some("20100315"));
    }

    #[test]
    fn parse_items_handles_array() {
        let raw = ok_response(&serde_json::json!([
            {
                "mgmBldrgstPk": "A",
                "bldNm": "공장1동",
                "mainPurpsCdNm": "공장",
                "totArea": "100.0",
                "useAprDay": "20100315"
            },
            {
                "mgmBldrgstPk": "B",
                "bldNm": "창고2동",
                "mainPurpsCdNm": "창고",
                "totArea": "200.0",
                "useAprDay": ""
            }
        ]));
        let items = parse_items(&raw).expect("parse ok");
        assert_eq!(items.len(), 2);
        assert_eq!(items[1].use_apr_day, None); // 빈 문자열 → None
    }

    #[test]
    fn parse_items_empty_string_returns_empty() {
        let raw = serde_json::json!({
            "response": {
                "header": { "resultCode": "00", "resultMsg": "NORMAL SERVICE." },
                "body": { "items": "" }
            }
        });
        let items = parse_items(&raw).expect("parse ok");
        assert!(items.is_empty());
    }

    #[test]
    fn parse_items_handles_number_mgm_bldrgst_pk() {
        // 실 API 응답 검증 (2026-05-08 강남구 역삼동 737 호출 결과).
        // mgmBldrgstPk 가 JSON number 로 옴 — `Value::as_str` 만 쓰면 None 으로 떨어져 502 발생.
        let raw = ok_response(&serde_json::json!({
            "mgmBldrgstPk": 1_024_112_777_i64,
            "bldNm": "강남파이낸스센터",
            "mainPurpsCdNm": "업무시설",
            "totArea": 212_615.29,
            "useAprDay": "20010731"
        }));
        let items = parse_items(&raw).expect("parse ok");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].mgm_bldrgst_pk, "1024112777");
        assert!((items[0].tot_area - 212_615.29).abs() < f64::EPSILON);
    }

    #[test]
    #[allow(clippy::cognitive_complexity)] // 21 필드 필드 별 검증 — 분해 시 fixture I/O 중복.
    fn parse_items_handles_live_fixture() {
        // 2026-05-08 라이브 호출 fixture — 정부 API 가 *지금* 실제로 보내는 응답.
        // schema drift 발생 시 본 테스트가 가장 먼저 깨짐.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("crates/data-clients/data-go-kr/tests/fixtures/live_2026-05-08_gangnam_yeoksam_737.json");
        let raw_str = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {}", path.display(), e));
        let raw: Value = serde_json::from_str(&raw_str).expect("valid JSON");
        let items = parse_items(&raw).expect("parse live fixture");
        assert_eq!(items.len(), 1);
        let b = &items[0];

        // === 식별자 / 위치 ===
        assert_eq!(b.mgm_bldrgst_pk, "1024112777"); // number → String
        assert!(!b.bldg_nm.is_empty()); // 강남파이낸스센터 (한글)
        assert!(b.plat_plc.is_some()); // platPlc 풀주소

        // === 면적 / 비율 (산업 매물 핵심) ===
        assert!(b.tot_area > 200_000.0); // 212615.29 m² (대형)
        assert!(b.plat_area.is_some_and(|v| v > 13_000.0)); // 13156.7 m²
        assert!(b.arch_area.is_some_and(|v| v > 5_000.0)); // 5600.51 m²
        assert!(b.bc_rat.is_some_and(|v| v > 40.0 && v < 50.0)); // 42.5677 %
        assert!(b.vl_rat.is_some_and(|v| v > 900.0)); // 995.1887 %

        // === 층수 / 높이 ===
        assert_eq!(b.grnd_flr_cnt, Some(45)); // 지상 45층
        assert_eq!(b.ugrnd_flr_cnt, Some(8)); // 지하 8층
        assert!(b.heit.is_some_and(|v| v > 200.0)); // 202.65 m

        // === 승강기 ===
        assert_eq!(b.ride_use_elvt_cnt, Some(29)); // 승용 29
        assert_eq!(b.emgen_use_elvt_cnt, Some(2)); // 비상 2

        // === 주차장 ===
        assert_eq!(b.indr_auto_utcnt, Some(1300)); // 옥내 1300대
        assert_eq!(b.oudr_auto_utcnt, Some(12)); // 옥외 12대

        // === 날짜 ===
        assert_eq!(b.pms_day.as_deref(), Some("19950504")); // 허가
        assert_eq!(b.stcns_day.as_deref(), Some("19950513")); // 착공
        assert_eq!(b.use_apr_day.as_deref(), Some("20010731")); // 사용승인
    }

    #[test]
    fn parse_id_as_string_rejects_invalid_types() {
        let item = serde_json::json!({"a": true, "b": null, "c": []});
        assert!(parse_id_as_string(&item, "a").is_err());
        assert!(parse_id_as_string(&item, "b").is_err());
        assert!(parse_id_as_string(&item, "c").is_err());
        assert!(parse_id_as_string(&item, "missing").is_err());
    }

    #[test]
    fn parse_items_api_error_returns_err() {
        let raw = serde_json::json!({
            "response": {
                "header": { "resultCode": "30", "resultMsg": "SERVICE_KEY_IS_NOT_REGISTERED_ERROR" }
            }
        });
        let err = parse_items(&raw).expect_err("api error");
        let msg = err.to_string();
        assert!(msg.contains("resultCode=30"));
    }
}
