//! `CourtAuctionReader` port. 구현체는 sub-project 4.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::bounding_box::BoundingBox;

use crate::entity::CourtAuction;
use crate::errors::ReaderError;

/// `CourtAuction` 조회 포트 (`R2` 정적).
#[async_trait]
pub trait CourtAuctionReader: Send + Sync {
    /// 사건번호로 단건 조회.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_case_number(
        &self,
        case_number: &str,
    ) -> Result<Option<CourtAuction>, ReaderError>;

    /// 활성 경매 (`Upcoming`/`InProgress`) 목록.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_active(&self) -> Result<Vec<CourtAuction>, ReaderError>;

    /// 지도 영역 내 경매 (활성 + 이력 모두 포함).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_in_bbox(&self, bbox: &BoundingBox) -> Result<Vec<CourtAuction>, ReaderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `CourtAuctionReader` is dyn-compatible (object-safe).
    #[allow(dead_code)]
    fn assert_obj_safe(_reader: &dyn CourtAuctionReader) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }
}
