//! `X-Request-Id` correlation chain — SP-Obs T2.
//!
//! 모든 요청 진입 시점에 `X-Request-Id` header 검사 → 있으면 사용, 없으면
//! `req_<ULID>` 자동 생성. `RequestId` extension 으로 downstream extractor
//! 주입. tracing span 자동 attach. 응답 header echo (debugging UX).
//!
//! `auth_layer` 보다 먼저 거치도록 라우터 조립 — 인증 실패해도 trace ID 부여.

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::Response;
use ulid::Ulid;

/// 요청에 부여된 correlation ID. handler 가 `Extension<RequestId>` 로 추출.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    /// 내부 문자열 접근. T3 (`MutationContextBuilder`) 가 사용 예정.
    #[must_use]
    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const HEADER_NAME: &str = "x-request-id";
/// 외부 input 의 max length — 헤더 abuse 방지 (logs flood / `DoS`).
const MAX_INBOUND_LEN: usize = 64;

/// `X-Request-Id` middleware — 요청에 ID 부여 + 응답 echo + tracing span attach.
///
/// `auth_layer` 보다 먼저 거치도록 router 조립. 인증 실패해도 trace ID 부여.
pub async fn request_id_layer(mut req: Request<Body>, next: Next) -> Response {
    let id = inbound_id(&req).unwrap_or_else(|| format!("req_{}", Ulid::new()));

    req.extensions_mut().insert(RequestId(id.clone()));

    // tracing span — downstream `tracing::instrument` 들이 자동 nested.
    let span = tracing::info_span!(
        "http_request",
        request_id = %id,
        method = %req.method(),
        path = %req.uri().path(),
    );
    let _enter = span.enter();

    let mut response = next.run(req).await;
    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(HEADER_NAME, value);
    }
    response
}

/// inbound `X-Request-Id` 추출 + sanitize. ASCII 만, ≤64 char.
///
/// 외부 사용자가 임의 header 보내도 안전 — invalid → None → 자동 생성.
fn inbound_id(req: &Request<Body>) -> Option<String> {
    let raw = req.headers().get(HEADER_NAME)?.to_str().ok()?.trim();
    if raw.is_empty() || raw.len() > MAX_INBOUND_LEN {
        return None;
    }
    // ASCII 영숫자 + dash/underscore 만 허용 — control char / 한글 등 거부.
    if !raw
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(raw.to_owned())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use axum::http::HeaderMap;

    fn req_with_header(value: &str) -> Request<Body> {
        let mut req = Request::new(Body::empty());
        let mut headers = HeaderMap::new();
        headers.insert(HEADER_NAME, HeaderValue::from_str(value).unwrap());
        *req.headers_mut() = headers;
        req
    }

    #[test]
    fn inbound_extracts_valid_id() {
        let req = req_with_header("test-abc-123");
        assert_eq!(inbound_id(&req), Some("test-abc-123".to_owned()));
    }

    #[test]
    fn inbound_rejects_empty() {
        let req = req_with_header("");
        assert_eq!(inbound_id(&req), None);
    }

    #[test]
    fn inbound_rejects_too_long() {
        let long = "a".repeat(MAX_INBOUND_LEN + 1);
        let req = req_with_header(&long);
        assert_eq!(inbound_id(&req), None);
    }

    #[test]
    fn inbound_rejects_non_ascii_alnum() {
        // 한글 / control char / 공백 등 거부 — log injection / line break 방지.
        // (HeaderValue::from_str 가 control char 차단하므로 from_bytes 로 우회 불가)
        let req = req_with_header("with space");
        assert_eq!(inbound_id(&req), None);
    }

    #[test]
    fn inbound_accepts_underscore() {
        let req = req_with_header("req_01HXYZ");
        assert_eq!(inbound_id(&req), Some("req_01HXYZ".to_owned()));
    }

    #[test]
    fn inbound_returns_none_when_missing() {
        let req: Request<Body> = Request::new(Body::empty());
        assert_eq!(inbound_id(&req), None);
    }

    #[test]
    fn inbound_trims_whitespace() {
        let req = req_with_header("  trimmed  ");
        assert_eq!(inbound_id(&req), Some("trimmed".to_owned()));
    }
}
