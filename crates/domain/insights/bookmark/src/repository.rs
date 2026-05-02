//! `BookmarkRepository` port. 구현체는 sub-project 5.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{BookmarkExternalMarker, Id, ListingMarker, UserMarker};
use thiserror::Error;

use crate::external::BookmarkExternal;
use crate::listing::BookmarkListing;

/// `Bookmark` 저장/조회 포트.
#[async_trait]
pub trait BookmarkRepository: Send + Sync {
    /// 사용자의 매물 북마크 목록.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_listing_bookmarks(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<BookmarkListing>, RepoError>;

    /// 사용자의 외부 북마크 목록.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_external_bookmarks(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<BookmarkExternal>, RepoError>;

    /// 매물 북마크 저장 (`UPSERT`). 동일 `(user_id, listing_id)` 중복 시 업데이트.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save_listing_bookmark(&self, bm: &BookmarkListing) -> Result<(), RepoError>;

    /// 외부 북마크 저장 (`INSERT` 또는 `UPDATE`).
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn save_external_bookmark(&self, bm: &BookmarkExternal) -> Result<(), RepoError>;

    /// 매물 북마크 삭제.
    ///
    /// # Errors
    ///
    /// 대상 미존재 → [`RepoError::NotFound`].
    /// DB 통신 실패 → [`RepoError::Database`].
    async fn delete_listing_bookmark(
        &self,
        user_id: &Id<UserMarker>,
        listing_id: &Id<ListingMarker>,
    ) -> Result<(), RepoError>;

    /// 외부 북마크 삭제.
    ///
    /// # Errors
    ///
    /// 대상 미존재 → [`RepoError::NotFound`].
    /// DB 통신 실패 → [`RepoError::Database`].
    async fn delete_external_bookmark(
        &self,
        id: &Id<BookmarkExternalMarker>,
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
    fn assert_obj_safe(_repo: &dyn BookmarkRepository) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }
}
