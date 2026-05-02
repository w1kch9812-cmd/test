//! Data Pipeline Control 도메인 에러 — `PipelineSchedule` / `PipelineRun` 공용.

use thiserror::Error;

/// `PipelineSchedule` / `PipelineRun` Aggregate 검증 에러.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PipelineError {
    /// `pipeline_kind` 빈 (trim 후).
    #[error("pipeline_kind cannot be empty")]
    EmptyPipelineKind,
    /// `pipeline_kind` 50자 초과.
    #[error("pipeline_kind exceeds 50 chars (got {actual})")]
    PipelineKindTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `cron_expression` 빈 (trim 후).
    #[error("cron_expression cannot be empty")]
    EmptyCronExpression,
    /// `cron_expression` 100자 초과.
    #[error("cron_expression exceeds 100 chars (got {actual})")]
    CronExpressionTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `timezone` 빈 (trim 후).
    #[error("timezone cannot be empty")]
    EmptyTimezone,
    /// `timezone` 50자 초과.
    #[error("timezone exceeds 50 chars (got {actual})")]
    TimezoneTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `running_worker_id` 빈 (trim 후).
    #[error("running_worker_id cannot be empty")]
    EmptyWorkerId,
    /// `running_worker_id` 50자 초과.
    #[error("running_worker_id exceeds 50 chars (got {actual})")]
    WorkerIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `correlation_id` 빈 (trim 후).
    #[error("correlation_id cannot be empty")]
    EmptyCorrelationId,
    /// `correlation_id` 30자 초과.
    #[error("correlation_id exceeds 30 chars (got {actual})")]
    CorrelationIdTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `error_message` 2000자 초과.
    #[error("error_message exceeds 2000 chars (got {actual})")]
    ErrorMessageTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `log_url` 500자 초과.
    #[error("log_url exceeds 500 chars (got {actual})")]
    LogUrlTooLong {
        /// 실제 길이.
        actual: usize,
    },
    /// `step_name` 빈.
    #[error("step_name cannot be empty")]
    EmptyStepName,
    /// 스텝이 없음 (`complete_step` / `fail_step` 호출 시).
    #[error("step '{0}' not found in steps array")]
    StepNotFound(String),
    /// 이미 터미널 상태 (`complete_run` / `fail_run` / `abort_run` 재호출).
    #[error("pipeline run is already in terminal status: {0}")]
    AlreadyTerminal(&'static str),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_pipeline_kind_message() {
        assert_eq!(
            PipelineError::EmptyPipelineKind.to_string(),
            "pipeline_kind cannot be empty"
        );
    }

    #[test]
    fn pipeline_kind_too_long_message() {
        assert_eq!(
            PipelineError::PipelineKindTooLong { actual: 51 }.to_string(),
            "pipeline_kind exceeds 50 chars (got 51)"
        );
    }

    #[test]
    fn empty_cron_expression_message() {
        assert_eq!(
            PipelineError::EmptyCronExpression.to_string(),
            "cron_expression cannot be empty"
        );
    }

    #[test]
    fn cron_expression_too_long_message() {
        assert_eq!(
            PipelineError::CronExpressionTooLong { actual: 101 }.to_string(),
            "cron_expression exceeds 100 chars (got 101)"
        );
    }

    #[test]
    fn empty_timezone_message() {
        assert_eq!(
            PipelineError::EmptyTimezone.to_string(),
            "timezone cannot be empty"
        );
    }

    #[test]
    fn timezone_too_long_message() {
        assert_eq!(
            PipelineError::TimezoneTooLong { actual: 51 }.to_string(),
            "timezone exceeds 50 chars (got 51)"
        );
    }

    #[test]
    fn empty_worker_id_message() {
        assert_eq!(
            PipelineError::EmptyWorkerId.to_string(),
            "running_worker_id cannot be empty"
        );
    }

    #[test]
    fn worker_id_too_long_message() {
        assert_eq!(
            PipelineError::WorkerIdTooLong { actual: 51 }.to_string(),
            "running_worker_id exceeds 50 chars (got 51)"
        );
    }

    #[test]
    fn empty_correlation_id_message() {
        assert_eq!(
            PipelineError::EmptyCorrelationId.to_string(),
            "correlation_id cannot be empty"
        );
    }

    #[test]
    fn correlation_id_too_long_message() {
        assert_eq!(
            PipelineError::CorrelationIdTooLong { actual: 31 }.to_string(),
            "correlation_id exceeds 30 chars (got 31)"
        );
    }

    #[test]
    fn error_message_too_long_message() {
        assert_eq!(
            PipelineError::ErrorMessageTooLong { actual: 2001 }.to_string(),
            "error_message exceeds 2000 chars (got 2001)"
        );
    }

    #[test]
    fn log_url_too_long_message() {
        assert_eq!(
            PipelineError::LogUrlTooLong { actual: 501 }.to_string(),
            "log_url exceeds 500 chars (got 501)"
        );
    }

    #[test]
    fn empty_step_name_message() {
        assert_eq!(
            PipelineError::EmptyStepName.to_string(),
            "step_name cannot be empty"
        );
    }

    #[test]
    fn step_not_found_message() {
        assert_eq!(
            PipelineError::StepNotFound("fetch".to_owned()).to_string(),
            "step 'fetch' not found in steps array"
        );
    }

    #[test]
    fn already_terminal_message() {
        assert_eq!(
            PipelineError::AlreadyTerminal("success").to_string(),
            "pipeline run is already in terminal status: success"
        );
    }
}
