//! `IndustrialComplex` Aggregate (`R2` 정적, 8 필드).

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::admin_division::SigunguCode;
use shared_kernel::area::AreaM2;
use shared_kernel::geometry::PolygonSrid;

use crate::kind::IndustrialComplexKind;

/// `IndustrialComplex` Aggregate. `R2` 정적 — *read-only*, mutation 메서드 없음.
///
/// 한국 산업단지 (국가/일반/도시첨단/농공). 식별은 정부 표준 코드(`code`)로.
/// 면적/지정일 같은 invariant는 Reader 구현 시점 (sub-project 4) 에서 체크 —
/// Aggregate 자체는 `R2` 데이터를 그대로 표현해요.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndustrialComplex {
    /// 산단 식별자 (정부 표준 코드, 예: "I000001").
    pub code: String,
    /// 산단명 (≤200자 기대).
    pub name: String,
    /// 산단 종류 (4값).
    pub kind: IndustrialComplexKind,
    /// 위치 행정구역 (`SigunguCode`, 5자리).
    pub sigungu: SigunguCode,
    /// 지정일 (선택).
    pub designated_at: Option<NaiveDate>,
    /// 총 면적 (`m²`).
    pub total_area_m2: AreaM2,
    /// 산단 폴리곤 (`WGS84`).
    pub geom: PolygonSrid,
    /// `R2` 객체에서 fetch한 시각 (캐시 만료 판단용).
    pub fetched_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::IndustrialComplex;
    use crate::kind::IndustrialComplexKind;
    use chrono::{NaiveDate, Utc};
    use geo_types::{Coord, LineString, Polygon as GeoPolygon};
    use shared_kernel::admin_division::SigunguCode;
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

    #[test]
    fn industrial_complex_constructs_from_r2_data() {
        let ic = IndustrialComplex {
            code: "I000001".to_owned(),
            name: "남동국가산업단지".to_owned(),
            kind: IndustrialComplexKind::National,
            sigungu: SigunguCode::try_new("28177").expect("valid"),
            designated_at: Some(NaiveDate::from_ymd_opt(1985, 3, 15).unwrap()),
            total_area_m2: AreaM2::try_new(9_574_000.0).unwrap(),
            geom: sample_polygon(),
            fetched_at: Utc::now(),
        };
        assert_eq!(ic.code, "I000001");
        assert_eq!(ic.kind, IndustrialComplexKind::National);
        assert_eq!(ic.sigungu.as_str(), "28177");
        assert!(ic.designated_at.is_some());
    }

    #[test]
    fn industrial_complex_optional_fields_none() {
        let ic = IndustrialComplex {
            code: "I999999".to_owned(),
            name: "테스트산단".to_owned(),
            kind: IndustrialComplexKind::AgriculturalIndustrial,
            sigungu: SigunguCode::try_new("11110").expect("valid"),
            designated_at: None,
            total_area_m2: AreaM2::try_new(100_000.0).unwrap(),
            geom: sample_polygon(),
            fetched_at: Utc::now(),
        };
        assert!(ic.designated_at.is_none());
        assert_eq!(ic.kind, IndustrialComplexKind::AgriculturalIndustrial);
    }

    #[test]
    fn industrial_complex_serde_roundtrip() {
        let ic = IndustrialComplex {
            code: "I000123".to_owned(),
            name: "판교도시첨단산업단지".to_owned(),
            kind: IndustrialComplexKind::UrbanHighTech,
            sigungu: SigunguCode::try_new("41135").expect("valid"),
            designated_at: Some(NaiveDate::from_ymd_opt(2010, 7, 1).unwrap()),
            total_area_m2: AreaM2::try_new(450_000.0).unwrap(),
            geom: sample_polygon(),
            fetched_at: Utc::now(),
        };
        let json = serde_json::to_string(&ic).expect("serialize");
        let back: IndustrialComplex = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ic, back);
    }

    #[test]
    fn industrial_complex_clone_preserves_fields() {
        let ic = IndustrialComplex {
            code: "I000050".to_owned(),
            name: "시화일반산업단지".to_owned(),
            kind: IndustrialComplexKind::General,
            sigungu: SigunguCode::try_new("41271").expect("valid"),
            designated_at: Some(NaiveDate::from_ymd_opt(1988, 2, 20).unwrap()),
            total_area_m2: AreaM2::try_new(17_000_000.0).unwrap(),
            geom: sample_polygon(),
            fetched_at: Utc::now(),
        };
        let cloned = ic.clone();
        assert_eq!(ic, cloned);
    }
}
