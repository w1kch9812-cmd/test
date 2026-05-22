//! 공간 좌표 (`PointSrid`) 값 객체.
//!
//! `WGS84` (`EPSG:4326`) `Point`만 V1에서 지원해요. `Srid`를 항상 동반하므로
//! `PostGIS` 쿼리에서 좌표계 누락이 컴파일 타임에 막혀요.
//!
//! `lat` ∈ `[-90, 90]`, `lng` ∈ `[-180, 180]` 범위 검증해요. `NaN`/`±∞` 거부.
//! Parcel-attached listing marker placement is resolved from PNU anchors outside this value object.

use crate::srid::Srid;
use geo_types::{MultiPolygon as GeoMultiPolygon, Point as GeoPoint, Polygon as GeoPolygon};
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
    /// `Polygon` 외곽 링 점 < 4 (`GeoJSON`은 첫=마지막 포함 ≥4 점 요구).
    #[error("polygon exterior ring must have ≥4 points (got {actual})")]
    ExteriorRingTooShort {
        /// 실제 점 수.
        actual: usize,
    },
    /// `MultiPolygon`이 비어 있음 (≥1 polygon 필수).
    #[error("multipolygon must contain at least one polygon")]
    EmptyMultiPolygon,
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

/// `WGS84` 강제 + 좌표 범위 검증된 `Polygon`.
///
/// Spec § 8.4 `Parcel.geom` 매핑. `PointSrid`의 `Polygon` 버전.
///
/// Exterior ring + 0개 이상 holes (interior rings) 지원. `geo-types::Polygon` wrapper.
/// Self-intersection 검증 *안* 함 — 비용이 큼, 외부 `R2` 데이터 신뢰.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolygonSrid {
    /// `Polygon` 데이터 (`geo-types::Polygon`).
    pub polygon: GeoPolygon<f64>,
    /// 좌표계.
    pub srid: Srid,
}

impl PolygonSrid {
    /// `WGS84` `Polygon` 생성. 모든 좌표 finite + 범위 검증 + exterior ring ≥4 점.
    ///
    /// # Errors
    ///
    /// 좌표 `NaN`/`±∞` → `NotFinite`. `lng` 범위 외 → `LngOutOfRange`.
    /// `lat` 범위 외 → `LatOutOfRange`. 외곽 링 점 < 4 → `ExteriorRingTooShort`.
    pub fn try_new_wgs84(polygon: GeoPolygon<f64>) -> Result<Self, GeometryError> {
        // Exterior ring 점 수 검증 (`GeoJSON`: 첫=마지막 포함 ≥4 점).
        let exterior_len = polygon.exterior().0.len();
        if exterior_len < 4 {
            return Err(GeometryError::ExteriorRingTooShort {
                actual: exterior_len,
            });
        }

        // 모든 좌표 (exterior + holes) finite + `WGS84` 범위 검증.
        for coord in &polygon.exterior().0 {
            Self::validate_coord(coord.x, coord.y)?;
        }
        for hole in polygon.interiors() {
            for coord in &hole.0 {
                Self::validate_coord(coord.x, coord.y)?;
            }
        }

        Ok(Self {
            polygon,
            srid: Srid::Wgs84,
        })
    }

    /// 내부 헬퍼 — 단일 좌표 검증.
    fn validate_coord(lng: f64, lat: f64) -> Result<(), GeometryError> {
        if !lng.is_finite() || !lat.is_finite() {
            return Err(GeometryError::NotFinite { lng, lat });
        }
        if !(-180.0..=180.0).contains(&lng) {
            return Err(GeometryError::LngOutOfRange { actual: lng });
        }
        if !(-90.0..=90.0).contains(&lat) {
            return Err(GeometryError::LatOutOfRange { actual: lat });
        }
        Ok(())
    }

    /// `geo-types::Polygon` 참조 반환 (`PostGIS` interop).
    #[must_use]
    pub const fn as_geo_polygon(&self) -> &GeoPolygon<f64> {
        &self.polygon
    }
}

/// `WGS84` 강제 + 좌표 범위 검증된 `MultiPolygon`.
///
/// 한국 필지(`Parcel.geom`) 매핑 — V-World `LP_PA_CBND_BUBUN` 응답이
/// `MultiPolygon`. 단일 `Polygon`만 가진 필지도 V-World는 `MultiPolygon`으로
/// 감싸서 반환하므로, 도메인은 `MultiPolygon`을 SSOT로 둠.
///
/// 구성 polygon ≥1 + 각 polygon의 exterior ring ≥4점 + 모든 좌표 finite/범위 검증.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiPolygonSrid {
    /// `MultiPolygon` 데이터 (`geo-types::MultiPolygon`).
    pub multi_polygon: GeoMultiPolygon<f64>,
    /// 좌표계.
    pub srid: Srid,
}

impl MultiPolygonSrid {
    /// `WGS84` `MultiPolygon` 생성. 모든 polygon에 `PolygonSrid::try_new_wgs84`와
    /// 동일한 검증 적용.
    ///
    /// # Errors
    ///
    /// Polygon 0개 → `EmptyMultiPolygon`. 그 외 좌표 검증 실패는
    /// `PolygonSrid::try_new_wgs84`와 동일.
    pub fn try_new_wgs84(multi: GeoMultiPolygon<f64>) -> Result<Self, GeometryError> {
        if multi.0.is_empty() {
            return Err(GeometryError::EmptyMultiPolygon);
        }
        for polygon in &multi.0 {
            // 기존 PolygonSrid 검증 재사용 — clone은 유효성 검사 후 버림.
            // 비용보다 검증 일관성이 우선 (V-World 응답은 polygon 수가 적음, 보통 1~3).
            PolygonSrid::try_new_wgs84(polygon.clone())?;
        }
        Ok(Self {
            multi_polygon: multi,
            srid: Srid::Wgs84,
        })
    }

    /// 첫 번째 polygon (단순 시각화 등 단일 polygon만 다루는 호출자용).
    #[must_use]
    pub fn first_polygon(&self) -> &GeoPolygon<f64> {
        &self.multi_polygon.0[0]
    }

    /// `geo-types::MultiPolygon` 참조 반환 (`PostGIS` interop).
    #[must_use]
    pub const fn as_geo_multi_polygon(&self) -> &GeoMultiPolygon<f64> {
        &self.multi_polygon
    }

    /// 구성 polygon 개수.
    #[must_use]
    pub const fn polygon_count(&self) -> usize {
        self.multi_polygon.0.len()
    }
}

#[cfg(test)]
mod tests;
