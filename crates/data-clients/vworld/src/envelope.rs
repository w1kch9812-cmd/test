//! V-World WFS GetFeature 응답 envelope 파서.
//!
//! 모든 V-World data API 응답의 *공통* 외피만 다룸 — 레이어별 property는
//! `layers/` 모듈이 처리. 이 분리가 핵심 — V-World가 새 레이어를 추가해도
//! envelope은 변하지 않으므로 envelope 코드는 한 번만 작성.
//!
//! 실 응답 변종 3가지 ([tests/fixtures/real_*.json] 참조):
//!
//! 1. `status: "OK"` + `result.featureCollection.features[]` — 정상
//! 2. `status: "NOT_FOUND"` + `record.total: "0"` (no `result` field) — 빈 결과
//! 3. `status: "ERROR"` + `error: { code, text }` (no `result`) — API 에러
//!
//! [`Envelope::parse`] 가 이 3 케이스를 type-safe하게 [`Outcome`] enum으로
//! 분기. 호출자(layer parser)는 `Outcome::Features(Vec<&Value>)`만 다루면 됨.

#![allow(clippy::module_name_repetitions)]

use serde_json::Value;

use crate::error::ParseError;

/// V-World envelope parsing 결과.
#[derive(Debug)]
pub enum Outcome<'a> {
    /// `status: "OK"` — feature 0개 이상 (빈 features 가능).
    Features(Vec<&'a Value>),
    /// `status: "NOT_FOUND"` — 조건에 맞는 record 없음.
    NotFound,
    // ERROR 케이스는 `Result<Outcome, ParseError::VWorldApi>` 로 분기 — Outcome
    // 자체가 성공 path만 표현하는 게 호출자 단순화에 유리.
}

/// `parse(raw)` — envelope만 검증/추출.
///
/// # Errors
///
/// - `status: "ERROR"` → [`ParseError::VWorldApi`] (code + text 보존)
/// - `response` 키 자체 누락 → [`ParseError::Malformed`]
/// - `status: "OK"` 인데 `result.featureCollection.features` 가 array 아님 →
///   [`ParseError::Malformed`]
/// - 인식 불가 status → [`ParseError::Malformed`] (envelope drift 검출)
pub fn parse(raw: &Value) -> Result<Outcome<'_>, ParseError> {
    let response = raw
        .get("response")
        .ok_or_else(|| ParseError::Malformed("missing 'response' root key".into()))?;

    let status = response
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| ParseError::Malformed("response.status missing or not string".into()))?;

    match status {
        "OK" => {
            let features = response
                .pointer("/result/featureCollection/features")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ParseError::Malformed(
                        "OK status but missing /result/featureCollection/features array".into(),
                    )
                })?;
            Ok(Outcome::Features(features.iter().collect()))
        }
        "NOT_FOUND" => Ok(Outcome::NotFound),
        "ERROR" => {
            let err = response
                .get("error")
                .ok_or_else(|| ParseError::Malformed("ERROR status but no error object".into()))?;
            let code = err
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_owned();
            let text = err
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("(no text)")
                .to_owned();
            Err(ParseError::VWorldApi { code, text })
        }
        other => Err(ParseError::Malformed(format!(
            "unrecognized response.status '{other}' (envelope drift?)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::match_wildcard_for_single_variants
    )]

    use super::*;
    use serde_json::json;

    #[test]
    fn parse_ok_with_features() {
        let raw = json!({
            "response": {
                "status": "OK",
                "result": {
                    "featureCollection": {
                        "features": [
                            { "type": "Feature", "properties": { "pnu": "1" } },
                            { "type": "Feature", "properties": { "pnu": "2" } }
                        ]
                    }
                }
            }
        });
        let outcome = parse(&raw).expect("ok");
        match outcome {
            Outcome::Features(f) => assert_eq!(f.len(), 2),
            other => panic!("expected Features, got {other:?}"),
        }
    }

    #[test]
    fn parse_ok_with_empty_features() {
        let raw = json!({
            "response": {
                "status": "OK",
                "result": { "featureCollection": { "features": [] } }
            }
        });
        let outcome = parse(&raw).expect("ok");
        match outcome {
            Outcome::Features(f) => assert!(f.is_empty()),
            other => panic!("expected empty Features, got {other:?}"),
        }
    }

    #[test]
    fn parse_not_found_returns_not_found() {
        // 실 V-World NOT_FOUND 응답은 `result` 자체가 없음.
        let raw = json!({
            "response": {
                "status": "NOT_FOUND",
                "record": { "total": "0", "current": "0" }
            }
        });
        let outcome = parse(&raw).expect("ok");
        assert!(matches!(outcome, Outcome::NotFound));
    }

    #[test]
    fn parse_error_status_maps_to_vworld_api_error() {
        let raw = json!({
            "response": {
                "status": "ERROR",
                "error": {
                    "level": "1",
                    "code": "INVALID_RANGE",
                    "text": "attrFilter 속성명은 ..."
                }
            }
        });
        let err = parse(&raw).unwrap_err();
        match err {
            ParseError::VWorldApi { code, text } => {
                assert_eq!(code, "INVALID_RANGE");
                assert!(text.contains("attrFilter"));
            }
            other => panic!("expected VWorldApi, got {other:?}"),
        }
    }

    #[test]
    fn parse_invalid_key_error() {
        let raw = json!({
            "response": {
                "status": "ERROR",
                "error": { "code": "INVALID_KEY", "text": "key 또는 domain 검증 실패" }
            }
        });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::VWorldApi { code, .. } if code == "INVALID_KEY"));
    }

    #[test]
    fn parse_missing_response_root() {
        let raw = json!({ "unexpected": "shape" });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("response")));
    }

    #[test]
    fn parse_missing_status() {
        let raw = json!({ "response": { "result": {} } });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("status")));
    }

    #[test]
    fn parse_unrecognized_status() {
        let raw = json!({ "response": { "status": "UNKNOWN_FUTURE_STATE" } });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("envelope drift")));
    }

    #[test]
    fn parse_ok_but_missing_feature_collection() {
        let raw = json!({ "response": { "status": "OK", "result": {} } });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("featureCollection")));
    }

    #[test]
    fn parse_error_status_but_no_error_object() {
        let raw = json!({ "response": { "status": "ERROR" } });
        let err = parse(&raw).unwrap_err();
        assert!(matches!(err, ParseError::Malformed(s) if s.contains("ERROR status")));
    }
}
