//! 공간 좌표 (`PointSrid`) 값 객체.
//!
//! `WGS84` (`EPSG:4326`) `Point`만 V1에서 지원해요. `Srid`를 항상 동반하므로
//! `PostGIS` 쿼리에서 좌표계 누락이 컴파일 타임에 막혀요.
//!
//! `lat` ∈ `[-90, 90]`, `lng` ∈ `[-180, 180]` 범위 검증해요. `NaN`/`±∞` 거부.
//! 한국 영역으로 좁힌 검증은 aggregate level (예: `Listing.geom_point`) 책임이에요.

use crate::srid::Srid;
use geo_types::Point as GeoPoint;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 좌표계 명시 `Point` 값 객체.
///
/// 모든 필드 `pub` — `Geometry`는 가벼운 좌표 묶음이라 캡슐화 비용이 정당화 안 돼요.
/// 무효 상태는 `try_new_*`에서 거부되므로 직접 구성도 안전해요.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PointSrid {
    /// 경도 (longitude).
    pub lng: f64,
    /// 위도 (latitude).
    pub lat: f64,
    /// 좌표계.
    pub srid: Srid,
}

/// `PointSrid` 검증 에러.
#[derive(Debug, Error)]
pub enum GeometryError {
    /// 경도가 `[-180, 180]` 범위 밖.
    #[error("longitude out of [-180, 180]: {actual}")]
    LngOutOfRange {
        /// 입력 경도.
        actual: f64,
    },
    /// 위도가 `[-90, 90]` 범위 밖.
    #[error("latitude out of [-90, 90]: {actual}")]
    LatOutOfRange {
        /// 입력 위도.
        actual: f64,
    },
    /// 좌표가 `NaN` 또는 `±∞`.
    #[error("coordinate must be finite (got lng={lng}, lat={lat})")]
    NotFinite {
        /// 입력 경도.
        lng: f64,
        /// 입력 위도.
        lat: f64,
    },
}

impl PointSrid {
    /// `WGS84` `Point` 생성. `lng/lat` 범위 검증.
    ///
    /// # Errors
    ///
    /// `NaN`/`±∞` → `NotFinite`. `lng ∉ [-180, 180]` → `LngOutOfRange`.
    /// `lat ∉ [-90, 90]` → `LatOutOfRange`.
    pub fn try_new_wgs84(lng: f64, lat: f64) -> Result<Self, GeometryError> {
        if !lng.is_finite() || !lat.is_finite() {
            return Err(GeometryError::NotFinite { lng, lat });
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(GeometryError::LngOutOfRange { actual: lng });
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(GeometryError::LatOutOfRange { actual: lat });
        }
        Ok(Self {
            lng,
            lat,
            srid: Srid::Wgs84,
        })
    }

    /// `geo-types::Point` 변환 (`PostGIS` interop).
    ///
    /// `geo-types::Point::new(x, y)`의 `x`는 `lng`, `y`는 `lat`로 매핑해요.
    #[must_use]
    pub fn to_geo_point(self) -> GeoPoint<f64> {
        GeoPoint::new(self.lng, self.lat)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    // ── Valid construction ────────────────────────────────────────

    #[test]
    fn wgs84_seoul_city_hall() {
        // 서울시청: lng=126.9784, lat=37.5666 (대략)
        let p = PointSrid::try_new_wgs84(126.9784, 37.5666).expect("valid WGS84");
        assert!((p.lng - 126.9784).abs() < f64::EPSILON);
        assert!((p.lat - 37.5666).abs() < f64::EPSILON);
        assert_eq!(p.srid, Srid::Wgs84);
    }

    #[test]
    fn wgs84_origin_zero_zero() {
        let p = PointSrid::try_new_wgs84(0.0, 0.0).expect("origin valid");
        assert_eq!(p.lng, 0.0);
        assert_eq!(p.lat, 0.0);
    }

    #[test]
    fn wgs84_boundary_lng_180() {
        let p = PointSrid::try_new_wgs84(180.0, 0.0).expect("boundary 180 inclusive");
        assert!((p.lng - 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wgs84_boundary_lng_neg_180() {
        let p = PointSrid::try_new_wgs84(-180.0, 0.0).expect("boundary -180 inclusive");
        assert!((p.lng + 180.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wgs84_boundary_lat_90() {
        let p = PointSrid::try_new_wgs84(0.0, 90.0).expect("boundary 90 inclusive");
        assert!((p.lat - 90.0).abs() < f64::EPSILON);
    }

    #[test]
    fn wgs84_boundary_lat_neg_90() {
        let p = PointSrid::try_new_wgs84(0.0, -90.0).expect("boundary -90 inclusive");
        assert!((p.lat + 90.0).abs() < f64::EPSILON);
    }

    // ── Range rejection ─────────────────────────────────────────────

    #[test]
    fn rejects_lng_above_180() {
        let err = PointSrid::try_new_wgs84(180.5, 0.0).unwrap_err();
        assert!(matches!(err, GeometryError::LngOutOfRange { actual } if actual > 180.0));
    }

    #[test]
    fn rejects_lng_below_neg_180() {
        let err = PointSrid::try_new_wgs84(-181.0, 0.0).unwrap_err();
        assert!(matches!(err, GeometryError::LngOutOfRange { .. }));
    }

    #[test]
    fn rejects_lat_above_90() {
        let err = PointSrid::try_new_wgs84(0.0, 91.0).unwrap_err();
        assert!(matches!(err, GeometryError::LatOutOfRange { .. }));
    }

    #[test]
    fn rejects_lat_below_neg_90() {
        let err = PointSrid::try_new_wgs84(0.0, -91.0).unwrap_err();
        assert!(matches!(err, GeometryError::LatOutOfRange { .. }));
    }

    // ── Not finite rejection ────────────────────────────────────────

    #[test]
    fn rejects_lng_nan() {
        let err = PointSrid::try_new_wgs84(f64::NAN, 0.0).unwrap_err();
        assert!(matches!(err, GeometryError::NotFinite { .. }));
    }

    #[test]
    fn rejects_lat_nan() {
        let err = PointSrid::try_new_wgs84(0.0, f64::NAN).unwrap_err();
        assert!(matches!(err, GeometryError::NotFinite { .. }));
    }

    #[test]
    fn rejects_lng_infinity() {
        let err = PointSrid::try_new_wgs84(f64::INFINITY, 0.0).unwrap_err();
        assert!(matches!(err, GeometryError::NotFinite { .. }));
    }

    #[test]
    fn rejects_lng_neg_infinity() {
        let err = PointSrid::try_new_wgs84(f64::NEG_INFINITY, 0.0).unwrap_err();
        assert!(matches!(err, GeometryError::NotFinite { .. }));
    }

    // ── geo-types interop ──────────────────────────────────────────

    #[test]
    fn to_geo_point_maps_lng_to_x_lat_to_y() {
        let p = PointSrid::try_new_wgs84(126.9784, 37.5666).expect("valid");
        let geo = p.to_geo_point();
        assert!((geo.x() - 126.9784).abs() < f64::EPSILON);
        assert!((geo.y() - 37.5666).abs() < f64::EPSILON);
    }

    #[test]
    fn copy_semantics_preserves_srid() {
        let p = PointSrid::try_new_wgs84(0.0, 0.0).expect("ok");
        let q = p; // Copy
        assert_eq!(p.srid, q.srid);
        assert_eq!(p.lng, q.lng);
    }
}
