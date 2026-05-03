//! `OperationsMetaRepository` port — `FeaturedContent` + `SystemAlert` 합친 1 trait.
//!
//! 두 Aggregate 모두 Operations BC 의 *meta* 데이터 (워크플로우 X) 이고 `version`
//! OCC 컬럼이 없어 단일 trait + 단순 [`RepoError`] (no `Conflict`) 로 묶었어요.
//! 구현체는 sub-project 5 (`crates/db`) 에서 추가해요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use shared_kernel::id::{FeaturedContentMarker, Id, SystemAlertMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::alert::SystemAlert;
use crate::featured::{FeaturedContent, FeaturedContentFeatureKind};

/// `FeaturedContent` + `SystemAlert` 저장/조회 포트.
#[async_trait]
pub trait OperationsMetaRepository: Send + Sync {
    // ── FeaturedContent ───────────────────────────────────────

    /// 저장 (`INSERT` or `UPDATE`). 버전 컬럼이 없으므로 OCC 충돌은 없어요.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save_featured(
        &self,
        fc: &FeaturedContent,
        ctx: MutationContext,
    ) -> Result<(), RepoError>;

    /// `id` 로 단건 조회. 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_featured_by_id(
        &self,
        id: &Id<FeaturedContentMarker>,
    ) -> Result<Option<FeaturedContent>, RepoError>;

    /// 특정 시각 `at` 에 활성인 `feature_kind` 슬롯의 콘텐츠
    /// (`starts_at <= at < ends_at`) 를 weight 내림차순으로 반환.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_active_featured(
        &self,
        feature_kind: FeaturedContentFeatureKind,
        at: DateTime<Utc>,
    ) -> Result<Vec<FeaturedContent>, RepoError>;

    // ── SystemAlert ───────────────────────────────────────────

    /// 저장 (`INSERT` or `UPDATE`).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save_alert(&self, alert: &SystemAlert, ctx: MutationContext) -> Result<(), RepoError>;

    /// `id` 로 단건 조회. 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_alert_by_id(
        &self,
        id: &Id<SystemAlertMarker>,
    ) -> Result<Option<SystemAlert>, RepoError>;

    /// 미 acknowledge 된 알림 (acknowledged_at IS NULL) 을 오래된 순(`created_at`
    /// ASC)으로 최대 `limit` 건 조회 (어드민 워크큐용).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_unacknowledged_alerts(&self, limit: u32) -> Result<Vec<SystemAlert>, RepoError>;
}

/// `Repository` 에러. **No `Conflict` variant** — 두 Aggregate 모두 OCC 미사용.
#[derive(Debug, Error)]
pub enum RepoError {
    /// 대상 Aggregate 미존재.
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
    fn assert_obj_safe(_repo: &dyn OperationsMetaRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }

    #[test]
    fn repo_error_not_found_message() {
        assert_eq!(RepoError::NotFound.to_string(), "not found");
    }

    #[test]
    fn repo_error_database_message() {
        assert_eq!(
            RepoError::Database("connection refused".to_owned()).to_string(),
            "database error: connection refused"
        );
    }
}
