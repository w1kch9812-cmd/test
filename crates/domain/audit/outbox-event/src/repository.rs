//! `OutboxRepository` port.
//!
//! Application handler 가 `Aggregate` 도메인 메서드 호출 후 emit 된 `DomainEvent` 를
//! `OutboxEvent` 로 wrap 해 *Aggregate save 와 같은 트랜잭션* 안에서 [`save`] 호출해요.
//! Publisher 워커 (sub-project 4) 는 [`fetch_unpublished`] 로 polling, 외부 발행 성공
//! 후 [`mark_published`] 호출.
//!
//! [`save`]: OutboxRepository::save
//! [`fetch_unpublished`]: OutboxRepository::fetch_unpublished
//! [`mark_published`]: OutboxRepository::mark_published
//!
//! 구현체는 sub-project 5 에서 추가해요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{Id, OutboxEventMarker};
use thiserror::Error;

use crate::entity::OutboxEvent;

/// `OutboxEvent` 저장/조회 포트.
///
/// 모든 구현체는 [`save`] 가 caller 의 트랜잭션에서 동작하도록 보장해야 해요
/// (Aggregate save 와 같은 트랜잭션이어야 transactional outbox 보장 성립).
///
/// [`save`]: OutboxRepository::save
#[async_trait]
pub trait OutboxRepository: Send + Sync {
    /// 단일 `OutboxEvent` `INSERT` (caller 트랜잭션 안에서).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save(&self, event: &OutboxEvent) -> Result<(), RepoError>;

    /// 미발행 (`published_at IS NULL`) 이벤트 polling — publisher 워커 사용.
    ///
    /// 결과는 `occurred_at` 오름차순, 최대 `limit` 건. spec § 5.3
    /// `outbox_unpublished_idx` partial index 활용.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn fetch_unpublished(&self, limit: u32) -> Result<Vec<OutboxEvent>, RepoError>;

    /// 발행 완료 마킹 — publisher 워커가 외부 발행 성공 후 호출.
    ///
    /// # Errors
    ///
    /// 대상 row 미존재 시 [`RepoError::NotFound`]. DB 통신 실패 시 [`RepoError::Database`].
    async fn mark_published(
        &self,
        id: &Id<OutboxEventMarker>,
        at: DateTime<Utc>,
    ) -> Result<(), RepoError>;
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
    fn assert_obj_safe(_repo: &dyn OutboxRepository) {}

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
