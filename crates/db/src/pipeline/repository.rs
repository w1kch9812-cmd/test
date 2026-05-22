#![allow(clippy::needless_pass_by_value, clippy::too_many_lines)]

use async_trait::async_trait;
use chrono::Utc;
use data_pipeline_control::repository::{PipelineRepository, RepoError};
use data_pipeline_control::run::PipelineRun;
use data_pipeline_control::schedule::PipelineSchedule;
use shared_kernel::id::{
    AuditLogMarker, Id, OutboxEventMarker, PipelineRunMarker, PipelineScheduleMarker,
};
use shared_kernel::mutation::MutationContext;
use tracing::instrument;

use super::rows::{row_to_run, row_to_schedule, RUN_COLUMNS, SCHEDULE_COLUMNS};
use super::PgPipelineRepository;
use crate::error_map::map_sqlx_err;

#[async_trait]
impl PipelineRepository for PgPipelineRepository {
    #[instrument(skip(self), fields(kind = %kind))]
    async fn find_schedule_by_kind(
        &self,
        kind: &str,
    ) -> Result<Option<PipelineSchedule>, RepoError> {
        let sql =
            format!("select {SCHEDULE_COLUMNS} from pipeline_schedule where pipeline_kind = $1");
        let row = sqlx::query(&sql)
            .bind(kind)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_schedule).transpose()
    }

    #[instrument(skip(self), fields(schedule_id = %id.as_str()))]
    async fn find_schedule_by_id(
        &self,
        id: &Id<PipelineScheduleMarker>,
    ) -> Result<Option<PipelineSchedule>, RepoError> {
        let sql = format!("select {SCHEDULE_COLUMNS} from pipeline_schedule where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_schedule).transpose()
    }

    #[instrument(skip(self))]
    async fn list_schedules(&self) -> Result<Vec<PipelineSchedule>, RepoError> {
        let sql =
            format!("select {SCHEDULE_COLUMNS} from pipeline_schedule order by pipeline_kind asc");
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_schedule).collect()
    }

    #[instrument(skip(self, schedule, ctx), fields(
        schedule_id = %schedule.id.as_str(),
        kind = %schedule.pipeline_kind,
        version = schedule.version,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_schedule(
        &self,
        schedule: &PipelineSchedule,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let result = sqlx::query(
            r"
            insert into pipeline_schedule (
                id, pipeline_kind, cron_expression, enabled, timezone,
                last_run_at, next_run_at, config,
                running_lock_acquired_at, running_worker_id,
                updated_at, updated_by, version
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            on conflict (id) do update set
                cron_expression = excluded.cron_expression,
                enabled = excluded.enabled,
                timezone = excluded.timezone,
                last_run_at = excluded.last_run_at,
                next_run_at = excluded.next_run_at,
                config = excluded.config,
                running_lock_acquired_at = excluded.running_lock_acquired_at,
                running_worker_id = excluded.running_worker_id,
                updated_at = excluded.updated_at,
                updated_by = excluded.updated_by,
                version = pipeline_schedule.version + 1
            where pipeline_schedule.version = $13
            ",
        )
        .bind(schedule.id.as_str())
        .bind(&schedule.pipeline_kind)
        .bind(&schedule.cron_expression)
        .bind(schedule.enabled)
        .bind(&schedule.timezone)
        .bind(schedule.last_run_at)
        .bind(schedule.next_run_at)
        .bind(&schedule.config)
        .bind(schedule.running_lock_acquired_at)
        .bind(schedule.running_worker_id.as_deref())
        .bind(schedule.updated_at)
        .bind(schedule.updated_by.as_ref().map(Id::as_str))
        .bind(schedule.version)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        if result.rows_affected() == 0 {
            return Err(RepoError::Conflict);
        }

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
            values ($1, $2, $3, 'pipeline_schedule', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(schedule.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'pipeline_schedule', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(schedule.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(run_id = %id.as_str()))]
    async fn find_run_by_id(
        &self,
        id: &Id<PipelineRunMarker>,
    ) -> Result<Option<PipelineRun>, RepoError> {
        let sql = format!("select {RUN_COLUMNS} from pipeline_run where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_run).transpose()
    }

    #[instrument(skip(self), fields(schedule_id = %schedule_id.as_str(), limit))]
    async fn find_recent_runs(
        &self,
        schedule_id: &Id<PipelineScheduleMarker>,
        limit: u32,
    ) -> Result<Vec<PipelineRun>, RepoError> {
        let sql = format!(
            "select {RUN_COLUMNS} from pipeline_run \
             where schedule_id = $1 \
             order by started_at desc \
             limit $2"
        );
        let rows = sqlx::query(&sql)
            .bind(schedule_id.as_str())
            .bind(i64::from(limit))
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_run).collect()
    }

    #[instrument(skip(self))]
    async fn find_active_runs(&self) -> Result<Vec<PipelineRun>, RepoError> {
        let sql = format!(
            "select {RUN_COLUMNS} from pipeline_run \
             where status = 'running' \
             order by started_at asc"
        );
        let rows = sqlx::query(&sql)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_run).collect()
    }

    #[instrument(skip(self, run, ctx), fields(
        run_id = %run.id.as_str(),
        schedule_id = %run.schedule_id.as_str(),
        status = %run.status.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_run(&self, run: &PipelineRun, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let items_processed_i64 = i64::try_from(run.items_processed).unwrap_or(i64::MAX);
        let items_changed_i64 = i64::try_from(run.items_changed).unwrap_or(i64::MAX);

        sqlx::query(
            r"
            insert into pipeline_run (
                id, schedule_id, started_at, finished_at, status,
                items_processed, items_changed, output_hashes, error_message,
                triggered_by, triggered_by_user, correlation_id, log_url, steps
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            on conflict (id) do update set
                finished_at = excluded.finished_at,
                status = excluded.status,
                items_processed = excluded.items_processed,
                items_changed = excluded.items_changed,
                output_hashes = excluded.output_hashes,
                error_message = excluded.error_message,
                log_url = excluded.log_url,
                steps = excluded.steps
            ",
        )
        .bind(run.id.as_str())
        .bind(run.schedule_id.as_str())
        .bind(run.started_at)
        .bind(run.finished_at)
        .bind(run.status.as_str())
        .bind(items_processed_i64)
        .bind(items_changed_i64)
        .bind(&run.output_hashes)
        .bind(run.error_message.as_deref())
        .bind(run.triggered_by.as_str())
        .bind(run.triggered_by_user.as_ref().map(Id::as_str))
        .bind(&run.correlation_id)
        .bind(run.log_url.as_deref())
        .bind(&run.steps)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

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
            values ($1, $2, $3, 'pipeline_run', $4, NULL, $5, $6::inet, $7, $8, $9)
            ",
        )
        .bind(audit_id.as_str())
        .bind(ctx.actor_id.as_ref().map(Id::as_str))
        .bind(&ctx.action)
        .bind(run.id.as_str())
        .bind(&ctx.metadata)
        .bind(ctx.client_ip.as_deref())
        .bind(ctx.user_agent.as_deref())
        .bind(&ctx.correlation_id)
        .bind(occurred_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        for event in &ctx.events {
            let outbox_id = Id::<OutboxEventMarker>::new();
            sqlx::query(
                r"
                insert into outbox_event (
                    id, aggregate_kind, aggregate_id, event_type, payload,
                    correlation_id, created_at, published_at
                )
                values ($1, 'pipeline_run', $2, $3, $4, $5, $6, NULL)
                ",
            )
            .bind(outbox_id.as_str())
            .bind(run.id.as_str())
            .bind(event.event_type())
            .bind(event.payload())
            .bind(&ctx.correlation_id)
            .bind(event.occurred_at())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        }

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}
