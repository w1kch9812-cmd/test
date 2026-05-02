//! `IndustrialComplexReader` port. 구현체는 sub-project 4 (`crates/data-clients/r2-public-data/`).

// `IndustrialComplexReader` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::admin_division::SigunguCode;
use shared_kernel::bounding_box::BoundingBox;

use crate::entity::IndustrialComplex;
use crate::errors::ReaderError;

/// `IndustrialComplex` 조회 포트 (`R2` 정적).
///
/// 산단 식별은 정부 표준 코드(예: `"I000001"`)로 해요.
/// 시군구 / `BoundingBox` 기반 다수 조회는 `Vec` 반환.
#[async_trait]
pub trait IndustrialComplexReader: Send + Sync {
    /// 단일 산단 코드로 조회.
    ///
    /// 미존재 시 `Ok(None)` — `NotFound` 는 hard-error 경로 (예: 인덱스 깨짐).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_code(&self, code: &str) -> Result<Option<IndustrialComplex>, ReaderError>;

    /// 시군구(`SigunguCode`)에 위치한 모든 산단.
    ///
    /// 매칭 없으면 빈 `Vec`. `R2` 자체 접근 실패만 에러로 반환.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_sigungu(
        &self,
        sigungu: &SigunguCode,
    ) -> Result<Vec<IndustrialComplex>, ReaderError>;

    /// 지도 영역(`BoundingBox`) 내 산단 (`WGS84`).
    ///
    /// 매칭 없으면 빈 `Vec`.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_in_bbox(
        &self,
        bbox: &BoundingBox,
    ) -> Result<Vec<IndustrialComplex>, ReaderError>;
}

// Trait shape 검증만 — 실제 비동기 실행은 sub-project 4 구현체 테스트에서.
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{
        BoundingBox, IndustrialComplex, IndustrialComplexReader, ReaderError, SigunguCode,
    };
    use crate::kind::IndustrialComplexKind;
    use async_trait::async_trait;
    use chrono::Utc;
    use geo_types::{Coord, LineString, Polygon as GeoPolygon};
    use shared_kernel::area::AreaM2;
    use shared_kernel::geometry::PolygonSrid;

    fn sample_polygon() -> PolygonSrid {
        let exterior = LineString(vec![
            Coord { x: 126.0, y: 37.0 },
            Coord { x: 127.0, y: 37.0 },
            Coord { x: 127.0, y: 38.0 },
            Coord { x: 126.0, y: 38.0 },
            Coord { x: 126.0, y: 37.0 },
        ]);
        PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![])).expect("valid")
    }

    fn sample_complex(code: &str) -> IndustrialComplex {
        IndustrialComplex {
            code: code.to_owned(),
            name: "샘플산단".to_owned(),
            kind: IndustrialComplexKind::General,
            sigungu: SigunguCode::try_new("28177").expect("valid"),
            designated_at: None,
            total_area_m2: AreaM2::try_new(500_000.0).unwrap(),
            geom: sample_polygon(),
            fetched_at: Utc::now(),
        }
    }

    struct StubReader {
        complexes: Vec<IndustrialComplex>,
    }

    #[async_trait]
    impl IndustrialComplexReader for StubReader {
        async fn fetch_by_code(
            &self,
            code: &str,
        ) -> Result<Option<IndustrialComplex>, ReaderError> {
            Ok(self.complexes.iter().find(|c| c.code == code).cloned())
        }

        async fn fetch_by_sigungu(
            &self,
            _sigungu: &SigunguCode,
        ) -> Result<Vec<IndustrialComplex>, ReaderError> {
            Ok(self.complexes.clone())
        }

        async fn fetch_in_bbox(
            &self,
            _bbox: &BoundingBox,
        ) -> Result<Vec<IndustrialComplex>, ReaderError> {
            Ok(self.complexes.clone())
        }
    }

    /// `IndustrialComplexReader` 가 trait object 로 사용 가능한지 (`Send + Sync`) 컴파일 타임 검증.
    #[test]
    fn reader_is_object_safe() {
        fn assert_obj_safe<T: IndustrialComplexReader + ?Sized>() {}
        assert_obj_safe::<dyn IndustrialComplexReader>();
    }

    /// `StubReader` 가 trait 을 만족하는지 (Send + Sync 포함) 컴파일 타임 검증.
    #[test]
    fn stub_reader_implements_trait() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StubReader>();
        let _r = StubReader {
            complexes: vec![sample_complex("I000001")],
        };
    }
}
