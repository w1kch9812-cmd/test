//! `ListingPhotoRepository` port (interface). 구현체는 sub-project 5.

// `ListingPhotoRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use thiserror::Error;

use crate::entity::ListingPhoto;

/// `ListingPhoto` 저장/조회 포트.
#[async_trait]
pub trait ListingPhotoRepository: Send + Sync {
    /// 매물의 활성 사진을 `display_order` 순으로 조회해요.
    /// (soft-deleted 제외, partial index 활용.)
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingPhoto>, RepoError>;

    /// 저장 (insert or update).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError>;

    /// 삭제 (hard delete — 일반 흐름은 `soft_delete` 후 별도 archive job).
    ///
    /// # Errors
    ///
    /// 대상 미존재 → [`RepoError::NotFound`].
    /// DB 통신 실패 → [`RepoError::Database`].
    async fn delete(&self, id: &Id<ListingPhotoMarker>) -> Result<(), RepoError>;
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
