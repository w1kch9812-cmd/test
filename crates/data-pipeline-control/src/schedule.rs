//! `PipelineSchedule` Aggregate — cron 기반 + optimistic locking + 워커 실행 락.
//!
//! Spec § 5.4 `pipeline_schedule` 테이블 매핑.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, PipelineScheduleMarker, UserMarker};

use crate::errors::PipelineError;

/// `pipeline_kind` 최대 길이 (spec § 5.4 `varchar(50)`).
const MAX_PIPELINE_KIND_LEN: usize = 50;
/// `cron_expression` 최대 길이 (spec § 5.4 `varchar(100)`).
const MAX_CRON_EXPRESSION_LEN: usize = 100;
/// `timezone` 최대 길이 (spec § 5.4 `varchar(50)`).
const MAX_TIMEZONE_LEN: usize = 50;
/// `running_worker_id` 최대 길이 (spec § 5.4 `varchar(50)`).
const MAX_WORKER_ID_LEN: usize = 50;

/// 데이터 파이프라인 스케줄 (어드민 관리).
///
/// 13 필드 — spec § 5.4 `pipeline_schedule` 매핑.
///
/// ## 락 (running)
///
/// 워커 시작 시 [`PipelineSchedule::acquire_lock`] → `running_lock_acquired_at`
/// 와 `running_worker_id` 를 설정. 종료 시 [`PipelineSchedule::release_lock`] 으로 clear.
/// Postgres advisory lock 의 *보조* 메타데이터 — 어드민 UI 에서 stuck 워커 감지용.
///
/// ## Optimistic locking
///
/// `version` 은 어드민이 내용 (cron / config / enabled) 을 변경할 때만 bump.
/// 락/실행 메타데이터 (`acquire_lock`, `release_lock`, `record_run`) 는 bump 안 해요.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineSchedule {
    /// 식별자 (`pls_<26 ULID>`).
    pub id: Id<PipelineScheduleMarker>,
    /// 파이프라인 종류 (UNIQUE, ≤50자, 비어있지 않음).
    pub pipeline_kind: String,
    /// cron 표현식 (≤100자, 비어있지 않음). cron 문법 검증은 sub-project 4 에서.
    pub cron_expression: String,
    /// 활성 여부.
    pub enabled: bool,
    /// 타임존 (≤50자, 기본 `"Asia/Seoul"`).
    pub timezone: String,
    /// 마지막 실행 시작 시각 (latest [`PipelineRun::started_at`]).
    ///
    /// [`PipelineRun::started_at`]: crate::run::PipelineRun::started_at
    pub last_run_at: Option<DateTime<Utc>>,
    /// 다음 실행 예정 시각 (어드민 또는 cron 계산기가 설정).
    pub next_run_at: Option<DateTime<Utc>>,
    /// 파이프라인별 설정 (`JSONB` default `'{}'`).
    pub config: serde_json::Value,
    /// 락 획득 시각 (`Some` 이면 워커가 실행 중).
    pub running_lock_acquired_at: Option<DateTime<Utc>>,
    /// 락을 획득한 워커 ID (≤50자, `running_lock_acquired_at` 와 동기).
    pub running_worker_id: Option<String>,
    /// 마지막 갱신 시각.
    pub updated_at: DateTime<Utc>,
    /// 마지막 갱신한 어드민.
    pub updated_by: Option<Id<UserMarker>>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl PipelineSchedule {
    /// 검증 후 새 `PipelineSchedule` 생성. `running_lock` 없음, `version = 1`.
    ///
    /// # Errors
    ///
    /// - `pipeline_kind` 빈/50자 초과 → [`PipelineError::EmptyPipelineKind`] / [`PipelineError::PipelineKindTooLong`].
    /// - `cron_expression` 빈/100자 초과 → [`PipelineError::EmptyCronExpression`] / [`PipelineError::CronExpressionTooLong`].
    /// - `timezone` 빈/50자 초과 → [`PipelineError::EmptyTimezone`] / [`PipelineError::TimezoneTooLong`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자
    pub fn try_new(
        id: Id<PipelineScheduleMarker>,
        pipeline_kind: &str,
        cron_expression: &str,
        enabled: bool,
        timezone: &str,
        config: serde_json::Value,
        next_run_at: Option<DateTime<Utc>>,
        updated_by: Option<Id<UserMarker>>,
        now: DateTime<Utc>,
    ) -> Result<Self, PipelineError> {
        let pipeline_kind = validate_pipeline_kind(pipeline_kind)?;
        let cron_expression = validate_cron_expression(cron_expression)?;
        let timezone = validate_timezone(timezone)?;
        Ok(Self {
            id,
            pipeline_kind,
            cron_expression,
            enabled,
            timezone,
            last_run_at: None,
            next_run_at,
            config,
            running_lock_acquired_at: None,
            running_worker_id: None,
            updated_at: now,
            updated_by,
            version: 1,
        })
    }

    /// 활성화 + `version` bump + `updated_at` / `updated_by` 갱신.
    pub fn enable(&mut self, by: Option<Id<UserMarker>>, at: DateTime<Utc>) {
        self.enabled = true;
        self.updated_by = by;
        self.updated_at = at;
        self.version += 1;
    }

    /// 비활성화 + `version` bump + `updated_at` / `updated_by` 갱신.
    pub fn disable(&mut self, by: Option<Id<UserMarker>>, at: DateTime<Utc>) {
        self.enabled = false;
        self.updated_by = by;
        self.updated_at = at;
        self.version += 1;
    }

    /// 워커 락 획득. `running_lock_acquired_at` + `running_worker_id` 설정.
    ///
    /// `version` 은 bump 안 해요 (어드민 의미 있는 변경이 아니라 워커 메타데이터).
    ///
    /// # Errors
    ///
    /// - `worker_id` 빈/50자 초과 → [`PipelineError::EmptyWorkerId`] / [`PipelineError::WorkerIdTooLong`].
    pub fn acquire_lock(
        &mut self,
        worker_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        let worker_id = validate_worker_id(worker_id)?;
        self.running_lock_acquired_at = Some(at);
        self.running_worker_id = Some(worker_id);
        self.updated_at = at;
        Ok(())
    }

    /// 워커 락 해제. `running_lock_acquired_at` + `running_worker_id` clear.
    ///
    /// `version` bump 안 해요.
    pub fn release_lock(&mut self, at: DateTime<Utc>) {
        self.running_lock_acquired_at = None;
        self.running_worker_id = None;
        self.updated_at = at;
    }

    /// 실행 시작 기록 — `last_run_at` 갱신.
    ///
    /// `version` bump 안 해요. 워커가 [`PipelineRun`] 생성과 함께 호출.
    ///
    /// [`PipelineRun`]: crate::run::PipelineRun
    pub const fn record_run(&mut self, run_started_at: DateTime<Utc>) {
        self.last_run_at = Some(run_started_at);
        self.updated_at = run_started_at;
    }

    /// `config` 변경 + `version` bump + `updated_at` / `updated_by` 갱신.
    pub fn update_config(
        &mut self,
        config: serde_json::Value,
        by: Option<Id<UserMarker>>,
        at: DateTime<Utc>,
    ) {
        self.config = config;
        self.updated_by = by;
        self.updated_at = at;
        self.version += 1;
    }

    /// cron 표현식 + `next_run_at` 변경 + `version` bump + `updated_at` / `updated_by` 갱신.
    ///
    /// # Errors
    ///
    /// - `cron_expression` 빈/100자 초과 → [`PipelineError::EmptyCronExpression`] / [`PipelineError::CronExpressionTooLong`].
    pub fn update_cron(
        &mut self,
        cron_expression: &str,
        next_run_at: Option<DateTime<Utc>>,
        by: Option<Id<UserMarker>>,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        let cron_expression = validate_cron_expression(cron_expression)?;
        self.cron_expression = cron_expression;
        self.next_run_at = next_run_at;
        self.updated_by = by;
        self.updated_at = at;
        self.version += 1;
        Ok(())
    }
}

fn validate_pipeline_kind(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyPipelineKind);
    }
    let len = trimmed.chars().count();
    if len > MAX_PIPELINE_KIND_LEN {
        return Err(PipelineError::PipelineKindTooLong { actual: len });
    }
    Ok(trimmed)
}

fn validate_cron_expression(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyCronExpression);
    }
    let len = trimmed.chars().count();
    if len > MAX_CRON_EXPRESSION_LEN {
        return Err(PipelineError::CronExpressionTooLong { actual: len });
    }
    Ok(trimmed)
}

fn validate_timezone(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyTimezone);
    }
    let len = trimmed.chars().count();
    if len > MAX_TIMEZONE_LEN {
        return Err(PipelineError::TimezoneTooLong { actual: len });
    }
    Ok(trimmed)
}

fn validate_worker_id(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyWorkerId);
    }
    let len = trimmed.chars().count();
    if len > MAX_WORKER_ID_LEN {
        return Err(PipelineError::WorkerIdTooLong { actual: len });
    }
    Ok(trimmed)
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
