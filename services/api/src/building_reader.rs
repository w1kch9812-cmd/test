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

use data_go_kr_client::{building_register::BuildingRegisterClient, pnu_split, DataGoKrClient};
use serde_json::Value;
use shared_kernel::pnu::Pnu;

use crate::routes::buildings::{BuildingItem, BuildingRegisterError, BuildingRegisterReader};

/// `BuildingRegisterReader` 의 data.go.kr 라이브 구현체.
pub struct DataGoKrBuildingRegisterReader {
    client: Arc<DataGoKrClient>,
}

impl DataGoKrBuildingRegisterReader {
    /// 새 [`DataGoKrBuildingRegisterReader`].
    #[must_use]
    pub const fn new(client: Arc<DataGoKrClient>) -> Self {
        Self { client }
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
    let mgm_bldrgst_pk = item
        .get("mgmBldrgstPk")
        .and_then(Value::as_str)
        .ok_or("item.mgmBldrgstPk missing")?
        .to_owned();
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
    let tot_area = item
        .get("totArea")
        .and_then(Value::as_str)
        .ok_or("item.totArea missing")?
        .parse::<f64>()
        .map_err(|e| format!("totArea parse: {e}"))?;
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

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

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
