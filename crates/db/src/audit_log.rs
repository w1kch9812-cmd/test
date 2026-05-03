//! `AuditLogRepository` `Postgres` 구현체. `V002` immutable 트리거가
//! `UPDATE`/`DELETE` 를 차단해요 — 본 저장소는 `INSERT`-only.
//!
//! `inet` 컬럼 ↔ [`std::net::IpAddr`] 매핑은 워크스페이스 `sqlx` 가 `ipnetwork`
//! feature 를 쓰지 않아요. `INSERT` 는 `Option<String>` 바인드 + `$N::inet` 캐스트,
//! `SELECT` 는 `host(ip_address)` 텍스트화 후 [`IpAddr::from_str`] 파싱으로 round-trip.

#![allow(clippy::module_name_repetitions)]

use std::net::IpAddr;
use std::str::FromStr;

use async_trait::async_trait;
use audit_log_domain::entity::AuditLog;
use audit_log_domain::repository::{AuditLogRepository, RepoError};
use chrono::{DateTime, Utc};
use shared_kernel::id::{AuditLogMarker, Id, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `AuditLog` Aggregate 의 `Postgres` 저장소.
///
/// 본 저장소는 `INSERT`-only 이며, `V002` immutable 트리거가 `UPDATE`/`DELETE`
/// 를 DB 레벨에서 차단해요 (정상 운영 경로의 `gongzzang_audit_archiver` 만 retention
/// 후 `DELETE` 가능).
#[derive(Debug, Clone)]
pub struct PgAuditLogRepository {
    pool: PgPool,
}

impl PgAuditLogRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `audit_log` 컬럼을 일관되게 읽기 위한 상수.
///
/// `ip_address` 는 `inet` 타입이라 `host()` 로 텍스트 캐스팅 후 [`IpAddr::from_str`]
/// 로 파싱해요 — 워크스페이스 `sqlx` 가 `ipnetwork` feature 미사용.
const AUDIT_COLUMNS: &str = "id, actor_id, action, resource_kind, resource_id, \
    before_state, after_state, \
    host(ip_address) as ip_text, user_agent, \
    correlation_id, created_at";

/// `PgRow` → [`AuditLog`] 변환. 11 필드 round-trip.
fn row_to_audit_log(row: &PgRow) -> Result<AuditLog, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let actor_id_str: Option<String> = row
        .try_get("actor_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let action: String = row
        .try_get("action")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let resource_kind: String = row
        .try_get("resource_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let resource_id: String = row
        .try_get("resource_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let before_state: Option<serde_json::Value> = row
        .try_get("before_state")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let after_state: Option<serde_json::Value> = row
        .try_get("after_state")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let ip_text: Option<String> = row
        .try_get("ip_text")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_agent: Option<String> = row
        .try_get("user_agent")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row
        .try_get("correlation_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<AuditLogMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed audit_log id: {e}")))?;
    let actor_id = actor_id_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(&s)
                .map_err(|e| RepoError::Database(format!("malformed actor_id: {e}")))
        })
        .transpose()?;
    let ip_address = ip_text
        .as_deref()
        .map(|s| {
            IpAddr::from_str(s).map_err(|e| RepoError::Database(format!("invalid ip in DB: {e}")))
        })
        .transpose()?;

    AuditLog::try_new(
        id,
        actor_id,
        &action,
        &resource_kind,
        &resource_id,
        before_state,
        after_state,
        ip_address,
        user_agent,
        &correlation_id,
        created_at,
    )
    .map_err(|e| RepoError::Database(format!("invalid audit_log row: {e}")))
}

#[async_trait]
impl AuditLogRepository for PgAuditLogRepository {
    #[instrument(skip(self, log), fields(audit_id = %log.id.as_str(), action = %log.action, correlation_id = %log.correlation_id))]
    async fn insert(&self, log: &AuditLog) -> Result<(), RepoError> {
        sqlx::query(
            r"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state,
                ip_address, user_agent,
                correlation_id, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10, $11)
            ",
        )
        .bind(log.id.as_str())
        .bind(log.actor_id.as_ref().map(Id::as_str))
        .bind(&log.action)
        .bind(&log.resource_kind)
        .bind(&log.resource_id)
        .bind(&log.before_state)
        .bind(&log.after_state)
        .bind(
            log.ip_address
                .as_ref()
                .map(std::string::ToString::to_string),
        )
        .bind(&log.user_agent)
        .bind(&log.correlation_id)
        .bind(log.created_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(resource_kind, resource_id, limit))]
    async fn find_by_resource(
        &self,
        resource_kind: &str,
        resource_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log \
             where resource_kind = $1 and resource_id = $2 \
             order by created_at desc \
             limit $3"
        );
        let rows = sqlx::query(&sql)
            .bind(resource_kind)
            .bind(resource_id)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }

    #[instrument(skip(self), fields(actor_id = %actor_id.as_str(), limit))]
    async fn find_by_actor(
        &self,
        actor_id: &Id<UserMarker>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log \
             where actor_id = $1 and created_at >= $2 \
             order by created_at desc \
             limit $3"
        );
        let rows = sqlx::query(&sql)
            .bind(actor_id.as_str())
            .bind(since)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }

    #[instrument(skip(self), fields(correlation_id))]
    async fn find_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<AuditLog>, RepoError> {
        let sql = format!(
            "select {AUDIT_COLUMNS} from audit_log \
             where correlation_id = $1 \
             order by created_at asc"
        );
        let rows = sqlx::query(&sql)
            .bind(correlation_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_audit_log).collect()
    }
}
