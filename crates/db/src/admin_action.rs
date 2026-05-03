//! `PgAdminActionRepository` — `Postgres` 구현체.
//!
//! `INSERT`-only — admin 액션은 immutable. `insert` 가 트랜잭션 안에서
//! `audit_log` + `outbox_event` 를 함께 `INSERT` 해 transactional 추적성 보장.
//!
//! 본 저장소는 SP5-iii 의 첫 transactional `audit_log`/`outbox_event` 패턴
//! 구현이에요. T6-T10 의 다른 저장소들이 같은 흐름을 따라가요:
//!
//! 1. `pool.begin()` 으로 트랜잭션 시작
//! 2. 본 도메인 row `INSERT`
//! 3. `audit_log` row `INSERT` (`MutationContext` 의 actor/action/metadata 매핑)
//! 4. `MutationContext::events` 의 각 도메인 이벤트마다 `outbox_event` `INSERT`
//! 5. `tx.commit()` — 실패 시 자동 rollback (tx `Drop`)

#![allow(clippy::module_name_repetitions)]

use admin_action_domain::entity::AdminAction;
use admin_action_domain::repository::{AdminActionRepository, RepoError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{AdminActionMarker, AuditLogMarker, Id, OutboxEventMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `AdminAction` Aggregate 의 `Postgres` 저장소.
///
/// `INSERT`-only — admin 액션은 immutable. `insert` 는 transactional 패턴
/// (`admin_action` + `audit_log` + `outbox_event` 를 한 `tx` 안에서 모두 기록).
#[derive(Debug, Clone)]
pub struct PgAdminActionRepository {
    pool: PgPool,
}

impl PgAdminActionRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `admin_action` 컬럼을 일관되게 가져오기 위한 상수.
const ADMIN_ACTION_COLUMNS: &str = "id, admin_id, action_kind, target_kind, target_id, \
    payload, correlation_id, created_at";

/// `PgRow` → [`AdminAction`] 변환. 8 필드 round-trip.
fn row_to_admin_action(row: &PgRow) -> Result<AdminAction, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let admin_id_str: String = row
        .try_get("admin_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let action_kind: String = row
        .try_get("action_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_kind: Option<String> = row
        .try_get("target_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_id: Option<String> = row
        .try_get("target_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let payload: serde_json::Value = row
        .try_get("payload")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row
        .try_get("correlation_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<AdminActionMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed admin_action id: {e}")))?;
    let admin_id = Id::<UserMarker>::try_from_str(admin_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed admin_id: {e}")))?;

    AdminAction::try_new(
        id,
        admin_id,
        &action_kind,
        target_kind.as_deref(),
        target_id.as_deref(),
        payload,
        &correlation_id,
        created_at,
    )
    .map_err(|e| RepoError::Database(format!("invalid admin_action row: {e}")))
}

#[async_trait]
impl AdminActionRepository for PgAdminActionRepository {
    /// 트랜잭션 안에서 `admin_action` + `audit_log` + `outbox_event` 를 함께 `INSERT`.
    ///
    /// `MutationContext` 매핑:
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL`)
    /// - `ctx.action` → `audit_log.action`
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet` 캐스팅)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id` (action 자체의
    ///   `correlation_id` 와 다를 수 있어요 — `ctx` 가 호출 컨텍스트)
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    /// - `ctx.events` → 각 이벤트마다 `outbox_event` row 1개
    ///
    /// `tx` Drop 시 자동 `rollback` 되므로 어느 단계든 실패하면 일관된 상태.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, action, ctx), fields(
        action_id = %action.id.as_str(),
        kind = %action.action_kind,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn insert(&self, action: &AdminAction, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 1. INSERT admin_action
        sqlx::query(
            r"
            insert into admin_action (
                id, admin_id, action_kind, target_kind, target_id,
                payload, correlation_id, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            ",
        )
        .bind(action.id.as_str())
        .bind(action.admin_id.as_str())
        .bind(&action.action_kind)
        .bind(&action.target_kind)
        .bind(&action.target_id)
        .bind(&action.payload)
        .bind(&action.correlation_id)
        .bind(action.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2. INSERT audit_log — 같은 tx
        let audit_id = Id::<AuditLogMarker>::new();
        let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
        sqlx::query(
            r"
            insert into audit_log (
                id, actor_id, action, resource_kind, resource_id,
                before_state, after_state,
                ip_address, user_agent,
                correlation_id, created_at
            )
            values ($1, $2, $3, 'admin_action', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(action.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 3. INSERT outbox_event for each ctx.events — 같은 tx
        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'admin_action', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(action.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        // 4. commit — 실패 시 자동 rollback (tx Drop)
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(admin_id = %admin_id.as_str(), limit))]
    async fn find_by_admin(
        &self,
        admin_id: &Id<UserMarker>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AdminAction>, RepoError> {
        let sql = format!(
            "select {ADMIN_ACTION_COLUMNS} from admin_action \
             where admin_id = $1 and created_at >= $2 \
             order by created_at desc \
             limit $3"
        );
        let rows = sqlx::query(&sql)
            .bind(admin_id.as_str())
            .bind(since)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_admin_action).collect()
    }

    #[instrument(skip(self), fields(target_kind, target_id, limit))]
    async fn find_by_target(
        &self,
        target_kind: &str,
        target_id: &str,
        limit: u32,
    ) -> Result<Vec<AdminAction>, RepoError> {
        let sql = format!(
            "select {ADMIN_ACTION_COLUMNS} from admin_action \
             where target_kind = $1 and target_id = $2 \
             order by created_at desc \
             limit $3"
        );
        let rows = sqlx::query(&sql)
            .bind(target_kind)
            .bind(target_id)
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_admin_action).collect()
    }

    #[instrument(skip(self), fields(correlation_id))]
    async fn find_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<AdminAction>, RepoError> {
        let sql = format!(
            "select {ADMIN_ACTION_COLUMNS} from admin_action \
             where correlation_id = $1 \
             order by created_at asc"
        );
        let rows = sqlx::query(&sql)
            .bind(correlation_id)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_admin_action).collect()
    }
}
