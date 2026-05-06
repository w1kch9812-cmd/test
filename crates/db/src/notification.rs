//! `NotificationRepository` `Postgres` 구현체 (SP5-ii).
//!
//! append-mostly + 멱등 `mark_read` + bulk `mark_all_read_by_kind`. 모든
//! mutation 은 SP5-iv 의 transactional `audit_log` + `outbox_event` 패턴.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_domain::entity::Notification;
use notification_domain::kind::NotificationKind;
use notification_domain::repository::{NotificationRepository, RepoError};
use shared_kernel::id::{AuditLogMarker, Id, NotificationMarker, OutboxEventMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use std::str::FromStr;
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `Notification` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgNotificationRepository {
    pool: PgPool,
}

impl PgNotificationRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const COLUMNS: &str = "id, user_id, kind, payload, read_at, created_at";

fn row_to_notification(row: &PgRow) -> Result<Notification, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_id_str: String = row
        .try_get("user_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let kind_str: String = row
        .try_get("kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    // SP6-v: DB varchar(50) → NotificationKind enum. Unknown 코드 = Other (forward-compat).
    // FromStr is Infallible — unwrap_or_else 가 실제로 호출되지 않음.
    let kind = NotificationKind::from_str(&kind_str).unwrap_or(NotificationKind::Other);
    let payload: serde_json::Value = row
        .try_get("payload")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let read_at: Option<DateTime<Utc>> = row
        .try_get("read_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<NotificationMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed notification id: {e}")))?;
    let user_id = Id::<UserMarker>::try_from_str(user_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed user_id in DB: {e}")))?;

    Ok(Notification {
        id,
        user_id,
        kind,
        payload,
        read_at,
        created_at,
    })
}

#[async_trait]
impl NotificationRepository for PgNotificationRepository {
    #[instrument(skip(self), fields(user_id = %user_id.as_str()))]
    async fn find_unread_by_user(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<Notification>, RepoError> {
        let sql = format!(
            "select {COLUMNS} from notification \
             where user_id = $1 and read_at is null \
             order by created_at desc"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_notification).collect()
    }

    #[instrument(skip(self), fields(user_id = %user_id.as_str(), limit))]
    async fn find_recent_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<Notification>, RepoError> {
        let sql = format!(
            "select {COLUMNS} from notification \
             where user_id = $1 \
               and created_at > now() - interval '365 days' \
             order by created_at desc \
             limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_notification).collect()
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, notification, ctx), fields(
        notification_id = %notification.id.as_str(),
        kind = %notification.kind,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn insert(
        &self,
        notification: &Notification,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query(
            r"
            insert into notification (id, user_id, kind, payload, read_at, created_at)
            values ($1, $2, $3, $4, $5, $6)
            ",
        )
        .bind(notification.id.as_str())
        .bind(notification.user_id.as_str())
        .bind(notification.kind.as_str())
        .bind(&notification.payload)
        .bind(notification.read_at)
        .bind(notification.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        write_audit_log(&mut tx, notification.id.as_str(), &ctx, None).await?;
        write_outbox_events(&mut tx, notification.id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    /// 단일 알림 읽음 처리 — 멱등 (`UPDATE ... WHERE read_at IS NULL`).
    ///
    /// `rows_affected == 0` 이어도 `Ok(())` — 이미 읽은 row 또는 row 미존재
    /// 모두 OK. caller 가 row 존재 검증 필요 시 별도 `find_by_id` 호출.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        notification_id = %id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn mark_read(
        &self,
        id: &Id<NotificationMarker>,
        at: DateTime<Utc>,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        sqlx::query("update notification set read_at = $1 where id = $2 and read_at is null")
            .bind(at)
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        // 멱등: rows_affected 검증 없음 (이미 읽음 또는 없는 row 모두 OK)

        write_audit_log(&mut tx, id.as_str(), &ctx, None).await?;
        write_outbox_events(&mut tx, id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        user_id = %user_id.as_str(),
        kind = %kind,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn mark_all_read_by_kind(
        &self,
        user_id: &Id<UserMarker>,
        kind: NotificationKind,
        at: DateTime<Utc>,
        ctx: MutationContext,
    ) -> Result<u64, RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let result = sqlx::query(
            "update notification set read_at = $1 \
             where user_id = $2 and kind = $3 and read_at is null",
        )
        .bind(at)
        .bind(user_id.as_str())
        .bind(kind.as_str())
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
        let rows_affected = result.rows_affected();

        // bulk audit row — resource_id = user_id 로 그룹화. metadata 에 kind +
        // rows_marked 보존.
        let bulk_meta = serde_json::json!({
            "kind": kind.as_str(),
            "rows_marked": rows_affected,
            "marked_at_iso": at.to_rfc3339(),
        });
        write_audit_log(&mut tx, user_id.as_str(), &ctx, Some(bulk_meta)).await?;
        write_outbox_events(&mut tx, user_id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(rows_affected)
    }
}

/// `audit_log` 1 row INSERT — `resource_kind = 'notification'`.
///
/// `override_metadata` 가 `Some` 이면 ctx.metadata 대신 사용 (bulk operation
/// 이 자체 metadata 를 채워넣을 때).
async fn write_audit_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resource_id: &str,
    ctx: &MutationContext,
    override_metadata: Option<serde_json::Value>,
) -> Result<(), RepoError> {
    let audit_id = Id::<AuditLogMarker>::new();
    let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
    let metadata = override_metadata.or_else(|| ctx.metadata.clone());
    sqlx::query(
        r"
        insert into audit_log (
            id, actor_id, action, resource_kind, resource_id,
            before_state, after_state,
            ip_address, user_agent,
            correlation_id, created_at
        )
        values ($1, $2, $3, 'notification', $4, NULL, $5, $6::inet, $7, $8, $9)
        ",
    )
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(resource_id)
    .bind(metadata)
    .bind(ctx.client_ip.as_deref())
    .bind(ctx.user_agent.as_deref())
    .bind(&ctx.correlation_id)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    Ok(())
}

/// `outbox_event` row INSERT — `aggregate_kind = 'notification'`.
async fn write_outbox_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    aggregate_id: &str,
    ctx: &MutationContext,
) -> Result<(), RepoError> {
    for event in &ctx.events {
        let outbox_id = Id::<OutboxEventMarker>::new();
        sqlx::query(
            r"
            insert into outbox_event (
                id, aggregate_kind, aggregate_id, event_type, payload,
                correlation_id, created_at, published_at
            )
            values ($1, 'notification', $2, $3, $4, $5, $6, NULL)
            ",
        )
        .bind(outbox_id.as_str())
        .bind(aggregate_id)
        .bind(event.event_type())
        .bind(event.payload())
        .bind(&ctx.correlation_id)
        .bind(event.occurred_at())
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_err)?;
    }
    Ok(())
}
