//! `AnalysisReportRepository` `Postgres` 구현체 (SP5-ii).
//!
//! `OCC` (`version`) + `target_pnus char(19)[]` round-trip + `snapshot jsonb`.
//! 모든 mutation 은 SP5-iv 의 transactional `audit_log` + `outbox_event` 패턴.

#![allow(clippy::module_name_repetitions)]

use analysis_report_domain::entity::AnalysisReport;
use analysis_report_domain::repository::{AnalysisReportRepository, RepoError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{AnalysisReportMarker, AuditLogMarker, Id, OutboxEventMarker, UserMarker};
use shared_kernel::mutation::MutationContext;
use shared_kernel::pnu::Pnu;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `AnalysisReport` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgAnalysisReportRepository {
    pool: PgPool,
}

impl PgAnalysisReportRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const COLUMNS: &str = "id, user_id, title, target_pnus, snapshot, created_at, updated_at, version";

fn row_to_analysis_report(row: &PgRow) -> Result<AnalysisReport, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_id_str: String = row
        .try_get("user_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let title: String = row
        .try_get("title")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_pnu_strs: Vec<String> = row
        .try_get("target_pnus")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let snapshot: serde_json::Value = row
        .try_get("snapshot")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<AnalysisReportMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed analysis_report id: {e}")))?;
    let user_id = Id::<UserMarker>::try_from_str(user_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed user_id in DB: {e}")))?;

    let mut target_pnus = Vec::with_capacity(target_pnu_strs.len());
    for s in target_pnu_strs {
        let p = Pnu::try_new(s.trim())
            .map_err(|e| RepoError::Database(format!("malformed PNU in DB: {e}")))?;
        target_pnus.push(p);
    }

    Ok(AnalysisReport {
        id,
        user_id,
        title,
        target_pnus,
        snapshot,
        created_at,
        updated_at,
        version,
    })
}

#[async_trait]
impl AnalysisReportRepository for PgAnalysisReportRepository {
    #[instrument(skip(self), fields(report_id = %id.as_str()))]
    async fn find_by_id(
        &self,
        id: &Id<AnalysisReportMarker>,
    ) -> Result<Option<AnalysisReport>, RepoError> {
        let sql = format!("select {COLUMNS} from analysis_report where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_analysis_report).transpose()
    }

    #[instrument(skip(self), fields(user_id = %user_id.as_str(), limit))]
    async fn find_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<AnalysisReport>, RepoError> {
        let sql = format!(
            "select {COLUMNS} from analysis_report \
             where user_id = $1 \
             order by created_at desc \
             limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_analysis_report).collect()
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, report, ctx), fields(
        report_id = %report.id.as_str(),
        version = report.version,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, report: &AnalysisReport, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state snapshot (None if INSERT).
        let before_state =
            crate::audit_state::read_analysis_report_json(&mut tx, &report.id).await?;

        // target_pnus: Vec<Pnu> → Vec<&str> bind for char(19)[].
        let pnu_strs: Vec<&str> = report.target_pnus.iter().map(Pnu::as_str).collect();

        // OCC pattern (SP5-iv 와 동일): UPSERT WHERE version = $N → 0 rows → Conflict.
        let result = sqlx::query(
            r"
            insert into analysis_report (
                id, user_id, title, target_pnus, snapshot,
                created_at, updated_at, version
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            on conflict (id) do update set
                title = excluded.title,
                target_pnus = excluded.target_pnus,
                snapshot = excluded.snapshot,
                updated_at = excluded.updated_at,
                version = analysis_report.version + 1
            where analysis_report.version = $8
            ",
        )
        .bind(report.id.as_str())
        .bind(report.user_id.as_str())
        .bind(&report.title)
        .bind(&pnu_strs)
        .bind(&report.snapshot)
        .bind(report.created_at)
        .bind(report.updated_at)
        .bind(report.version)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }

        // SP-Obs T4: after_state snapshot + metadata merge.
        let after_state_raw =
            crate::audit_state::read_analysis_report_json(&mut tx, &report.id).await?;
        let after_state =
            crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

        write_audit_log(
            &mut tx,
            report.id.as_str(),
            &ctx,
            before_state,
            after_state,
        )
        .await?;
        write_outbox_events(&mut tx, report.id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        report_id = %id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn delete(
        &self,
        id: &Id<AnalysisReportMarker>,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // SP-Obs T4: before_state — DELETE 직전 row 마지막 상태.
        let before_state =
            crate::audit_state::read_analysis_report_json(&mut tx, id).await?;

        let result = sqlx::query("delete from analysis_report where id = $1")
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        let after_state = crate::audit_state::merge_metadata(None, ctx.metadata.as_ref());

        write_audit_log(&mut tx, id.as_str(), &ctx, before_state, after_state).await?;
        write_outbox_events(&mut tx, id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}

/// `audit_log` 1 row INSERT — `resource_kind = 'analysis_report'` (SP-Obs T4 갱신).
async fn write_audit_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resource_id: &str,
    ctx: &MutationContext,
    before_state: Option<serde_json::Value>,
    after_state: Option<serde_json::Value>,
) -> Result<(), RepoError> {
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
        values ($1, $2, $3, 'analysis_report', $4, $5, $6, $7::inet, $8, $9, $10)
        ",
    )
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(resource_id)
    .bind(&before_state)
    .bind(&after_state)
    .bind(ctx.client_ip.as_deref())
    .bind(ctx.user_agent.as_deref())
    .bind(&ctx.correlation_id)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    Ok(())
}

/// `outbox_event` row INSERT — `aggregate_kind = 'analysis_report'`.
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
            values ($1, 'analysis_report', $2, $3, $4, $5, $6, NULL)
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
