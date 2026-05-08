//! `POST /internal/auth/event` — frontend 가 emit 하는 `AuthEvent` 수신 → `audit_log` INSERT.
//!
//! # Security (audit 2026-05-08 fix)
//!
//! 본 endpoint 는 *frontend BFF (Next.js)* 만 호출. 외부 노출 시 임의 `AuthEvent` inject
//! 로 `audit_log` 오염 가능. 이를 차단하기 위해 *constant-time* `X-Internal-Auth` 헤더
//! 검증 — secret 일치 안 하면 401. secret 은 Rust API + Next 양쪽이 *같은* `INTERNAL_AUTH_SECRET`
//! 환경변수 공유 (production 은 Pulumi secret).
//!
//! 추가 layer (선택, 후속 작업): network ACL 로 ingress 차단 (SP6-iam-infra) — defence in depth.

use auth::audit::{self, AuthEvent};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use sqlx::PgPool;
use std::sync::Arc;
use subtle::ConstantTimeEq;

/// 핸들러용 상태 (DB pool + shared secret).
#[derive(Clone)]
pub struct AuthEventState {
    /// Postgres 연결 풀.
    pub pool: PgPool,
    /// `X-Internal-Auth` header 검증용 shared secret.
    /// `Arc<str>` 으로 cheap clone (handler 호출마다 `State::clone`).
    pub internal_auth_secret: Arc<str>,
}

const INTERNAL_AUTH_HEADER: &str = "x-internal-auth";

/// 요청 본문.
#[derive(Debug, Deserialize)]
pub struct AuthEventPayload {
    /// 이벤트 이름 (`AuthEvent` serde tag).
    pub event: String,
    /// 이벤트 데이터 (`AuthEvent` 필드들).
    pub payload: serde_json::Value,
}

/// 핸들러 — `event` + `payload` 를 합쳐 `AuthEvent` 로 deserialize 한 후 `audit_log` 에 기록해요.
///
/// # Errors
///
/// JSON 파싱 / DB INSERT 실패 시 4xx/5xx 반환.
pub async fn post_auth_event(
    State(state): State<AuthEventState>,
    headers: HeaderMap,
    Json(body): Json<AuthEventPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    // shared secret header 검증 — *constant-time* 비교로 timing attack 방어.
    let provided = headers
        .get(INTERNAL_AUTH_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let expected = state.internal_auth_secret.as_bytes();
    let actual = provided.as_bytes();
    // 길이 다른 경우 ct_eq 가 false 이지만 *길이 자체* 가 leak 가능 → 길이 검증 후 ct_eq.
    if actual.len() != expected.len() || actual.ct_eq(expected).unwrap_u8() != 1 {
        return Err((StatusCode::UNAUTHORIZED, "invalid internal auth".to_owned()));
    }

    let mut combined = body.payload;
    if let Some(obj) = combined.as_object_mut() {
        obj.insert("event".into(), serde_json::Value::String(body.event));
    } else {
        return Err((StatusCode::BAD_REQUEST, "payload must be object".to_owned()));
    }

    let event: AuthEvent = serde_json::from_value(combined)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event: {e}")))?;

    let correlation_id = format!("cor_{}", audit::generate_id());

    audit::write(&state.pool, &event, &correlation_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}
