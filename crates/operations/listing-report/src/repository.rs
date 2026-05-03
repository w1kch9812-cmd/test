//! `ListingReportRepository` port. **No OCC** — admin 신고 처리는 동시 충돌이
//! 드물어 단순 `INSERT`/`UPDATE` 로 처리해요. (BVQ/LRQ 와 다른 점.)
//!
//! 구현체는 sub-project 5 (`crates/db`) 에서 추가해요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{Id, ListingMarker, ListingReportMarker};
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::entity::ListingReport;

/// `ListingReport` 저장/조회 포트.
#[async_trait]
pub trait ListingReportRepository: Send + Sync {
    /// `id` 로 조회. 없으면 `Ok(None)`.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_id(
        &self,
        id: &Id<ListingReportMarker>,
    ) -> Result<Option<ListingReport>, RepoError>;

    /// 미처리 신고 (`Open` + `Investigating`) 를 오래된 순(`created_at` ASC)으로 최대
    /// `limit` 건 조회 (어드민 워크큐용).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_open(&self, limit: u32) -> Result<Vec<ListingReport>, RepoError>;

    /// 매물 ID 로 모든 신고 (terminal 포함) 조회.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingReport>, RepoError>;

    /// 저장 (`INSERT` or `UPDATE`). 버전 컬럼이 없으므로 OCC 충돌은 없어요.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save(&self, report: &ListingReport, ctx: MutationContext) -> Result<(), RepoError>;
}

/// `Repository` 에러. **No `Conflict` variant** — `ListingReport` 는 OCC 사용 안 함.
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
    fn assert_obj_safe(_repo: &dyn ListingReportRepository) {}

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
