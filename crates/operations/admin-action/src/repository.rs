//! `AdminActionRepository` port. **`INSERT`-only** — admin 액션은 immutable
//! (`AuditLog` 와 같은 설계).
//!
//! 구현체는 sub-project 5 에서 추가해요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{Id, UserMarker};
use thiserror::Error;

use crate::entity::AdminAction;

/// `AdminAction` 저장/조회 포트. **append-only** — `save`/`update`/`delete` *없음*.
#[async_trait]
pub trait AdminActionRepository: Send + Sync {
    /// 단일 `AdminAction` `INSERT` — append-only.
    ///
    /// admin 액션은 immutable — 한 번 기록되면 수정/삭제 불가.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn insert(&self, action: &AdminAction) -> Result<(), RepoError>;

    /// 어드민 사용자별 `AdminAction` 조회 (composite index
    /// `admin_action_admin_idx (admin_id, created_at desc)` 활용).
    ///
    /// `since` 이후의 row 만, 최신 순, 최대 `limit` 건.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_admin(
        &self,
        admin_id: &Id<UserMarker>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AdminAction>, RepoError>;

    /// 대상 리소스별 `AdminAction` 조회 (composite index
    /// `admin_action_target_idx (target_kind, target_id)` 활용).
    ///
    /// 최신 순. 결과는 최대 `limit` 건.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_target(
        &self,
        target_kind: &str,
        target_id: &str,
        limit: u32,
    ) -> Result<Vec<AdminAction>, RepoError>;

    /// `correlation_id` 로 조회 (분산 추적 — 한 어드민 작업의 모든 액션 묶임).
    ///
    /// 시간순 (오래된 순).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<AdminAction>, RepoError>;

    // *NO save/update/delete* — admin 액션은 immutable.
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
    fn assert_obj_safe(_repo: &dyn AdminActionRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }

    #[test]
    fn repo_error_messages() {
        assert_eq!(RepoError::NotFound.to_string(), "not found");
        assert_eq!(
            RepoError::Database("oops".to_owned()).to_string(),
            "database error: oops"
        );
    }
}
