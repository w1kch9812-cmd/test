//! `AuditLogRepository` port. **`INSERT`-only** — V002 immutable trigger 가 `UPDATE`/`DELETE` 차단.
//!
//! 구현체는 sub-project 5 에서 추가해요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{Id, UserMarker};
use thiserror::Error;

use crate::entity::AuditLog;

/// `AuditLog` 저장/조회 포트. **append-only** — `save`/`update`/`delete` *없음*.
#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    /// 단일 `AuditLog` `INSERT` — append-only.
    ///
    /// V002 immutable trigger 가 같은 `id` 에 대한 `UPDATE`/`DELETE` 를 DB 레벨에서 차단해요.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn insert(&self, log: &AuditLog) -> Result<(), RepoError>;

    /// `resource_kind` + `resource_id` 로 `AuditLog` 조회.
    ///
    /// 결과는 `created_at` `DESC`, 최대 `limit` 건. admin audit 화면에서 자주 사용.
    /// composite index `audit_log_resource_idx (resource_kind, resource_id, created_at desc)` 활용.
    ///
    /// # Errors
    ///
    /// `DB` 통신 실패 시 [`RepoError::Database`].
    async fn find_by_resource(
        &self,
        resource_kind: &str,
        resource_id: &str,
        limit: u32,
    ) -> Result<Vec<AuditLog>, RepoError>;

    /// 특정 사용자가 일으킨 `AuditLog` 조회 (`since` 시점부터).
    ///
    /// 결과는 `created_at` `DESC`, 최대 `limit` 건. admin 의 사용자별 활동 추적용.
    /// partial index `audit_log_actor_idx (actor_id, created_at desc) where actor_id is not null` 활용.
    ///
    /// # Errors
    ///
    /// `DB` 통신 실패 시 [`RepoError::Database`].
    async fn find_by_actor(
        &self,
        actor_id: &Id<UserMarker>,
        since: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AuditLog>, RepoError>;

    /// `correlation_id` 로 조회 (분산 추적 — 한 요청의 모든 `AuditLog` 묶임).
    ///
    /// 시간순 (오래된 순).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_correlation_id(
        &self,
        correlation_id: &str,
    ) -> Result<Vec<AuditLog>, RepoError>;

    // *NO save/update/delete* — V002 immutable trigger 가 차단.
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
    fn assert_obj_safe(_repo: &dyn AuditLogRepository) {}

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
