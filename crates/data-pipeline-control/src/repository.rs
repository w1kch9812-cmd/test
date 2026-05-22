//! `PipelineRepository` port — `PipelineSchedule` + `PipelineRun` 합친 1 trait.
//!
//! 두 Aggregate 가 항상 같이 사용되고 (워커가 schedule 잠금 + run INSERT 를 같은
//! 트랜잭션 안에서 수행) BC 가 1 개라 단일 trait 으로 묶었어요. 구현체는
//! sub-project 5 에서 추가.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{Id, PipelineRunMarker, PipelineScheduleMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::run::PipelineRun;
use crate::schedule::PipelineSchedule;

/// `PipelineSchedule` + `PipelineRun` 저장/조회 포트.
#[async_trait]
pub trait PipelineRepository: Send + Sync {
    // ── PipelineSchedule ──────────────────────────────────────

    /// `pipeline_kind` 로 단건 조회 (UNIQUE 컬럼).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_schedule_by_kind(
        &self,
        kind: &str,
    ) -> Result<Option<PipelineSchedule>, RepoError>;

    /// `id` 로 단건 조회.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_schedule_by_id(
        &self,
        id: &Id<PipelineScheduleMarker>,
    ) -> Result<Option<PipelineSchedule>, RepoError>;

    /// 모든 스케줄 (어드민 UI 목록).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn list_schedules(&self) -> Result<Vec<PipelineSchedule>, RepoError>;

    /// `INSERT` or `UPDATE`. Optimistic lock(`version`) 충돌 시 [`RepoError::Conflict`].
    ///
    /// # Errors
    ///
    /// - 동시 갱신으로 `version` 어긋남 → [`RepoError::Conflict`].
    /// - DB 통신 실패 → [`RepoError::Database`].
    async fn save_schedule(
        &self,
        schedule: &PipelineSchedule,
        ctx: MutationContext,
    ) -> Result<(), RepoError>;

    // ── PipelineRun ───────────────────────────────────────────

    /// `id` 로 단건 조회.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_run_by_id(
        &self,
        id: &Id<PipelineRunMarker>,
    ) -> Result<Option<PipelineRun>, RepoError>;

    /// 특정 스케줄의 최근 실행 (`started_at` desc, 최대 `limit` 건).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_recent_runs(
        &self,
        schedule_id: &Id<PipelineScheduleMarker>,
        limit: u32,
    ) -> Result<Vec<PipelineRun>, RepoError>;

    /// 진행 중 (`status = 'running'`) 인 모든 실행 — stuck 워커 감지 / 모니터링용.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_active_runs(&self) -> Result<Vec<PipelineRun>, RepoError>;

    /// `INSERT` or `UPDATE`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save_run(&self, run: &PipelineRun, ctx: MutationContext) -> Result<(), RepoError>;
}

/// `Repository` 에러.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 대상 미존재.
    #[error("not found")]
    NotFound,
    /// Optimistic lock 충돌 (동시 갱신).
    #[error("optimistic lock conflict")]
    Conflict,
    /// DB 통신/SQL 에러 (정보 누설 방지로 메시지만).
    #[error("database error: {0}")]
    Database(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn assert_obj_safe(_repo: &dyn PipelineRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }

    #[test]
    fn repo_error_messages() {
        assert_eq!(RepoError::NotFound.to_string(), "not found");
        assert_eq!(RepoError::Conflict.to_string(), "optimistic lock conflict");
        assert_eq!(
            RepoError::Database("oops".to_owned()).to_string(),
            "database error: oops"
        );
    }
}
