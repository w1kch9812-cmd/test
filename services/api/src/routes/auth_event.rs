//! `POST /internal/auth/event` — frontend 가 emit 하는 `AuthEvent` 수신 → `audit_log` INSERT.
//!
//! # Security
//!
//! 현재 unauthenticated. SP6-iam-infra 가 network ACL 로 ingress 차단 필요 (production 배포 전).
//! 외부 노출 시 임의 `AuthEvent` inject 가능 → `audit_log` 오염.

use auth::audit::{self, AuthEvent};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;

/// 핸들러용 상태 (DB pool).
#[derive(Clone)]
pub struct AuthEventState {
    /// Postgres 연결 풀.
    pub pool: PgPool,
}

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
    Json(body): Json<AuthEventPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
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
