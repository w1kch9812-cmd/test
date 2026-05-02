//! `PipelineRun` Aggregate — 실행 1건 + 상태 머신 + `steps` JSONB.
//!
//! Spec § 5.4 `pipeline_run` 테이블 매핑.
//!
//! ## 상태 머신
//!
//! `Running` (initial) → `Success` / `SkippedUnchanged` / `Failed` / `Aborted` (terminal).
//! 터미널 상태 도달 후 `complete_run` / `fail_run` / `abort_run` /
//! `add_step` / `complete_step` / `fail_step` 호출 시 [`PipelineError::AlreadyTerminal`].

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, PipelineRunMarker, PipelineScheduleMarker, UserMarker};

use crate::errors::PipelineError;
use crate::status::RunStatus;
use crate::trigger_kind::TriggerKind;

/// `correlation_id` 최대 길이 (spec § 5.4 `varchar(30)`).
const MAX_CORRELATION_ID_LEN: usize = 30;
/// `error_message` 최대 길이 (spec § 5.4 `text` — 도메인 제한 2000자).
const MAX_ERROR_MESSAGE_LEN: usize = 2000;
/// `log_url` 최대 길이 (spec § 5.4 `text` — 도메인 제한 500자).
const MAX_LOG_URL_LEN: usize = 500;

/// 데이터 파이프라인 실행 1건 (`PipelineSchedule` 1건당 N).
///
/// 13 필드 — spec § 5.4 `pipeline_run` 매핑.
///
/// ## `steps` JSONB
///
/// 단계별 진행 + 결과 (UI 시각화용). 각 step 은 `{order, name, status, started_at, ...}` 형식.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PipelineRun {
    /// 식별자 (`plr_<26 ULID>`).
    pub id: Id<PipelineRunMarker>,
    /// 부모 스케줄.
    pub schedule_id: Id<PipelineScheduleMarker>,
    /// 실행 시작 시각.
    pub started_at: DateTime<Utc>,
    /// 실행 종료 시각 (terminal 상태일 때만 `Some`).
    pub finished_at: Option<DateTime<Utc>>,
    /// 실행 상태.
    pub status: RunStatus,
    /// 처리한 항목 수.
    pub items_processed: u64,
    /// 변경된 항목 수.
    pub items_changed: u64,
    /// 시도별/스텝별 output hash (`JSONB` default `'{}'`).
    pub output_hashes: serde_json::Value,
    /// 실패 메시지 (≤2000자).
    pub error_message: Option<String>,
    /// 트리거 종류.
    pub triggered_by: TriggerKind,
    /// 트리거한 사용자 (`Manual` 일 때 보통 `Some`).
    pub triggered_by_user: Option<Id<UserMarker>>,
    /// 분산 추적 `correlation_id` (≤30자, 비어있지 않음).
    pub correlation_id: String,
    /// 로그 링크 (Loki / `CloudWatch` 등, ≤500자).
    pub log_url: Option<String>,
    /// 단계별 진행 (`JSONB` default `'[]'`).
    pub steps: serde_json::Value,
}

impl PipelineRun {
    /// 새 `PipelineRun` 시작 — `status = Running`, `steps = []`.
    ///
    /// # Errors
    ///
    /// - `correlation_id` 빈/30자 초과 → [`PipelineError::EmptyCorrelationId`] / [`PipelineError::CorrelationIdTooLong`].
    #[allow(clippy::too_many_arguments)] // 의도된 풀 생성자 (clippy.toml threshold = 5)
    pub fn try_new_started(
        id: Id<PipelineRunMarker>,
        schedule_id: Id<PipelineScheduleMarker>,
        triggered_by: TriggerKind,
        triggered_by_user: Option<Id<UserMarker>>,
        correlation_id: &str,
        at: DateTime<Utc>,
    ) -> Result<Self, PipelineError> {
        let correlation_id = validate_correlation_id(correlation_id)?;
        Ok(Self {
            id,
            schedule_id,
            started_at: at,
            finished_at: None,
            status: RunStatus::Running,
            items_processed: 0,
            items_changed: 0,
            output_hashes: serde_json::Value::Object(serde_json::Map::new()),
            error_message: None,
            triggered_by,
            triggered_by_user,
            correlation_id,
            log_url: None,
            steps: serde_json::Value::Array(Vec::new()),
        })
    }

    /// 새 step 을 `running` 상태로 추가.
    ///
    /// `step_payload` 는 caller 가 구성한 step 메타 (label, progress 등). `name` /
    /// `status` / `started_at` 은 자동 주입.
    ///
    /// # Errors
    ///
    /// - `step_name` 빈 → [`PipelineError::EmptyStepName`].
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn add_step(
        &mut self,
        step_name: &str,
        step_payload: serde_json::Value,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        self.ensure_running()?;
        let step_name = validate_step_name(step_name)?;

        let mut entry = match step_payload {
            serde_json::Value::Object(map) => map,
            _ => serde_json::Map::new(),
        };
        entry.insert("name".to_owned(), serde_json::Value::String(step_name));
        entry.insert(
            "status".to_owned(),
            serde_json::Value::String("running".to_owned()),
        );
        entry.insert(
            "started_at".to_owned(),
            serde_json::Value::String(at.to_rfc3339()),
        );

        push_step(&mut self.steps, serde_json::Value::Object(entry));
        Ok(())
    }

    /// step 을 `success` 로 마킹 + `items_processed` / `items_changed` / `output_hash` 누적.
    ///
    /// # Errors
    ///
    /// - 해당 `step_name` 없음 → [`PipelineError::StepNotFound`].
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    #[allow(clippy::too_many_arguments)] // 의도된 — step 이름 + 누적 카운터 + hash + 시각
    pub fn complete_step(
        &mut self,
        step_name: &str,
        items_processed: u64,
        items_changed: u64,
        output_hash: Option<(String, String)>,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        self.ensure_running()?;
        let step_name = validate_step_name(step_name)?;
        update_step(&mut self.steps, &step_name, |entry| {
            entry.insert(
                "status".to_owned(),
                serde_json::Value::String("success".to_owned()),
            );
            entry.insert(
                "finished_at".to_owned(),
                serde_json::Value::String(at.to_rfc3339()),
            );
        })?;
        self.items_processed = self.items_processed.saturating_add(items_processed);
        self.items_changed = self.items_changed.saturating_add(items_changed);
        if let Some((key, hash)) = output_hash {
            if let serde_json::Value::Object(map) = &mut self.output_hashes {
                map.insert(key, serde_json::Value::String(hash));
            }
        }
        Ok(())
    }

    /// step 을 `failed` 로 마킹 + `error` 기록 (run status 는 변경하지 않아요).
    ///
    /// run 자체를 실패로 만들려면 별도로 [`PipelineRun::fail_run`] 호출.
    ///
    /// # Errors
    ///
    /// - 해당 `step_name` 없음 → [`PipelineError::StepNotFound`].
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn fail_step(
        &mut self,
        step_name: &str,
        error: &str,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        self.ensure_running()?;
        let step_name = validate_step_name(step_name)?;
        let error_owned = error.to_owned();
        update_step(&mut self.steps, &step_name, |entry| {
            entry.insert(
                "status".to_owned(),
                serde_json::Value::String("failed".to_owned()),
            );
            entry.insert(
                "finished_at".to_owned(),
                serde_json::Value::String(at.to_rfc3339()),
            );
            entry.insert("error".to_owned(), serde_json::Value::String(error_owned));
        })?;
        Ok(())
    }

    /// 실행 완료 (`Success`) — `finished_at` 설정. *immutable after* (이후 mutation 불가).
    ///
    /// # Errors
    ///
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn complete_run(&mut self, at: DateTime<Utc>) -> Result<(), PipelineError> {
        self.ensure_running()?;
        self.status = RunStatus::Success;
        self.finished_at = Some(at);
        Ok(())
    }

    /// 실행 완료 (`SkippedUnchanged`) — output hash 비교 결과 변경 없음.
    ///
    /// # Errors
    ///
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn complete_run_skipped_unchanged(
        &mut self,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        self.ensure_running()?;
        self.status = RunStatus::SkippedUnchanged;
        self.finished_at = Some(at);
        Ok(())
    }

    /// 실행 실패 (`Failed`) — `error_message` 기록 + `finished_at` 설정.
    ///
    /// # Errors
    ///
    /// - `error_message` 2000자 초과 → [`PipelineError::ErrorMessageTooLong`].
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn fail_run(
        &mut self,
        error_message: &str,
        at: DateTime<Utc>,
    ) -> Result<(), PipelineError> {
        self.ensure_running()?;
        let len = error_message.chars().count();
        if len > MAX_ERROR_MESSAGE_LEN {
            return Err(PipelineError::ErrorMessageTooLong { actual: len });
        }
        self.status = RunStatus::Failed;
        self.error_message = Some(error_message.to_owned());
        self.finished_at = Some(at);
        Ok(())
    }

    /// 실행 중단 (`Aborted`) — 외부 중단 명령.
    ///
    /// # Errors
    ///
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn abort_run(&mut self, at: DateTime<Utc>) -> Result<(), PipelineError> {
        self.ensure_running()?;
        self.status = RunStatus::Aborted;
        self.finished_at = Some(at);
        Ok(())
    }

    /// 로그 링크 설정 + 검증.
    ///
    /// # Errors
    ///
    /// - `log_url` 500자 초과 → [`PipelineError::LogUrlTooLong`].
    /// - 이미 터미널 상태 → [`PipelineError::AlreadyTerminal`].
    pub fn set_log_url(&mut self, log_url: &str) -> Result<(), PipelineError> {
        self.ensure_running()?;
        let len = log_url.chars().count();
        if len > MAX_LOG_URL_LEN {
            return Err(PipelineError::LogUrlTooLong { actual: len });
        }
        self.log_url = Some(log_url.to_owned());
        Ok(())
    }

    const fn ensure_running(&self) -> Result<(), PipelineError> {
        if self.status.is_terminal() {
            return Err(PipelineError::AlreadyTerminal(self.status.as_str()));
        }
        Ok(())
    }
}

fn validate_correlation_id(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyCorrelationId);
    }
    let len = trimmed.chars().count();
    if len > MAX_CORRELATION_ID_LEN {
        return Err(PipelineError::CorrelationIdTooLong { actual: len });
    }
    Ok(trimmed)
}

fn validate_step_name(value: &str) -> Result<String, PipelineError> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        return Err(PipelineError::EmptyStepName);
    }
    Ok(trimmed)
}

fn push_step(steps: &mut serde_json::Value, entry: serde_json::Value) {
    if let serde_json::Value::Array(arr) = steps {
        arr.push(entry);
    } else {
        *steps = serde_json::Value::Array(vec![entry]);
    }
}

fn update_step<F>(
    steps: &mut serde_json::Value,
    step_name: &str,
    mutator: F,
) -> Result<(), PipelineError>
where
    F: FnOnce(&mut serde_json::Map<String, serde_json::Value>),
{
    let serde_json::Value::Array(arr) = steps else {
        return Err(PipelineError::StepNotFound(step_name.to_owned()));
    };
    for entry in arr.iter_mut().rev() {
        if let serde_json::Value::Object(map) = entry {
            if map.get("name").and_then(serde_json::Value::as_str) == Some(step_name) {
                mutator(map);
                return Ok(());
            }
        }
    }
    Err(PipelineError::StepNotFound(step_name.to_owned()))
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
