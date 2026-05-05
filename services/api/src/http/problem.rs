//! RFC 7807 `ProblemDetails` (`application/problem+json`) — backend 공통.
//!
//! Frontend 의 `apps/web/lib/http/problem.ts` 와 동일 shape.

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// RFC 7807 Problem Details shape.
#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    /// URI 식별자 (`https://gongzzang.com/errors/<id>`).
    #[serde(rename = "type")]
    pub type_: String,
    /// 사람이 읽는 요약.
    pub title: String,
    /// HTTP 상태 코드.
    pub status: u16,
    /// 상세 설명 (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// RFC 7807 응답 wrapper — `application/problem+json` content-type 강제.
pub struct ProblemResponse {
    /// HTTP 상태.
    pub status: StatusCode,
    /// 응답 body.
    pub body: ProblemDetails,
}

impl IntoResponse for ProblemResponse {
    fn into_response(self) -> Response {
        let mut response = (self.status, Json(self.body)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

/// RFC 7807 응답 생성 헬퍼.
pub fn problem(
    type_id: &str,
    title: &str,
    status: StatusCode,
    detail: Option<String>,
) -> ProblemResponse {
    ProblemResponse {
        status,
        body: ProblemDetails {
            type_: format!("https://gongzzang.com/errors/{type_id}"),
            title: title.to_owned(),
            status: status.as_u16(),
            detail,
        },
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn problem_details_serializes_with_type_field() {
        let p = problem(
            "listings/invalid-bounds",
            "잘못된 bounds",
            StatusCode::BAD_REQUEST,
            None,
        );
        let json = serde_json::to_string(&p.body).unwrap();
        assert!(
            json.contains("\"type\":\"https://gongzzang.com/errors/listings/invalid-bounds\""),
            "type field missing: {json}"
        );
        assert!(
            json.contains("\"status\":400"),
            "status field missing: {json}"
        );
    }

    #[test]
    fn problem_details_omits_detail_when_none() {
        let p = problem("listings/test", "t", StatusCode::BAD_REQUEST, None);
        let json = serde_json::to_string(&p.body).unwrap();
        assert!(
            !json.contains("\"detail\""),
            "detail should be omitted when None: {json}"
        );
    }

    #[test]
    fn problem_details_includes_detail_when_some() {
        let p = problem(
            "listings/test",
            "t",
            StatusCode::BAD_REQUEST,
            Some("d".into()),
        );
        let json = serde_json::to_string(&p.body).unwrap();
        assert!(json.contains("\"detail\":\"d\""), "detail missing: {json}");
    }
}
