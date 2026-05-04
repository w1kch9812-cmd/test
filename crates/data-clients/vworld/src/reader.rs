//! `VWorldParcelReader` — `parcel_domain::reader::ParcelReader` 구현체.
//!
//! `fetch_by_pnu` 만 구현 — `fetch_markers_in_bbox` 는 SP4-iii (PMTiles 또는
//! WFS BBOX 후속).

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
use crate::parser;

/// V-World 용도지역 레이어 — `LT_C_UQ111` (도시지역 용도지역).
///
/// `docs/data-sources/v-world.md` § 핵심 레이어 참조.
const LAYER_USE_ZONE: &str = "LT_C_UQ111";

/// `ParcelReader` 의 V-World 구현체.
///
/// `Arc<VWorldClient>` + `Arc<dyn RawCapture>` — 멀티 task 공유 가능.
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
    /// V-World WFS GetFeature → 도메인 [`Parcel`] 변환.
    ///
    /// 1. `client.fetch_feature_by_pnu(LT_C_UQ111, pnu)` — circuit breaker 통과
    /// 2. `raw_capture.capture(...)` (best-effort, 실패 시 warn)
    /// 3. `parser::parse_parcel(raw, now)` — ACL 변환
    /// 4. `Ok(Some(parcel))` 또는 `Ok(None)` (PNU 미존재)
    #[instrument(skip(self), fields(pnu = %pnu.as_str()))]
    async fn fetch_by_pnu(&self, pnu: &Pnu) -> Result<Option<Parcel>, ReaderError> {
        let raw = self
            .client
            .fetch_feature_by_pnu(LAYER_USE_ZONE, pnu.as_str())
            .await
            .map_err(|e| ReaderError::Fetch(e.to_string()))?;

        let now = Utc::now();
        // raw_response 보존 — best-effort. 실패 시 warn 후 진행 (fetch 결과는 우선).
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

        parser::parse_parcel(&raw, now).map_err(|e| ReaderError::Parse(e.to_string()))
    }

    /// 미구현 — `Err(Fetch("bbox markers deferred to SP4-iii"))` 반환 (honest failure).
    ///
    /// SP4-iii 에서 V-World BBOX WFS 또는 R2 PMTiles 로 구현 예정.
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
            // 잘못된 base_url 로 실제 호출 시 즉시 실패하게.
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
