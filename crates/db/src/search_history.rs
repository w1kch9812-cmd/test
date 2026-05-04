//! `SearchHistoryRepository` `Postgres` 구현체 (SP5-ii).
//!
//! append-mostly + `PIPA` 가명화. 모든 mutation 은 SP5-iv 의 transactional
//! `audit_log` + `outbox_event` 패턴.
//!
//! `pseudonymize_older_than` 은 bulk operation — 단일 `audit_log` row 만
//! 기록되며 `metadata` 에 `rows_pseudonymized` 카운트가 보존돼요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use search_history_domain::entity::SearchHistory;
use search_history_domain::repository::{RepoError, SearchHistoryRepository};
use shared_kernel::id::{AuditLogMarker, Id, OutboxEventMarker, SearchHistoryMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `SearchHistory` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgSearchHistoryRepository {
    pool: PgPool,
}

impl PgSearchHistoryRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const COLUMNS: &str = "id, user_id, query, filters, result_count, correlation_id, created_at";

fn row_to_search_history(row: &PgRow) -> Result<SearchHistory, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_id_str: Option<String> = row
        .try_get("user_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let query: String = row
        .try_get("query")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let filters: serde_json::Value = row
        .try_get("filters")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let result_count_i64: i32 = row
        .try_get("result_count")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row
        .try_get("correlation_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<SearchHistoryMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed search_history id: {e}")))?;
    let user_id = user_id_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed user_id in DB: {e}")))
        })
        .transpose()?;
    let result_count = u32::try_from(result_count_i64).unwrap_or(0);

    Ok(SearchHistory {
        id,
        user_id,
        query,
        filters,
        result_count,
        correlation_id,
        created_at,
    })
}

#[async_trait]
impl SearchHistoryRepository for PgSearchHistoryRepository {
    #[instrument(skip(self), fields(user_id = %user_id.as_str(), limit))]
    async fn find_recent_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<SearchHistory>, RepoError> {
        let sql = format!(
            "select {COLUMNS} from search_history \
             where user_id = $1 \
               and created_at > now() - interval '90 days' \
             order by created_at desc \
             limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_search_history).collect()
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, history, ctx), fields(
        history_id = %history.id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn insert(&self, history: &SearchHistory, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let result_count_i32 = i32::try_from(history.result_count).unwrap_or(i32::MAX);

        sqlx::query(
            r"
            insert into search_history (
                id, user_id, query, filters, result_count, correlation_id, created_at
            )
            values ($1, $2, $3, $4, $5, $6, $7)
            ",
        )
        .bind(history.id.as_str())
        .bind(history.user_id.as_ref().map(Id::as_str))
        .bind(&history.query)
        .bind(&history.filters)
        .bind(result_count_i32)
        .bind(&history.correlation_id)
        .bind(history.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        write_audit_log(&mut tx, history.id.as_str(), &ctx, None).await?;
        write_outbox_events(&mut tx, history.id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        cutoff = %cutoff,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn pseudonymize_older_than(
        &self,
        cutoff: DateTime<Utc>,
        ctx: MutationContext,
    ) -> Result<u64, RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let result = sqlx::query(
            "update search_history set user_id = NULL \
             where created_at < $1 and user_id is not null",
        )
        .bind(cutoff)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;
        let rows_affected = result.rows_affected();

        // bulk audit row — resource_id = "cutoff_<unix_ts>" (≤25 chars).
        let resource_id = format!("cutoff_{}", cutoff.timestamp());
        // bulk metadata 보존 — caller 의 ctx.metadata 가 None 이면 자동 채움.
        let bulk_meta = serde_json::json!({
            "cutoff_iso": cutoff.to_rfc3339(),
            "rows_pseudonymized": rows_affected,
        });
        write_audit_log(&mut tx, &resource_id, &ctx, Some(bulk_meta)).await?;
        // outbox 는 ctx.events 그대로 — bulk 자체 이벤트는 caller 책임.
        write_outbox_events(&mut tx, &resource_id, &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(rows_affected)
    }
}

/// `audit_log` 1 row INSERT — `resource_kind = 'search_history'`.
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
        values ($1, $2, $3, 'search_history', $4, NULL, $5, $6::inet, $7, $8, $9)
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

/// `outbox_event` row INSERT — `aggregate_kind = 'search_history'`, ctx.events 마다.
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
            values ($1, 'search_history', $2, $3, $4, $5, $6, NULL)
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
