//! V-World `LP_PA_CBND_BUBUN` 어댑터 — `ParcelReader` 위에 좁은 lookup port 를
//! 얹음.
//!
//! 호출 흐름:
//! 1. [`ParcelReader::fetch_by_pnu`] (V-World HTTP + Circuit Breaker)
//! 2. [`Parcel`] → [`ParcelInfo`] narrow 매핑 (geometry/면적/주소 버림)
//! 3. `ReaderError` → [`LookupError`] 매핑

use std::sync::Arc;

use async_trait::async_trait;
use parcel_domain::errors::ReaderError;
use parcel_domain::reader::ParcelReader;
use shared_kernel::pnu::Pnu;
use tracing::instrument;

use crate::info::ParcelInfo;
use crate::lookup::{LookupError, ParcelInfoLookup};

/// `ParcelReader` 를 좁은 [`ParcelInfoLookup`] 으로 어댑팅.
///
/// 의존성은 trait object — V-World 외 다른 reader (Bronze SHP R-tree 등) 가
/// 도입돼도 본 어댑터 그대로 재사용.
pub struct VWorldParcelInfoLookup {
    reader: Arc<dyn ParcelReader>,
}

impl VWorldParcelInfoLookup {
    /// 새 어댑터.
    #[must_use]
    pub const fn new(reader: Arc<dyn ParcelReader>) -> Self {
        Self { reader }
    }
}

#[async_trait]
impl ParcelInfoLookup for VWorldParcelInfoLookup {
    #[instrument(skip(self), fields(pnu = %pnu.as_str()))]
    async fn lookup_by_pnu(&self, pnu: &Pnu) -> Result<Option<ParcelInfo>, LookupError> {
        match self.reader.fetch_by_pnu(pnu).await {
            Ok(Some(parcel)) => Ok(Some(ParcelInfo {
                admin: parcel.admin,
                land_use_type: parcel.land_use_type,
                zoning: parcel.zoning,
                official_land_price_per_m2: parcel.official_land_price_per_m2,
                gosi_year_month: parcel.gosi_year_month,
            })),
            Ok(None) | Err(ReaderError::NotFound) => Ok(None),
            Err(ReaderError::Fetch(s)) => Err(LookupError::Backend(s)),
            Err(ReaderError::Parse(s)) => Err(LookupError::Parse(s)),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::match_wildcard_for_single_variants
    )]

    use super::*;
    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};
    use parcel_domain::entity::Parcel;
    use parcel_domain::reader::{ParcelMarker, ParcelReader};
    use shared_kernel::address::JibunAddress;
    use shared_kernel::admin_division::{AdminDivision, EupmyeondongCode, SidoCode, SigunguCode};
    use shared_kernel::bounding_box::BoundingBox;
    use shared_kernel::geometry::MultiPolygonSrid;
    use shared_kernel::land_use_type::LandUseType;

    /// In-memory `ParcelReader` — single PNU fixture.
    struct FakeReader {
        result: Result<Option<Parcel>, ReaderError>,
    }

    #[async_trait]
    impl ParcelReader for FakeReader {
        async fn fetch_by_pnu(&self, _pnu: &Pnu) -> Result<Option<Parcel>, ReaderError> {
            match &self.result {
                Ok(opt) => Ok(opt.clone()),
                Err(ReaderError::Fetch(s)) => Err(ReaderError::Fetch(s.clone())),
                Err(ReaderError::Parse(s)) => Err(ReaderError::Parse(s.clone())),
                Err(ReaderError::NotFound) => Err(ReaderError::NotFound),
            }
        }

        async fn fetch_markers_in_bbox(
            &self,
            _bbox: &BoundingBox,
        ) -> Result<Vec<ParcelMarker>, ReaderError> {
            Err(ReaderError::Fetch("not used in lookup tests".into()))
        }
    }

    fn sample_parcel(pnu_str: &str) -> Parcel {
        Parcel {
            pnu: Pnu::try_new(pnu_str).expect("valid pnu"),
            admin: AdminDivision::try_new(
                SidoCode::try_new("11").unwrap(),
                SigunguCode::try_new("11680").unwrap(),
                EupmyeondongCode::try_new("11680101").unwrap(),
            )
            .unwrap(),
            road_address: None,
            jibun_address: JibunAddress::try_new("서울특별시 강남구 역삼동 737").unwrap(),
            land_use_type: LandUseType::Building,
            area: None,
            official_land_price_per_m2: None,
            gosi_year_month: None,
            zoning: None,
            geom: MultiPolygonSrid::try_new_wgs84(geo_types::MultiPolygon::new(vec![
                geo_types::Polygon::new(
                    geo_types::LineString::from(vec![
                        (127.0, 37.5),
                        (127.001, 37.5),
                        (127.001, 37.501),
                        (127.0, 37.501),
                        (127.0, 37.5),
                    ]),
                    vec![],
                ),
            ]))
            .expect("valid multipolygon"),
            fetched_at: Utc.with_ymd_and_hms(2026, 5, 6, 0, 0, 0).unwrap(),
        }
    }

    #[tokio::test]
    async fn lookup_some_returns_narrow_info() {
        let reader = Arc::new(FakeReader {
            result: Ok(Some(sample_parcel("1168010100107370000"))),
        });
        let lookup = VWorldParcelInfoLookup::new(reader);
        let pnu = Pnu::try_new("1168010100107370000").unwrap();

        let info = lookup.lookup_by_pnu(&pnu).await.unwrap().unwrap();
        assert_eq!(info.admin.eupmyeondong.as_str(), "11680101");
        assert_eq!(info.land_use_type, LandUseType::Building);
    }

    #[tokio::test]
    async fn lookup_none_propagates() {
        let reader = Arc::new(FakeReader { result: Ok(None) });
        let lookup = VWorldParcelInfoLookup::new(reader);
        let pnu = Pnu::try_new("9999999999999999999").unwrap();

        assert!(lookup.lookup_by_pnu(&pnu).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn lookup_fetch_error_maps_to_backend() {
        let reader = Arc::new(FakeReader {
            result: Err(ReaderError::Fetch("circuit open".into())),
        });
        let lookup = VWorldParcelInfoLookup::new(reader);
        let pnu = Pnu::try_new("1168010100107370000").unwrap();

        match lookup.lookup_by_pnu(&pnu).await.unwrap_err() {
            LookupError::Backend(s) => assert!(s.contains("circuit open")),
            other => panic!("expected Backend, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn lookup_parse_error_maps_to_parse() {
        let reader = Arc::new(FakeReader {
            result: Err(ReaderError::Parse("bad jibun".into())),
        });
        let lookup = VWorldParcelInfoLookup::new(reader);
        let pnu = Pnu::try_new("1168010100107370000").unwrap();

        match lookup.lookup_by_pnu(&pnu).await.unwrap_err() {
            LookupError::Parse(s) => assert!(s.contains("bad jibun")),
            other => panic!("expected Parse, got {other:?}"),
        }
    }
}
