//! `AnalysisReportRepository` port. 구현체는 sub-project 5.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{AnalysisReportMarker, Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::entity::AnalysisReport;

/// `AnalysisReport` 저장/조회 포트.
#[async_trait]
pub trait AnalysisReportRepository: Send + Sync {
    /// 단건 조회.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_id(
        &self,
        id: &Id<AnalysisReportMarker>,
    ) -> Result<Option<AnalysisReport>, RepoError>;

    /// 사용자의 리포트 (최신 순, `limit` 만큼).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<AnalysisReport>, RepoError>;

    /// `INSERT` or `UPDATE`. Optimistic lock(`version`) 충돌 시 [`RepoError::Conflict`].
    ///
    /// `ctx` 의 actor/action/events 가 같은 트랜잭션 안에서 `audit_log` 와
    /// `outbox_event` 로 자동 기록돼요 (SP5-ii transactional 패턴).
    ///
    /// # Errors
    ///
    /// - 동시 갱신으로 `version`이 어긋난 경우 [`RepoError::Conflict`].
    /// - DB 통신 실패 시 [`RepoError::Database`].
    async fn save(&self, report: &AnalysisReport, ctx: MutationContext) -> Result<(), RepoError>;

    /// 삭제 (사용자 요청 또는 retention) — hard delete 도 audit 대상.
    ///
    /// # Errors
    ///
    /// - 대상 미존재 시 [`RepoError::NotFound`].
    /// - DB 통신 실패 시 [`RepoError::Database`].
    async fn delete(
        &self,
        id: &Id<AnalysisReportMarker>,
        ctx: MutationContext,
    ) -> Result<(), RepoError>;
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
    fn assert_obj_safe(_repo: &dyn AnalysisReportRepository) {}

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
