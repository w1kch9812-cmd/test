//! `BuildingReader` port. 구현체는 sub-project 4 (`crates/data-clients/r2-public-data/`).

// `BuildingReader` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::pnu::Pnu;

use crate::entity::Building;
use crate::errors::ReaderError;

/// `Building` 조회 포트 (`R2` 정적).
///
/// 한 필지(`Pnu`)에 여러 건물 가능 → `fetch_by_pnu` 는 `Vec` 반환.
/// 단일 건물 식별은 `R2` 객체 키 (구현 sub-project 4 에서 정의).
#[async_trait]
pub trait BuildingReader: Send + Sync {
    /// 단일 `PNU` 의 모든 건물 (`PMTiles` spatial index 또는 `JSON` 인덱스).
    ///
    /// 매칭되는 건물이 없으면 빈 `Vec`. `R2` 자체 접근 실패만 에러로 반환.
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Vec<Building>, ReaderError>;

    /// 단일 건물 `ID` 로 조회 (`R2` 객체 키 기반).
    ///
    /// 미존재 시 `Ok(None)` — `NotFound` 는 hard-error 경로 (예: 인덱스 깨짐).
    ///
    /// # Errors
    ///
    /// 네트워크 실패 → `Fetch`. 데이터 파싱 실패 → `Parse`.
    async fn fetch_by_id(&self, building_id: &str) -> Result<Option<Building>, ReaderError>;
}

// Trait shape 검증만 — 실제 비동기 실행은 sub-project 4 구현체 테스트에서.
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{Building, BuildingReader, Pnu, ReaderError};
    use crate::purpose_code::BuildingPurposeCode;
    use crate::structure_code::BuildingStructureCode;
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

    fn sample_building(name: &str) -> Building {
        Building {
            pnu: Pnu::try_new("1111010100100010000").unwrap(),
            mgm_bldrgst_pk: format!("PK_{name}"),
            plat_plc: None,
            building_name: Some(name.to_owned()),
            main_purpose_code: BuildingPurposeCode::Factory,
            structure_code: BuildingStructureCode::Steel,
            plat_area_m2: None,
            arch_area_m2: None,
            building_coverage_ratio: None,
            total_floor_area_m2: AreaM2::try_new(1000.0).unwrap(),
            floor_area_ratio: None,
            ground_floors: 3,
            underground_floors: 0,
            height_m: Some(12.0),
            passenger_elevators: None,
            emergency_elevators: None,
            indoor_self_parking: None,
            outdoor_self_parking: None,
            annex_building_count: None,
            annex_building_area_m2: None,
            permit_date: None,
            construction_start_date: None,
            use_approval_date: None,
            geom: Some(sample_polygon()),
            fetched_at: Utc::now(),
        }
    }

    struct StubReader {
        buildings: Vec<Building>,
    }

    #[async_trait]
    impl BuildingReader for StubReader {
        async fn fetch_by_pnu(&self, _pnu: &Pnu) -> Result<Vec<Building>, ReaderError> {
            Ok(self.buildings.clone())
        }

        async fn fetch_by_id(&self, building_id: &str) -> Result<Option<Building>, ReaderError> {
            Ok(self
                .buildings
                .iter()
                .find(|b| b.building_name.as_deref() == Some(building_id))
                .cloned())
        }
    }

    /// `BuildingReader` 가 trait object 로 사용 가능한지 (`Send + Sync`) 컴파일 타임 검증.
    #[test]
    fn reader_is_object_safe() {
        fn assert_obj_safe<T: BuildingReader + ?Sized>() {}
        assert_obj_safe::<dyn BuildingReader>();
    }

    /// `StubReader` 가 trait 을 만족하는지 (Send + Sync 포함) 컴파일 타임 검증.
    #[test]
    fn stub_reader_implements_trait() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<StubReader>();
        let _r = StubReader {
            buildings: vec![sample_building("A동")],
        };
    }

    /// 샘플 건물 두 개로 `Vec<Building>` 일관성 (multi-building per parcel) 검증.
    #[test]
    fn sample_supports_multi_building_per_parcel() {
        let buildings = vec![sample_building("A동"), sample_building("B동")];
        assert_eq!(buildings.len(), 2);
        assert_eq!(buildings[0].pnu, buildings[1].pnu);
        assert_ne!(buildings[0].building_name, buildings[1].building_name);
    }
}
