//! `SearchHistoryRepository` port. 구현체는 sub-project 5.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{Id, UserMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::entity::SearchHistory;

/// `SearchHistory` 저장/조회 포트.
#[async_trait]
pub trait SearchHistoryRepository: Send + Sync {
    /// 사용자의 검색 이력 (최신 순, 90일 이내).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_recent_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<SearchHistory>, RepoError>;

    /// 단일 검색 기록 `INSERT` (대량 — 매 검색마다).
    ///
    /// `ctx` 의 actor/action/events 가 같은 트랜잭션 안에서 `audit_log` 와
    /// `outbox_event` 로 자동 기록돼요 (SP5-ii transactional 패턴).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn insert(&self, history: &SearchHistory, ctx: MutationContext) -> Result<(), RepoError>;

    /// `PIPA` 가명화 — `created_at < cutoff`인 모든 기록의 `user_id` → `NULL`.
    ///
    /// 90일 retention 워커가 호출. 결과는 가명화된 row 수.
    /// bulk operation 이라 `ctx.action` 은 system action 권장 — `audit_log` 에
    /// 단일 row 만 기록되며 `metadata` 에 `rows_pseudonymized` 카운트 보존.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn pseudonymize_older_than(
        &self,
        cutoff: DateTime<Utc>,
        ctx: MutationContext,
    ) -> Result<u64, RepoError>;
}

/// `Repository` 에러.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 대상 미존재.
    #[error("not found")]
    NotFound,
    /// DB 통신/SQL 에러 (정보 누설 방지로 메시지만).
    #[error("database error: {0}")]
    Database(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn assert_obj_safe(_repo: &dyn SearchHistoryRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }
}
