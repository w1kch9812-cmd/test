//! `DataGoKrBuildingReader` — `building_domain::reader::BuildingReader` 구현체.
//!
//! `fetch_by_pnu` 만 구현 — `fetch_by_id` 는 FU 42 (`mgmBldrgstPk` 별도 endpoint).

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::sync::Arc;

use async_trait::async_trait;
use building_domain::entity::Building;
use building_domain::errors::ReaderError;
use building_domain::reader::BuildingReader;
use chrono::Utc;
use raw_capture_client::RawCapture;
use shared_kernel::geometry::PolygonSrid;
use shared_kernel::pnu::Pnu;
use tracing::{instrument, warn};
use vworld_client::VWorldClient;

use crate::building_register::client::BuildingRegisterClient;
use crate::building_register::parser::parse_building_title;
use crate::client::DataGoKrClient;
use crate::pnu_split;

/// V-World 연속지적도 레이어 — `LP_PA_CBND_BUBUN`.
///
/// data.go.kr 건축물대장 응답에 폴리곤 없음 → V-World 필지 폴리곤(연속지적도)을
/// `Building.geom` 으로 *합성* (approximation). 정확한 건물 footprint 는 FU 40
/// (R2 PMTiles 또는 V-World 건물 레이어) 에서 교체.
///
/// **레이어 선택 이유**: PNU `attrFilter` 가 작동하는 표준 레이어. 옛
/// `LT_C_UQ111` (용도지역) 은 `pnu` attribute 미보유 — `INVALID_RANGE` 에러.
/// → ADR 0015 (V-World ACL re-architecture) 참조.
const VWORLD_LAYER_PARCEL_BOUNDARY: &str = "LP_PA_CBND_BUBUN";

/// raw_capture 의 `source` 라벨 — `parcel_external_data.source` 컬럼 그대로.
const RAW_CAPTURE_SOURCE: &str = "data_go_kr_building";

/// `BuildingReader` 의 data.go.kr 구현체.
///
/// 합성 의존:
/// - `DataGoKrClient` — 건축물대장 표제부 raw JSON
/// - `VWorldClient` — 필지 폴리곤 (geom 합성)
/// - `RawCapture` — raw_response best-effort 보존
pub struct DataGoKrBuildingReader {
    data_go_kr: Arc<DataGoKrClient>,
    vworld: Arc<VWorldClient>,
    raw_capture: Arc<dyn RawCapture>,
}

impl DataGoKrBuildingReader {
    /// 새 [`DataGoKrBuildingReader`].
    #[must_use]
    pub const fn new(
        data_go_kr: Arc<DataGoKrClient>,
        vworld: Arc<VWorldClient>,
        raw_capture: Arc<dyn RawCapture>,
    ) -> Self {
        Self {
            data_go_kr,
            vworld,
            raw_capture,
        }
    }

    /// V-World 에서 필지 폴리곤만 추출. 다음 중 하나라도 실패면 `Err(Fetch)`.
    ///
    /// `Building.geom: PolygonSrid` 호환을 위해 `MultiPolygonSrid` 에서 첫
    /// polygon 만 추출. FU 40 에서 R2 PMTiles 건물 footprint 로 교체될 placeholder.
    /// feature 0 일 때 None 인데, 본 함수는 None 도 에러로 (geom 없으면
    /// `Building.geom` invariant 위반).
    async fn fetch_polygon(&self, pnu: &Pnu) -> Result<PolygonSrid, ReaderError> {
        let raw = self
            .vworld
            .fetch_feature_by_pnu(VWORLD_LAYER_PARCEL_BOUNDARY, pnu.as_str())
            .await
            .map_err(|e| ReaderError::Fetch(format!("vworld geom fetch: {e}")))?;

        let parcel_opt =
            vworld_client::layers::parcel_boundary::parse_parcel_boundary(&raw, Utc::now())
                .map_err(|e| ReaderError::Parse(format!("vworld geom parse: {e}")))?;
        match parcel_opt {
            Some(parcel) => {
                // MultiPolygon → 첫 polygon 발췌 (Building.geom 호환).
                // 좌표는 이미 MultiPolygonSrid::try_new_wgs84 에서 검증됨 — 재검증은
                // PolygonSrid 보장 일관성을 위한 정직한 비용.
                let first = parcel.geom.first_polygon().clone();
                PolygonSrid::try_new_wgs84(first).map_err(|e| {
                    ReaderError::Parse(format!("vworld first polygon invalid: {e}"))
                })
            }
            None => Err(ReaderError::Fetch(format!(
                "vworld returned no feature for pnu '{}' — cannot synthesize Building.geom",
                pnu.as_str()
            ))),
        }
    }
}

#[async_trait]
impl BuildingReader for DataGoKrBuildingReader {
    /// 1. data.go.kr `getBrTitleInfo` 호출 — raw JSON
    /// 2. raw_capture (best-effort)
    /// 3. V-World 필지 폴리곤 fetch — geom 합성
    /// 4. `parse_building_title` ACL → `Vec<Building>`
    ///
    /// 빈 응답 → `Ok(vec![])`. data.go.kr API error → `Err(Fetch)`.
    #[instrument(skip(self), fields(pnu = %pnu.as_str()))]
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Vec<Building>, ReaderError> {
        let parts = pnu_split::split(pnu);
        let br_client = BuildingRegisterClient::new(&self.data_go_kr);

        let raw = br_client
            .fetch_title_info(parts)
            .await
            .map_err(|e| ReaderError::Fetch(e.to_string()))?;

        let now = Utc::now();
        if let Err(capture_err) = self
            .raw_capture
            .capture(pnu.as_str(), RAW_CAPTURE_SOURCE, &raw, now)
            .await
        {
            warn!(
                pnu = %pnu.as_str(),
                error = %capture_err,
                "raw_capture failed — proceeding with parsed result"
            );
        }

        // 빠른 종료 — items 가 비어있으면 V-World 호출 불필요.
        // parse_building_title 는 polygon 을 받지만 검사하지 않고, items=빈 일
        // 때는 즉시 빈 vec 반환. 폴리곤 fetch 비용 절약 위해 미리 호출 회피.
        let items_present = raw
            .pointer("/response/body/items")
            .and_then(|v| v.get("item"))
            .filter(|v| !v.is_null())
            .is_some();

        if !items_present {
            // ApiError / Malformed 가 있을 수 있으니 parse 는 수행 (poly 더미).
            // 더미 poly 가 결과에 안 들어감 — items 빈 분기 안에서 즉시 vec 반환.
            // 단 ApiError / Malformed 는 그대로 전파.
            return parse_building_title(&raw, pnu, &dummy_polygon(), now)
                .map_err(|e| ReaderError::Parse(e.to_string()));
        }

        let polygon = self.fetch_polygon(pnu).await?;
        parse_building_title(&raw, pnu, &polygon, now)
            .map_err(|e| ReaderError::Parse(e.to_string()))
    }

    /// 미구현 — `mgmBldrgstPk` 별도 endpoint 필요. FU 42.
    #[instrument(skip(self))]
    async fn fetch_by_id(&self, _building_id: &str) -> Result<Option<Building>, ReaderError> {
        Err(ReaderError::Fetch(
            "fetch_by_id deferred to FU 42 (mgmBldrgstPk endpoint)".to_owned(),
        ))
    }
}

/// `parse_building_title` 가 빈 items 분기에서 polygon 을 사용하지 않지만 인자
/// 시그니처상 필요 — 1×1 단위 폴리곤. 결과 `Vec` 에 절대 들어가지 않음.
fn dummy_polygon() -> PolygonSrid {
    use geo_types::{Coord, LineString, Polygon as GeoPolygon};
    let exterior = LineString(vec![
        Coord { x: 126.0, y: 37.0 },
        Coord { x: 127.0, y: 37.0 },
        Coord { x: 127.0, y: 38.0 },
        Coord { x: 126.0, y: 38.0 },
        Coord { x: 126.0, y: 37.0 },
    ]);
    PolygonSrid::try_new_wgs84(GeoPolygon::new(exterior, vec![]))
        .unwrap_or_else(|_| unreachable!("constant polygon always valid WGS84"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::client::DataGoKrConfig;
    use raw_capture_client::NoOpRawCapture;
    use vworld_client::VWorldConfig;

    fn dummy_data_go_kr() -> Arc<DataGoKrClient> {
        Arc::new(DataGoKrClient::new(DataGoKrConfig {
            service_key: "k".to_owned(),
            base_url: "http://127.0.0.1:1".to_owned(),
        }))
    }

    fn dummy_vworld() -> Arc<VWorldClient> {
        Arc::new(VWorldClient::new(VWorldConfig {
            api_key: "k".to_owned(),
            domain: "localhost".to_owned(),
            base_url: "http://127.0.0.1:1".to_owned(),
        }))
    }

    #[tokio::test]
    async fn fetch_by_id_returns_deferred_error() {
        let reader = DataGoKrBuildingReader::new(
            dummy_data_go_kr(),
            dummy_vworld(),
            Arc::new(NoOpRawCapture::new()),
        );
        let err = reader.fetch_by_id("12345").await.unwrap_err();
        assert!(matches!(err, ReaderError::Fetch(s) if s.contains("FU 42")));
    }
}
