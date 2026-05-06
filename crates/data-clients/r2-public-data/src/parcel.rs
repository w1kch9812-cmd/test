//! `R2ParcelReader` — `parcel_domain::reader::ParcelReader` 구현체.
//!
//! 1차 = honest failure stub. PMTiles directory + tile_at + MVT decode 가
//! [FU 60] (ETL 빌더 + production fixture) 와 함께 구현되기 전까지는
//! `fetch_markers_in_bbox` 가 `Err(Fetch("FU 60 pending"))` 반환. composition
//! root 가 V-World 합성 또는 다른 fallback 사용.
//!
//! 본 stub 의 가치 — *architecture* 가 wire-up. R2Client + raw_capture +
//! `BuildingFootprintSource` (T7 후속) 통합 path 가 명확. PMTiles 디코드만
//! 추가하면 production-ready.
//!
//! [FU 60]: docs/superpowers/specs/2026-05-06-sub-project-4-iii-e-r2-pmtiles-design.md § 7

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::sync::Arc;

use async_trait::async_trait;
use parcel_domain::entity::Parcel;
use parcel_domain::errors::ReaderError;
use parcel_domain::reader::{ParcelMarker, ParcelReader};
use raw_capture_client::RawCapture;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::pnu::Pnu;
use tracing::{instrument, warn};

use crate::client::R2Client;
use crate::pmtiles::tile_at_pending_message;

/// PMTiles 객체 key — `static/parcels.pmtiles` (FU 60 ETL 빌더 출력 위치).
pub const PMTILES_PARCELS_KEY: &str = "static/parcels.pmtiles";

/// `ParcelReader` 의 R2 PMTiles 구현체.
///
/// 1차 = stub. PMTiles tile_at + MVT decode 가 FU 60 와 함께 통합되면
/// `fetch_markers_in_bbox` 가 실데이터 반환.
pub struct R2ParcelReader {
    client: Arc<R2Client>,
    raw_capture: Arc<dyn RawCapture>,
}

impl R2ParcelReader {
    /// 새 [`R2ParcelReader`].
    #[must_use]
    pub const fn new(client: Arc<R2Client>, raw_capture: Arc<dyn RawCapture>) -> Self {
        Self {
            client,
            raw_capture,
        }
    }
}

#[async_trait]
impl ParcelReader for R2ParcelReader {
    /// PMTiles `static/parcels.pmtiles` 헤더 검증 + bbox → tile coords →
    /// tile_at lookup → MVT decode → ParcelMarker. 1차 = honest failure
    /// (FU 60).
    #[instrument(skip(self, _pnu))]
    async fn fetch_by_pnu(&self, _pnu: &Pnu) -> Result<Option<Parcel>, ReaderError> {
        // SP4-iii-e 1차 미포함 — V-World 가 fetch_by_pnu 의 SSOT (parcel + zoning).
        // R2 PMTiles 의 fetch_by_pnu 는 spatial index 를 별도 JSON 으로 빌드해야
        // 하므로 별도 SP 분리 (FU 61).
        Err(ReaderError::Fetch(
            "R2 fetch_by_pnu deferred -- use V-World; FU 61 R2 spatial index 후 활성화".to_owned(),
        ))
    }

    /// bbox → tile coords → PMTiles fetch → MVT decode → ParcelMarker[].
    /// 1차 = `tile_at` 미구현 → FU 60 honest failure.
    ///
    /// architecture wire-up 자체는 검증됨 (R2 GET → header parse → ...).
    #[instrument(skip(self, _bbox))]
    async fn fetch_markers_in_bbox(
        &self,
        _bbox: &BoundingBox,
    ) -> Result<Vec<ParcelMarker>, ReaderError> {
        // 1차 — PMTiles header 까지만 검증. tile_at 디코드는 FU 60.
        let raw = self
            .client
            .get_object_bytes(PMTILES_PARCELS_KEY)
            .await
            .map_err(|e| ReaderError::Fetch(format!("R2 PMTiles fetch: {e}")))?;

        let header = crate::pmtiles::parse_header(&raw)
            .map_err(|e| ReaderError::Parse(format!("PMTiles header: {e}")))?;

        // best-effort raw_capture (binary 자체는 R2 영구 보존, meta 만 기록).
        let now = chrono::Utc::now();
        let meta = serde_json::json!({
            "source": "r2_public_data",
            "key": PMTILES_PARCELS_KEY,
            "version": header.version,
            "size_bytes": raw.len(),
        });
        if let Err(capture_err) = self
            .raw_capture
            .capture("r2:parcels:pmtiles", "r2_public_data", &meta, now)
            .await
        {
            warn!(
                error = %capture_err,
                "raw_capture failed for r2 parcels — proceeding"
            );
        }

        // tile_at 미구현 — FU 60.
        Err(ReaderError::Fetch(format!(
            "fetch_markers_in_bbox: {}",
            tile_at_pending_message()
        )))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::client::R2Config;
    use raw_capture_client::NoOpRawCapture;

    fn dummy_client() -> Arc<R2Client> {
        Arc::new(R2Client::new(R2Config {
            public_url_base: "http://127.0.0.1:1".to_owned(),
        }))
    }

    #[tokio::test]
    async fn fetch_by_pnu_returns_deferred_error() {
        let reader = R2ParcelReader::new(dummy_client(), Arc::new(NoOpRawCapture::new()));
        let pnu = Pnu::try_new("1111010100100010000").unwrap();
        let err = reader.fetch_by_pnu(&pnu).await.unwrap_err();
        assert!(matches!(err, ReaderError::Fetch(s) if s.contains("FU 61")));
    }
}
