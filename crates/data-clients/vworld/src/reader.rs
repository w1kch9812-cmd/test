//! `VWorldParcelReader` — `parcel_domain::reader::ParcelReader` 구현체.
//!
//! `fetch_by_pnu` — V-World `LP_PA_CBND_BUBUN` (연속지적도) 단일 호출.
//! 면적/용도지역은 본 레이어 미제공 → `Parcel.area = None`, `Parcel.zoning = None`.
//! 향후 `LT_C_UQ111` spatial intersect 보강은 별도 메서드(미정).
//!
//! `fetch_markers_in_bbox` — SP4-iii (R2 PMTiles) 후속.

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use parcel_domain::entity::Parcel;
use parcel_domain::errors::ReaderError;
use parcel_domain::reader::{ParcelMarker, ParcelReader};
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::pnu::Pnu;
use tracing::{instrument, warn};

use raw_capture_client::RawCapture;

use crate::client::VWorldClient;
use crate::layers::parcel_boundary::parse_parcel_boundary;

/// V-World 연속지적도 레이어 — `LP_PA_CBND_BUBUN`.
///
/// PNU 기반 단일 필지 조회의 SSOT. `attrFilter=pnu:=:` 이 작동하는 유일한
/// 표준 레이어. 자세한 응답 schema 는
/// [`crate::layers::parcel_boundary`] 참조.
const LAYER_PARCEL_BOUNDARY: &str = "LP_PA_CBND_BUBUN";

/// `ParcelReader` 의 V-World 구현체.
pub struct VWorldParcelReader {
    client: Arc<VWorldClient>,
    raw_capture: Arc<dyn RawCapture>,
}

impl VWorldParcelReader {
    /// 새 [`VWorldParcelReader`].
    #[must_use]
    pub fn new(client: Arc<VWorldClient>, raw_capture: Arc<dyn RawCapture>) -> Self {
        Self {
            client,
            raw_capture,
        }
    }
}

#[async_trait]
impl ParcelReader for VWorldParcelReader {
    /// V-World `LP_PA_CBND_BUBUN` WFS GetFeature → 도메인 [`Parcel`].
    ///
    /// 1. `client.fetch_feature_by_pnu(LP_PA_CBND_BUBUN, pnu)` (Circuit Breaker)
    /// 2. `raw_capture.capture(...)` (best-effort, raw_response 보존 — SSOT 보호)
    /// 3. `layers::parcel_boundary::parse_parcel_boundary(raw, now)` (ACL)
    ///
    /// `Ok(Some(Parcel))` 또는 `Ok(None)` (status `NOT_FOUND` 또는 빈 features).
    #[instrument(skip(self), fields(pnu = %pnu.as_str()))]
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError> {
        let raw = self
            .client
            .fetch_feature_by_pnu(LAYER_PARCEL_BOUNDARY, pnu.as_str())
            .await
            .map_err(|e| ReaderError::Fetch(e.to_string()))?;

        let now = Utc::now();
        if let Err(capture_err) = self
            .raw_capture
            .capture(pnu.as_str(), "vworld", &raw, now)
            .await
        {
            warn!(
                pnu = %pnu.as_str(),
                error = %capture_err,
                "raw_capture failed — proceeding with parsed result"
            );
        }

        parse_parcel_boundary(&raw, now).map_err(|e| ReaderError::Parse(e.to_string()))
    }

    /// 미구현 — `Err(Fetch("bbox markers deferred to SP4-iii"))` (honest failure).
    ///
    /// SP4-iii (R2 PMTiles) 또는 V-World BBOX WFS 후속.
    #[instrument(skip(self, _bbox))]
    async fn fetch_markers_in_bbox(
        &self,
        _bbox: &BoundingBox,
    ) -> Result<Vec<ParcelMarker>, ReaderError> {
        Err(ReaderError::Fetch(
            "bbox markers deferred to SP4-iii".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use shared_kernel::bounding_box::BoundingBox;

    use super::*;
    use raw_capture_client::NoOpRawCapture;

    fn dummy_client() -> Arc<VWorldClient> {
        let config = crate::client::VWorldConfig {
            api_key: "test".to_owned(),
            domain: "localhost".to_owned(),
            base_url: "http://127.0.0.1:1".to_owned(),
        };
        Arc::new(VWorldClient::new(config))
    }

    #[tokio::test]
    async fn fetch_markers_in_bbox_returns_deferred_error() {
        let reader = VWorldParcelReader::new(dummy_client(), Arc::new(NoOpRawCapture::new()));
        let bbox = BoundingBox::try_new_wgs84(126.9, 37.4, 127.1, 37.6).unwrap();
        let err = reader.fetch_markers_in_bbox(&bbox).await.unwrap_err();
        assert!(matches!(err, ReaderError::Fetch(s) if s.contains("SP4-iii")));
    }
}
