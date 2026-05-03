//! `PgPipelineRepository` — `Postgres` 구현체. 2 Aggregate (`PipelineSchedule` +
//! `PipelineRun`) 합친 1 trait 의 단일 구현 (SP5-iii T10).
//!
//! `PipelineSchedule` 은 OCC + transactional `audit_log`/`outbox_event` 패턴
//! (T6 BVQ 와 동일), `PipelineRun` 은 no-OCC + transactional 패턴
//! (T8 `listing_report` 와 동일) 을 사용해요. 두 Aggregate 모두 시스템 액션
//! (`actor_id = None`) 을 자연스럽게 지원해요 — pipeline scheduler / 워커가
//! 자기 자신의 mutation 을 기록하기 때문.
//!
//! ## 설계 메모
//!
//! - `PipelineSchedule` 의 `updated_at` 은 DB 컬럼 *있음* — BVQ/LRQ 의 합성
//!   로직 (`reviewed_at.unwrap_or(submitted_at)`) 같은 우회 안 해요.
//! - `PipelineRun` 의 `items_processed`/`items_changed` 는 도메인 `u64` ↔ DB
//!   `bigint` (`i64`) — `try_from` 으로 안전 변환 (오버플로 시 `i64::MAX`/`0`
//!   으로 saturate, 음수 DB 값은 도메인 `0` 으로 fallback).
//! - `RunStatus`/`TriggerKind` 는 도메인이 제공하는 `FromStr`/`as_str` 사용.
//! - 모든 메서드 `#[tracing::instrument]` — `MutationContext::metadata` 등
//!   PII 가능 필드는 `skip` 처리.
//!
//! ## `resource_kind` / `aggregate_kind` 매핑
//!
//! - `save_schedule` → `audit_log.resource_kind = 'pipeline_schedule'`,
//!   `outbox_event.aggregate_kind = 'pipeline_schedule'`
//! - `save_run` → `audit_log.resource_kind = 'pipeline_run'`,
//!   `outbox_event.aggregate_kind = 'pipeline_run'`

#![allow(
    clippy::module_name_repetitions,
    clippy::needless_pass_by_value,
    clippy::too_many_lines
)]

use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use data_pipeline_control::repository::{PipelineRepository, RepoError};
use data_pipeline_control::run::PipelineRun;
use data_pipeline_control::schedule::PipelineSchedule;
use data_pipeline_control::status::RunStatus;
use data_pipeline_control::trigger_kind::TriggerKind;
use shared_kernel::id::{
    AuditLogMarker, Id, OutboxEventMarker, PipelineRunMarker, PipelineScheduleMarker, UserMarker,
};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `PipelineSchedule` + `PipelineRun` Aggregate 의 `Postgres` 저장소.
///
/// 단일 `PipelineRepository` trait 을 구현해 두 Aggregate 를 함께 노출. 호출자
/// (워커/어드민 API) 는 동일 풀 `PgPool` 위에서 schedule 갱신과 run `INSERT` 를
/// 별도 트랜잭션으로 수행해요.
#[derive(Debug, Clone)]
pub struct PgPipelineRepository {
    pool: PgPool,
}

impl PgPipelineRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `pipeline_schedule` 컬럼 (13) 을 일관되게 가져오기 위한 상수.
const SCHEDULE_COLUMNS: &str = "id, pipeline_kind, cron_expression, enabled, timezone, \
    last_run_at, next_run_at, config, \
    running_lock_acquired_at, running_worker_id, \
    updated_at, updated_by, version";

/// `select` 절에서 모든 `pipeline_run` 컬럼 (14) 을 일관되게 가져오기 위한 상수.
const RUN_COLUMNS: &str = "id, schedule_id, started_at, finished_at, status, \
    items_processed, items_changed, output_hashes, error_message, \
    triggered_by, triggered_by_user, correlation_id, log_url, steps";

fn parse_run_status(s: &str) -> Result<RunStatus, RepoError> {
    RunStatus::from_str(s).map_err(|_| RepoError::Database(format!("unexpected run status: {s}")))
}

fn parse_trigger_kind(s: &str) -> Result<TriggerKind, RepoError> {
    TriggerKind::from_str(s)
        .map_err(|_| RepoError::Database(format!("unexpected trigger_kind: {s}")))
}

/// `PgRow` → [`PipelineSchedule`] 변환 (13 컬럼 round-trip).
fn row_to_schedule(row: &PgRow) -> Result<PipelineSchedule, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let pipeline_kind: String = row
        .try_get("pipeline_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let cron_expression: String = row
        .try_get("cron_expression")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let enabled: bool = row
        .try_get("enabled")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let timezone: String = row
        .try_get("timezone")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let last_run_at: Option<DateTime<Utc>> = row
        .try_get("last_run_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let next_run_at: Option<DateTime<Utc>> = row
        .try_get("next_run_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let config: serde_json::Value = row
        .try_get("config")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let running_lock_acquired_at: Option<DateTime<Utc>> =
        row.try_get("running_lock_acquired_at")
            .map_err(|e| RepoError::Database(e.to_string()))?;
    let running_worker_id: Option<String> = row
        .try_get("running_worker_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_at: DateTime<Utc> = row
        .try_get("updated_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let updated_by_str: Option<String> = row
        .try_get("updated_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let version: i64 = row
        .try_get("version")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<PipelineScheduleMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed pipeline_schedule id: {e}")))?;
    let updated_by = updated_by_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed updated_by: {e}")))
        })
        .transpose()?;

    Ok(PipelineSchedule {
        id,
        pipeline_kind,
        cron_expression,
        enabled,
        timezone,
        last_run_at,
        next_run_at,
        config,
        running_lock_acquired_at,
        running_worker_id,
        updated_at,
        updated_by,
        version,
    })
}

/// `PgRow` → [`PipelineRun`] 변환 (14 컬럼 round-trip, `u64` ↔ `i64` saturate).
fn row_to_run(row: &PgRow) -> Result<PipelineRun, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let schedule_id_str: String = row
        .try_get("schedule_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let started_at: DateTime<Utc> = row
        .try_get("started_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let finished_at: Option<DateTime<Utc>> = row
        .try_get("finished_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let status_str: String = row
        .try_get("status")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let items_processed_i64: i64 = row
        .try_get("items_processed")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let items_changed_i64: i64 = row
        .try_get("items_changed")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let output_hashes: serde_json::Value = row
        .try_get("output_hashes")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let error_message: Option<String> = row
        .try_get("error_message")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let triggered_by_str: String = row
        .try_get("triggered_by")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let triggered_by_user_str: Option<String> = row
        .try_get("triggered_by_user")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let correlation_id: String = row
        .try_get("correlation_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let log_url: Option<String> = row
        .try_get("log_url")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let steps: serde_json::Value = row
        .try_get("steps")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<PipelineRunMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed pipeline_run id: {e}")))?;
    let schedule_id = Id::<PipelineScheduleMarker>::try_from_str(schedule_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed schedule_id: {e}")))?;
    let status = parse_run_status(&status_str)?;
    let triggered_by = parse_trigger_kind(&triggered_by_str)?;
    let triggered_by_user = triggered_by_user_str
        .map(|s| {
            Id::<UserMarker>::try_from_str(s.trim())
                .map_err(|e| RepoError::Database(format!("malformed triggered_by_user: {e}")))
        })
        .transpose()?;

    // 음수 DB 값은 비정상 — 도메인 `0` 으로 fallback (안전 saturate).
    let items_processed = u64::try_from(items_processed_i64).unwrap_or(0);
    let items_changed = u64::try_from(items_changed_i64).unwrap_or(0);

    Ok(PipelineRun {
        id,
        schedule_id,
        started_at,
        finished_at,
        status,
        items_processed,
        items_changed,
        output_hashes,
        error_message,
        triggered_by,
        triggered_by_user,
        correlation_id,
        log_url,
        steps,
    })
}

#[async_trait]
impl PipelineRepository for PgPipelineRepository {
    // ── PipelineSchedule ────────────────────────────────────────

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

    /// 트랜잭션 안에서 `pipeline_schedule` + `audit_log` + `outbox_event` 를 함께 저장.
    ///
    /// `OCC` 는 `ON CONFLICT (id) DO UPDATE … WHERE version = $version` 로
    /// 강제. `rows_affected() == 0` → INSERT 도 UPDATE 도 적용 안 됨 →
    /// [`RepoError::Conflict`] (`tx` `Drop` 으로 audit/outbox 도 자동 rollback).
    ///
    /// `MutationContext` 매핑 (T5/T6/T7/T8 와 동일):
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL`, 시스템 액션
    ///   = pipeline scheduler/워커 자체 mutation)
    /// - `ctx.action` → `audit_log.action`
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet`)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id`
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    /// - `ctx.events` → 각 이벤트마다 `outbox_event` row
    ///   (`aggregate_kind = 'pipeline_schedule'`)
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

        // 1. UPSERT pipeline_schedule — OCC via WHERE version = $13.
        //    `pipeline_kind` 는 immutable (UNIQUE) — DO UPDATE 절에 포함하지 않음.
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
            // INSERT 도 UPDATE 도 적용 안 됨 → OCC 충돌. tx Drop → 자동 rollback.
            return Err(RepoError::Conflict);
        }

        // 2. INSERT audit_log — resource_kind = 'pipeline_schedule'
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

        // 3. INSERT outbox_event — aggregate_kind = 'pipeline_schedule'
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

    // ── PipelineRun ─────────────────────────────────────────────

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
        // pipeline_run_schedule_time_idx (schedule_id, started_at desc) 활용.
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
        // pipeline_run_running_idx (started_at) where status = 'running' 활용.
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

    /// 트랜잭션 안에서 `pipeline_run` + `audit_log` + `outbox_event` 를 함께 저장.
    ///
    /// `version` 컬럼이 없어요 (no `OCC`) — `INSERT … ON CONFLICT (id) DO
    /// UPDATE …` (조건 없음) 으로 신규/업데이트 모두 1행 적용. `started_at` /
    /// `schedule_id` / `triggered_by` / `triggered_by_user` / `correlation_id`
    /// 는 immutable so `DO UPDATE` 절에 포함하지 않아요. 어느 단계든 실패 시
    /// `tx` `Drop` 자동 rollback.
    ///
    /// `MutationContext` 매핑:
    /// - `ctx.actor_id` → `audit_log.actor_id` (`None` → `NULL`, 시스템 액션
    ///   — 워커가 자기 실행을 기록하는 경우 일반적)
    /// - `ctx.action` → `audit_log.action` (예: `"create"` / `"complete"` /
    ///   `"fail"`)
    /// - `ctx.metadata` → `audit_log.after_state`
    /// - `ctx.client_ip` → `audit_log.ip_address` (`$N::inet`)
    /// - `ctx.user_agent` → `audit_log.user_agent`
    /// - `ctx.correlation_id` → `audit_log.correlation_id`
    /// - `ctx.occurred_at` → `audit_log.created_at` (`None` → `Utc::now()`)
    /// - `ctx.events` → 각 이벤트마다 `outbox_event` row
    ///   (`aggregate_kind = 'pipeline_run'`)
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

        // 도메인 `u64` → DB `bigint` (`i64`) 변환 — overflow 시 `i64::MAX`
        // saturate (현실적으로 도달 불가 — 항목 수 9.2e18 = 920경 건).
        let items_processed_i64 = i64::try_from(run.items_processed).unwrap_or(i64::MAX);
        let items_changed_i64 = i64::try_from(run.items_changed).unwrap_or(i64::MAX);

        // 1. UPSERT pipeline_run — no OCC, no version 컬럼.
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

        // 2. INSERT audit_log — resource_kind = 'pipeline_run'
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

        // 3. INSERT outbox_event — aggregate_kind = 'pipeline_run'
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
