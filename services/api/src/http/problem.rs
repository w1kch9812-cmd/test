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

/// 도메인 [`ListingError`] → RFC 7807 매핑 (SP6-iv).
///
/// - `TransactionFieldsMismatch` → 400 `transaction-fields-mismatch`
/// - `InvalidTransition` → 409 `invalid-transition`
/// - `ImmutableState` → 409 `immutable-state`
#[must_use]
#[allow(dead_code)] // T4 가 wire-up. T3 단독 commit 시점엔 caller 없음.
pub fn from_listing_error(e: &listing_domain::errors::ListingError) -> ProblemResponse {
    use listing_domain::errors::ListingError;
    match e {
        ListingError::TransactionFieldsMismatch { .. } => problem(
            "transaction-fields-mismatch",
            "거래 유형과 보증금/월세 조합이 맞지 않습니다",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        ),
        ListingError::InvalidTransition { .. } => problem(
            "invalid-transition",
            "현재 상태에서 허용되지 않는 전이입니다",
            StatusCode::CONFLICT,
            Some(e.to_string()),
        ),
        ListingError::ImmutableState { .. } => problem(
            "immutable-state",
            "현재 상태에서 매물을 수정할 수 없습니다",
            StatusCode::CONFLICT,
            Some(e.to_string()),
        ),
    }
}

/// 도메인 [`RepoError`] → RFC 7807 매핑 (SP6-iv).
///
/// - `NotFound` → 404 `not-found`
/// - `Conflict` (OCC) → 409 `version-conflict`
/// - `Database` → 500 `internal-error` (detail 은 prod 에서 leak 위험 — None)
#[must_use]
#[allow(dead_code)] // T4 가 wire-up.
pub fn from_listing_repo_error(e: &listing_domain::repository::RepoError) -> ProblemResponse {
    use listing_domain::repository::RepoError;
    match e {
        RepoError::NotFound => problem(
            "not-found",
            "매물을 찾을 수 없습니다",
            StatusCode::NOT_FOUND,
            None,
        ),
        RepoError::Conflict => problem(
            "version-conflict",
            "동시 수정 충돌 — 다시 불러오세요",
            StatusCode::CONFLICT,
            None,
        ),
        RepoError::Database(_) => {
            // SECURITY: DB error message 가 application/problem+json 으로 외부 노출
            // 되면 schema 정보 leak. 로그는 caller 가 tracing 으로 남기고, body 는 generic.
            problem(
                "internal-error",
                "내부 서버 오류",
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            )
        }
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
