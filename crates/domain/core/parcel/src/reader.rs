//! `ParcelReader` port. 구현체는 sub-project 4 (`crates/data-clients/r2-public-data/`).

// `ParcelReader` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::geometry::PointSrid;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::pnu::Pnu;

use crate::entity::Parcel;
use crate::errors::ReaderError;

/// `Parcel` 조회 포트 (`R2` 정적).
#[async_trait]
pub trait ParcelReader: Send + Sync {
    /// 단일 필지 조회 (`PMTiles` spatial index 또는 `JSON` 인덱스).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError>;

    /// 지도 영역 내 마커 (lightweight projection — 풀 Aggregate fetch 없이).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_markers_in_bbox(
        &self,
        bbox: &BoundingBox,
    ) -> Result<Vec<ParcelMarker>, ReaderError>;
}

/// `Parcel` 지도 마커 — 지도 렌더용 경량 데이터.
///
/// 풀 Aggregate 대신 4 필드만 (`PMTiles`에서 직접 추출 가능).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParcelMarker {
    /// 필지 식별자.
    pub pnu: Pnu,
    /// 폴리곤 중심점 (`WGS84`).
    pub centroid: PointSrid,
    /// 면적 (`m²`).
    pub area: AreaM2,
    /// 지목.
    pub land_use_type: LandUseType,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn sample_marker() -> ParcelMarker {
        ParcelMarker {
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            centroid: PointSrid::try_new_wgs84(126.9784, 37.5666).unwrap(),
            area: AreaM2::try_new(250.0).unwrap(),
            land_use_type: LandUseType::Building,
        }
    }

    #[test]
    fn marker_constructs_with_all_fields() {
        let m = sample_marker();
        assert_eq!(m.pnu.as_str(), "1111010100100010000");
        assert_eq!(m.land_use_type, LandUseType::Building);
    }

    #[test]
    fn marker_serde_roundtrip() {
        let m = sample_marker();
        let json = serde_json::to_string(&m).expect("serialize");
        let back: ParcelMarker = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(m, back);
    }

    #[test]
    fn marker_clone_preserves_fields() {
        let m = sample_marker();
        let cloned = m.clone();
        assert_eq!(m, cloned);
    }
}
