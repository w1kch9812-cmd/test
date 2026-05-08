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
    // useAprDay = `YYYYMMDD` 8자리. 빈 문자열 / 길이 불일치 → None.
    let use_apr_day = item
        .get("useAprDay")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| s.len() == 8)
        .map(str::to_owned);

    Ok(BuildingItem {
        mgm_bldrgst_pk,
        bldg_nm,
        main_purps_cd_nm,
        tot_area,
        use_apr_day,
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
        // 실 PK number → string conversion 검증.
        assert_eq!(items[0].mgm_bldrgst_pk, "1024112777");
        // totArea = 212615.29 (대형 건물).
        assert!(items[0].tot_area > 200_000.0);
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
