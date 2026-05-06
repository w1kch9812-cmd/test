//! Health check endpoints (SP-Obs T7).
//!
//! K8s / ECS 가 *liveness* (재시작 trigger) ↔ *readiness* (트래픽 cut) 구분
//! 필요. 본 모듈이 두 개 분리:
//!
//! - `GET /healthz` — liveness (process 가 살아 있는가). 항상 200
//! - `GET /healthz/ready` — readiness (DB ping 성공? Redis 미설정이면 skip). 다운
//!   시 503 → load balancer 가 트래픽 cut, restart trigger X
//! - `GET /healthz/db` — DB 단독 (debug). production internal access only — FU
//!
//! 응답 body 는 작은 JSON `{ "status": "ok" }`. 503 시 `ProblemDetails`.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use sqlx::PgPool;

use crate::http::problem::{problem, ProblemResponse};

/// 핸들러 공유 상태 — DB pool + (optional) Redis pool.
#[derive(Clone)]
pub struct HealthState {
    /// Postgres 연결 풀.
    pub pool: PgPool,
    /// Redis 풀 — `REDIS_URL` 미설정 환경 (개발) 에선 `None`. readiness probe
    /// 가 None 일 때 Redis check skip.
    pub redis_pool: Option<Arc<deadpool_redis::Pool>>,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// 상태 라벨 (`"ok"`).
    pub status: &'static str,
}

/// `GET /healthz` — liveness probe.
///
/// process 가 살아서 응답할 수 있다는 것만 보장. DB / Redis check X — K8s
/// liveness probe 가 false 면 *재시작* trigger 라 외부 의존 down 으로 재시작
/// 사이클 일으키면 안 됨.
#[allow(clippy::unused_async)] // axum handler 시그니처 요구.
pub async fn liveness() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// `GET /healthz/ready` — readiness probe.
///
/// DB ping (`SELECT 1`) + Redis ping (configured 시). 어느 하나라도 실패 → 503
/// → load balancer 가 트래픽 cut. K8s readiness probe 가 false 일 때 traffic
/// drain 되지만 *재시작은 안* 함.
///
/// SP-Obs T7 1차: 의존성 down 시 503. 후속 (FU 95) 가 부분 degraded mode (Redis
/// down 이지만 DB OK 면 200 + `degraded: true`) 검토.
#[tracing::instrument(skip(state))]
pub async fn readiness(
    State(state): State<HealthState>,
) -> Result<Json<HealthResponse>, ProblemResponse> {
    // 1. DB ping.
    sqlx::query("select 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "readiness: DB ping failed");
            problem(
                "not-ready/db",
                "DB 에 연결할 수 없어요",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;

    // 2. Redis ping (configured 시).
    if let Some(redis_pool) = &state.redis_pool {
        let mut conn = redis_pool.get().await.map_err(|e| {
            tracing::warn!(error = %e, "readiness: Redis pool get failed");
            problem(
                "not-ready/redis",
                "Redis 에 연결할 수 없어요",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;
        deadpool_redis::redis::cmd("PING")
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "readiness: Redis PING failed");
                problem(
                    "not-ready/redis",
                    "Redis ping 실패",
                    StatusCode::SERVICE_UNAVAILABLE,
                    None,
                )
            })?;
    }

    Ok(Json(HealthResponse { status: "ok" }))
}

/// `GET /healthz/db` — DB 단독 health check (debug / on-call).
///
/// **SECURITY**: production 에서는 internal network 또는 admin auth 보호 필요.
/// 1차 = 공개 (running env 내 어디서든 ping 가능). FU (SP8 IaC ALB rule 또는
/// admin role 가드) 가 production 보호.
#[tracing::instrument(skip(state))]
pub async fn db_health(
    State(state): State<HealthState>,
) -> Result<Json<HealthResponse>, ProblemResponse> {
    sqlx::query("select 1")
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "db_health: ping failed");
            problem(
                "not-ready/db",
                "DB 에 연결할 수 없어요",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;
    Ok(Json(HealthResponse { status: "ok" }))
}
