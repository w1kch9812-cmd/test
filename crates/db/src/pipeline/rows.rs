use std::str::FromStr;

use chrono::{DateTime, Utc};
use data_pipeline_control::repository::RepoError;
use data_pipeline_control::run::PipelineRun;
use data_pipeline_control::schedule::PipelineSchedule;
use data_pipeline_control::status::RunStatus;
use data_pipeline_control::trigger_kind::TriggerKind;
use shared_kernel::id::{Id, PipelineRunMarker, PipelineScheduleMarker, UserMarker};
use sqlx::postgres::PgRow;
use sqlx::Row;

/// Select-list for a full `pipeline_schedule` aggregate round-trip.
pub(super) const SCHEDULE_COLUMNS: &str = "id, pipeline_kind, cron_expression, enabled, timezone, \
    last_run_at, next_run_at, config, \
    running_lock_acquired_at, running_worker_id, \
    updated_at, updated_by, version";

/// Select-list for a full `pipeline_run` aggregate round-trip.
pub(super) const RUN_COLUMNS: &str = "id, schedule_id, started_at, finished_at, status, \
    items_processed, items_changed, output_hashes, error_message, \
    triggered_by, triggered_by_user, correlation_id, log_url, steps";

fn parse_run_status(s: &str) -> Result<RunStatus, RepoError> {
    RunStatus::from_str(s).map_err(|_| RepoError::Database(format!("unexpected run status: {s}")))
}

fn parse_trigger_kind(s: &str) -> Result<TriggerKind, RepoError> {
    TriggerKind::from_str(s)
        .map_err(|_| RepoError::Database(format!("unexpected trigger_kind: {s}")))
}

pub(super) fn row_to_schedule(row: &PgRow) -> Result<PipelineSchedule, RepoError> {
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

pub(super) fn row_to_run(row: &PgRow) -> Result<PipelineRun, RepoError> {
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
