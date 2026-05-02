//! 좌표계 식별자 (`SRID` / `EPSG`).
//!
//! 한국 부동산 도메인은 3개 좌표계를 사용해요:
//! - `Wgs84` (4326): 글로벌 표준, 네이버/구글 지도 호환
//! - `UtmK` (5179): 한국 측량 표준 (국토지리정보원)
//! - `KoreaCentralTm` (5186): 중부원점 `TM` (행정 측량)
//!
//! `AGENTS.md` §1 헌법 — `SRID` 미지정 공간 쿼리 금지. 본 enum이 컴파일 타임에 강제해요.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 좌표계 식별자 (`EPSG` 코드 기반).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(i32)]
pub enum Srid {
    /// `WGS84` — 글로벌 표준, 네이버/구글 호환 (`EPSG:4326`).
    Wgs84 = 4326,
    /// `UTM-K` — 한국 측량 표준 (`EPSG:5179`, 국토지리정보원).
    UtmK = 5179,
    /// 중부원점 `TM` — 한국 행정 측량 (`EPSG:5186`).
    KoreaCentralTm = 5186,
}

/// `Srid` 변환 에러.
#[derive(Debug, Error)]
pub enum SridError {
    /// 지원하지 않는 `EPSG` 코드.
    #[error("unsupported EPSG code: {code} (supported: 4326, 5179, 5186)")]
    Unsupported {
        /// 입력 코드.
        code: i32,
    },
}

impl Srid {
    /// `EPSG` 코드로부터 `Srid` 생성.
    ///
    /// # Errors
    ///
    /// 4326/5179/5186 외의 코드는 [`SridError::Unsupported`].
    pub const fn from_epsg(code: i32) -> Result<Self, SridError> {
        match code {
            4326 => Ok(Self::Wgs84),
            5179 => Ok(Self::UtmK),
            5186 => Ok(Self::KoreaCentralTm),
            other => Err(SridError::Unsupported { code: other }),
        }
    }

    /// `EPSG` 코드 (`i32`) 반환.
    #[must_use]
    pub const fn epsg(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn wgs84_epsg_is_4326() {
        assert_eq!(Srid::Wgs84.epsg(), 4326);
    }

    #[test]
    fn utm_k_epsg_is_5179() {
        assert_eq!(Srid::UtmK.epsg(), 5179);
    }

    #[test]
    fn korea_central_tm_epsg_is_5186() {
        assert_eq!(Srid::KoreaCentralTm.epsg(), 5186);
    }

    #[test]
    fn from_epsg_4326_yields_wgs84() {
        assert_eq!(Srid::from_epsg(4326).expect("supported"), Srid::Wgs84);
    }

    #[test]
    fn from_epsg_5179_yields_utm_k() {
        assert_eq!(Srid::from_epsg(5179).expect("supported"), Srid::UtmK);
    }

    #[test]
    fn from_epsg_5186_yields_korea_central_tm() {
        assert_eq!(
            Srid::from_epsg(5186).expect("supported"),
            Srid::KoreaCentralTm
        );
    }

    #[test]
    fn from_epsg_unsupported_returns_err() {
        let err = Srid::from_epsg(3857).unwrap_err();
        assert!(matches!(err, SridError::Unsupported { code: 3857 }));
    }

    #[test]
    fn round_trip_wgs84() {
        let s = Srid::Wgs84;
        let recovered = Srid::from_epsg(s.epsg()).expect("round trip");
        assert_eq!(s, recovered);
    }

    #[test]
    fn round_trip_all_three() {
        for s in [Srid::Wgs84, Srid::UtmK, Srid::KoreaCentralTm] {
            let recovered = Srid::from_epsg(s.epsg()).expect("round trip");
            assert_eq!(s, recovered);
        }
    }

    #[test]
    fn copy_semantics() {
        let s = Srid::Wgs84;
        let t = s; // Copy
        assert_eq!(s, t); // s still usable after move-as-copy
    }
}
