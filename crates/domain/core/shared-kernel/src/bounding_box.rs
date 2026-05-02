//! `BoundingBox` — `WGS84` 지도 영역.
//!
//! Listing/Parcel/Building/IndustrialComplex Reader/Repository에서 공통 사용.
//! `min_lng < max_lng`, `min_lat < max_lat`, 모든 좌표 finite + `WGS84` 범위 검증.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::geometry::PointSrid;

/// `WGS84` 지도 영역 (`min` 모서리 + `max` 모서리).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BoundingBox {
    /// 최소 경도.
    pub min_lng: f64,
    /// 최소 위도.
    pub min_lat: f64,
    /// 최대 경도.
    pub max_lng: f64,
    /// 최대 위도.
    pub max_lat: f64,
}

/// `BoundingBox` 검증 에러.
#[derive(Debug, Error, PartialEq)]
pub enum BoundingBoxError {
    /// 좌표가 `NaN`/`±∞`.
    #[error(
        "coordinate must be finite (got min_lng={min_lng}, min_lat={min_lat}, max_lng={max_lng}, max_lat={max_lat})"
    )]
    NotFinite {
        /// `min_lng`.
        min_lng: f64,
        /// `min_lat`.
        min_lat: f64,
        /// `max_lng`.
        max_lng: f64,
        /// `max_lat`.
        max_lat: f64,
    },
    /// `lng` 범위가 `[-180, 180]` 밖.
    #[error("longitude out of [-180, 180]: min_lng={min_lng}, max_lng={max_lng}")]
    LngOutOfRange {
        /// `min_lng`.
        min_lng: f64,
        /// `max_lng`.
        max_lng: f64,
    },
    /// `lat` 범위가 `[-90, 90]` 밖.
    #[error("latitude out of [-90, 90]: min_lat={min_lat}, max_lat={max_lat}")]
    LatOutOfRange {
        /// `min_lat`.
        min_lat: f64,
        /// `max_lat`.
        max_lat: f64,
    },
    /// `min` ≥ `max`.
    #[error(
        "min must be < max: min_lng={min_lng} max_lng={max_lng} min_lat={min_lat} max_lat={max_lat}"
    )]
    InvalidOrder {
        /// `min_lng`.
        min_lng: f64,
        /// `max_lng`.
        max_lng: f64,
        /// `min_lat`.
        min_lat: f64,
        /// `max_lat`.
        max_lat: f64,
    },
}

impl BoundingBox {
    /// `WGS84` `BoundingBox` 생성. 모든 좌표 finite + 범위 + `min < max` 검증.
    ///
    /// # Errors
    ///
    /// `NaN`/`±∞` → `NotFinite`. 범위 외 → `LngOutOfRange`/`LatOutOfRange`.
    /// `min >= max` → `InvalidOrder`.
    pub fn try_new_wgs84(
        min_lng: f64,
        min_lat: f64,
        max_lng: f64,
        max_lat: f64,
    ) -> Result<Self, BoundingBoxError> {
        if !min_lng.is_finite()
            || !min_lat.is_finite()
            || !max_lng.is_finite()
            || !max_lat.is_finite()
        {
            return Err(BoundingBoxError::NotFinite {
                min_lng,
                min_lat,
                max_lng,
                max_lat,
            });
        }
        if !(-180.0..=180.0).contains(&min_lng) || !(-180.0..=180.0).contains(&max_lng) {
            return Err(BoundingBoxError::LngOutOfRange { min_lng, max_lng });
        }
        if !(-90.0..=90.0).contains(&min_lat) || !(-90.0..=90.0).contains(&max_lat) {
            return Err(BoundingBoxError::LatOutOfRange { min_lat, max_lat });
        }
        if min_lng >= max_lng || min_lat >= max_lat {
            return Err(BoundingBoxError::InvalidOrder {
                min_lng,
                max_lng,
                min_lat,
                max_lat,
            });
        }
        Ok(Self {
            min_lng,
            min_lat,
            max_lng,
            max_lat,
        })
    }

    /// `Point`가 영역 안에 있는지 (경계 포함).
    #[must_use]
    pub fn contains(&self, point: &PointSrid) -> bool {
        point.lng >= self.min_lng
            && point.lng <= self.max_lng
            && point.lat >= self.min_lat
            && point.lat <= self.max_lat
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn try_new_wgs84_seoul_metro_area() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        assert!((bbox.min_lng - 126.7).abs() < f64::EPSILON);
        assert!((bbox.max_lat - 37.7).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_min_lng_ge_max_lng() {
        let err = BoundingBox::try_new_wgs84(127.2, 37.4, 126.7, 37.7).unwrap_err();
        assert!(matches!(err, BoundingBoxError::InvalidOrder { .. }));
    }

    #[test]
    fn rejects_min_lat_ge_max_lat() {
        let err = BoundingBox::try_new_wgs84(126.7, 37.7, 127.2, 37.4).unwrap_err();
        assert!(matches!(err, BoundingBoxError::InvalidOrder { .. }));
    }

    #[test]
    fn rejects_min_eq_max_lng() {
        let err = BoundingBox::try_new_wgs84(127.0, 37.4, 127.0, 37.7).unwrap_err();
        assert!(matches!(err, BoundingBoxError::InvalidOrder { .. }));
    }

    #[test]
    fn rejects_lng_out_of_range() {
        let err = BoundingBox::try_new_wgs84(-181.0, 37.4, 127.2, 37.7).unwrap_err();
        assert!(matches!(err, BoundingBoxError::LngOutOfRange { .. }));
    }

    #[test]
    fn rejects_lat_out_of_range() {
        let err = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 91.0).unwrap_err();
        assert!(matches!(err, BoundingBoxError::LatOutOfRange { .. }));
    }

    #[test]
    fn rejects_nan() {
        let err = BoundingBox::try_new_wgs84(f64::NAN, 37.4, 127.2, 37.7).unwrap_err();
        assert!(matches!(err, BoundingBoxError::NotFinite { .. }));
    }

    #[test]
    fn rejects_infinity() {
        let err = BoundingBox::try_new_wgs84(126.7, 37.4, f64::INFINITY, 37.7).unwrap_err();
        assert!(matches!(err, BoundingBoxError::NotFinite { .. }));
    }

    #[test]
    fn contains_point_inside() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        let point = PointSrid::try_new_wgs84(127.0, 37.5).expect("valid");
        assert!(bbox.contains(&point));
    }

    #[test]
    fn contains_point_on_boundary() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        let point = PointSrid::try_new_wgs84(126.7, 37.4).expect("valid");
        assert!(bbox.contains(&point));
    }

    #[test]
    fn contains_point_outside() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        let point = PointSrid::try_new_wgs84(128.0, 37.5).expect("valid");
        assert!(!bbox.contains(&point));
    }

    #[test]
    fn copy_semantics() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        let copied = bbox; // Copy
        assert_eq!(bbox, copied);
    }

    #[test]
    fn serde_roundtrip() {
        let bbox = BoundingBox::try_new_wgs84(126.7, 37.4, 127.2, 37.7).expect("valid");
        let json = serde_json::to_string(&bbox).expect("serialize");
        let back: BoundingBox = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bbox, back);
    }
}
