//! `RealTransactionReader` port. 구현체는 sub-project 4.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::NaiveDate;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::pnu::Pnu;

use crate::entity::RealTransaction;
use crate::errors::ReaderError;

/// `RealTransaction` 조회 포트 (`R2` 정적).
#[async_trait]
pub trait RealTransactionReader: Send + Sync {
    /// 단일 `PNU`의 모든 거래 (시간 순).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Vec<RealTransaction>, ReaderError>;

    /// 지도 영역 내 거래 (시점 필터).
    ///
    /// `since` 이후 거래일만 반환해요 — `R2` `PMTiles`는 거래일 단위 인덱스로 추정해요.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_in_bbox(
        &self,
        bbox: &BoundingBox,
        since: NaiveDate,
    ) -> Result<Vec<RealTransaction>, ReaderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `RealTransactionReader` is dyn-compatible (object-safe).
    #[allow(dead_code)]
    fn assert_obj_safe(_reader: &dyn RealTransactionReader) {}

    #[test]
    fn trait_is_object_safe() {
        // Compile-time check via above fn signature.
    }
}
