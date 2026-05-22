use serde::{Deserialize, Serialize};

use super::super::manifest::BuildLineage;

/// 한 layer 의 build artifact 메타 — R2 staging 에 박제 후 promote 가 모음.
///
/// `Serialize` + `Deserialize` 양쪽 — write/read 가 *동일 schema* 통과 (P0 typed 검증):
/// staging spec 의 누락 필드 / 오타 / 변조는 [`serde_json::from_slice`] 단계에서 거부.
/// 더 이상 `serde_json::Value` + `unwrap_or_default()` path 0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactSpec {
    /// PMTiles + flat tiles 의 R2 prefix (예: `gold/v3/parcels`).
    pub key_prefix: String,
    /// PMTiles 파일 size (bytes).
    pub pmtiles_bytes: u64,
    /// PMTiles SHA-256.
    pub pmtiles_sha256: String,
    /// 빌드 결과의 feature 수. `None` = tippecanoe metadata 미지원/파싱 실패.
    pub row_count: Option<u64>,
    /// flat tile 개수.
    pub flat_tile_count: u64,
    /// flat tile 합계 bytes.
    pub flat_tiles_total_bytes: u64,
    /// L10 lineage (본 layer 의 provenance).
    pub lineage: BuildLineage,
}
