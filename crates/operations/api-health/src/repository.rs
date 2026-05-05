//! `HealthCheckRepository` — port trait + `RepoError`.

use async_trait::async_trait;
use thiserror::Error;

use crate::entity::{HealthCheckRecord, NewHealthCheck};

/// 도메인 레벨 repository 에러.
///
/// 인프라 (`crates/db`) 가 sqlx 에러를 흡수해 본 enum 으로 변환.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 데이터 무결성 위반 (CHECK constraint / NOT NULL / etc).
    #[error("integrity violation: {0}")]
    Integrity(String),

    /// DB 연결 / 쿼리 실패.
    #[error("database error: {0}")]
    Database(String),
}

/// `api_health_check` 테이블 access port.
#[async_trait]
pub trait HealthCheckRepository: Send + Sync {
    /// 새 record `INSERT`.
    ///
    /// # Errors
    ///
    /// - CHECK constraint / NOT NULL 위반 시 [`RepoError::Integrity`].
    /// - DB 통신 실패 시 [`RepoError::Database`].
    async fn record(&self, new: NewHealthCheck<'_>) -> Result<HealthCheckRecord, RepoError>;

    /// 가장 최근 N개 cron run 이 모두 fail 인가? (수동 trigger 무관)
    ///
    /// `n=3` 으로 호출 시: 최근 3개의 `cron_run=true` record 가 모두 `status != 'success'` 면 true.
    /// 정부 일시 장애 (5xx / timeout) 의 3일 연속 escalation detection 에 사용.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn is_n_cron_runs_failed(&self, api_name: &str, n: u32) -> Result<bool, RepoError>;

    /// 가장 최근 record (success / fail 무관, cron / 수동 무관).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_latest(&self, api_name: &str) -> Result<Option<HealthCheckRecord>, RepoError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn assert_obj_safe(_repo: &dyn HealthCheckRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }

    #[test]
    fn repo_error_integrity_message() {
        assert_eq!(
            RepoError::Integrity("status check failed".to_owned()).to_string(),
            "integrity violation: status check failed"
        );
    }

    #[test]
    fn repo_error_database_message() {
        assert_eq!(
            RepoError::Database("connection refused".to_owned()).to_string(),
            "database error: connection refused"
        );
    }
}
