//! `NotificationRepository` port. 구현체는 sub-project 5.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{Id, NotificationMarker, UserMarker};
use thiserror::Error;

use crate::entity::Notification;

/// `Notification` 저장/조회 포트.
#[async_trait]
pub trait NotificationRepository: Send + Sync {
    /// 사용자의 미읽음 알림 (최신 순).
    ///
    /// `notification_user_unread_idx` (`read_at IS NULL`) 부분 인덱스를 활용해요.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_unread_by_user(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<Notification>, RepoError>;

    /// 사용자의 모든 알림 (최신 순, 365일 이내).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_recent_by_user(
        &self,
        user_id: &Id<UserMarker>,
        limit: u32,
    ) -> Result<Vec<Notification>, RepoError>;

    /// 단일 알림 `INSERT` (대량 — 이벤트 발생 시).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn insert(&self, notification: &Notification) -> Result<(), RepoError>;

    /// 단일 알림 읽음 처리.
    ///
    /// 멱등 — 이미 읽은 알림이어도 에러가 아니고, `read_at`는 보존돼요
    /// (`UPDATE ... WHERE read_at IS NULL`).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn mark_read(
        &self,
        id: &Id<NotificationMarker>,
        at: DateTime<Utc>,
    ) -> Result<(), RepoError>;

    /// 사용자의 특정 `kind` 알림 모두 읽음 처리 (batch). 결과는 갱신된 row 수.
    ///
    /// 멱등 — 이미 읽은 row는 영향 없음 (`WHERE read_at IS NULL`).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn mark_all_read_by_kind(
        &self,
        user_id: &Id<UserMarker>,
        kind: &str,
        at: DateTime<Utc>,
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
    fn assert_obj_safe(_repo: &dyn NotificationRepository) {}

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
