//! Gold manifest — 클라이언트가 R2 에서 *어떤 버전* 의 vector tile 을 fetch 할지 결정.
//!
//! ADR 0016 hot-swap + ADR 0021 (`PMTiles` 분해 → flat tile) 패턴:
//! 1. 새 빌드 → tippecanoe 로 `<layer>.pmtiles` (build artifact, R2 미업로드)
//! 2. tile-join 으로 `<layer>/{z}/{x}/{y}.pbf` 분해
//! 3. flat tile 들 R2 batch upload (`gold/<version>/<layer>/{z}/{x}/{y}.pbf`)
//! 4. smoke 테스트 (강남 PNU + row count Δ < 5%)
//! 5. **검증 통과 후에만** `gold/manifest.json` 의 `current_version` 갱신
//! 6. 클라이언트는 manifest 조회 → `tiles_url_template` 에 `{layer}` 치환 → mapbox-gl
//!    `addSource({type:"vector", tiles:[URL_TEMPLATE]})` (ADR 0021)
//!
//! 실패 시 manifest 변경 없음 → 클라가 이전 버전 그대로 사용 (degrade gracefully).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 단일 Gold 아티팩트 — 한 layer 의 빌드/검증 메타.
///
/// `PMTiles` 파일은 build artifact (`sha256` / `row_count` 검증 용도), R2 에는 분해된
/// flat `.pbf` 만 업로드. 본 ADR 0021 결정.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldArtifact {
    /// flat tile prefix — `gold/<version>/<layer>` (URL template 에 `{layer}` 치환됨).
    pub key: String,
    /// `PMTiles` 파일 크기 (bytes) — build artifact, R2 미업로드.
    pub pmtiles_bytes: u64,
    /// `PMTiles` sha256 — 빌드 결정성 검증.
    pub pmtiles_sha256: String,
    /// 빌드 완료 시각 (UTC).
    pub built_at: DateTime<Utc>,
    /// feature 행 수 — `row_count_delta_pct < 5%` smoke 검증 기준.
    pub row_count: u64,
    /// ADR 0021 — flat tile (.pbf) 개수.
    pub flat_tile_count: u64,
    /// ADR 0021 — flat tile 합계 bytes.
    pub flat_tiles_total_bytes: u64,
}

/// Gold manifest — 매월 빌드 결과 + 활성 버전 포인터.
///
/// `gold/manifest.json` (CDN `Cache-Control: no-cache, max-age=0` 권장) 으로 업로드.
/// 클라이언트 흐름:
/// 1. `fetch(<r2_public>/gold/manifest.json)` → `current_version` + `tiles_url_template`
/// 2. mapbox-gl `addSource({ type:"vector", tiles:[tiles_url_template] })`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldManifest {
    /// 활성 버전 (예: `v3`). 빌드/검증 통과 후 hot-swap.
    pub current_version: String,
    /// 활성 버전의 빌드 시각.
    pub current_activated_at: DateTime<Utc>,
    /// ADR 0021 — flat tile URL template. `{layer}` 는 source-layer 이름 (parcels/admin/complex).
    /// 예: `https://r2.gongzzang.dev/gold/v3/{layer}/{z}/{x}/{y}.pbf`
    pub tiles_url_template: String,
    /// 활성 버전의 아티팩트들 (`parcels` / `admin` / `complex` 등 → 메타).
    /// `BTreeMap` — 안정적 직렬화 순서 (sha256 비교 용이).
    pub artifacts: BTreeMap<String, GoldArtifact>,
    /// 매니페스트 자체의 갱신 시각.
    pub manifest_updated_at: DateTime<Utc>,
}

impl GoldManifest {
    /// 새 manifest. activate 직전에 호출.
    #[allow(dead_code)]
    #[must_use]
    pub fn new(
        version: String,
        tiles_url_template: String,
        artifacts: BTreeMap<String, GoldArtifact>,
    ) -> Self {
        let now = Utc::now();
        Self {
            current_version: version,
            current_activated_at: now,
            tiles_url_template,
            artifacts,
            manifest_updated_at: now,
        }
    }

    /// JSON pretty 직렬화.
    ///
    /// # Errors
    ///
    /// `serde_json` 직렬화 실패 시 [`serde_json::Error`].
    #[allow(dead_code)]
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::TimeZone;

    fn fixture_artifact(key: &str) -> GoldArtifact {
        GoldArtifact {
            key: key.into(),
            pmtiles_bytes: 1_234,
            pmtiles_sha256: "abc".into(),
            built_at: Utc.with_ymd_and_hms(2026, 5, 6, 10, 0, 0).unwrap(),
            row_count: 1_400_000_000,
            flat_tile_count: 800_000,
            flat_tiles_total_bytes: 8_000_000_000,
        }
    }

    #[test]
    fn manifest_roundtrips() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert("parcels".into(), fixture_artifact("gold/v3/parcels"));
        artifacts.insert("admin".into(), fixture_artifact("gold/v3/admin"));

        let m = GoldManifest::new(
            "v3".into(),
            "https://r2.gongzzang.dev/gold/v3/{layer}/{z}/{x}/{y}.pbf".into(),
            artifacts,
        );
        let json = m.to_pretty_json().unwrap();
        let back: GoldManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.current_version, "v3");
        assert!(back.tiles_url_template.contains("{layer}/{z}/{x}/{y}.pbf"));
        assert_eq!(back.artifacts.len(), 2);
        assert!(back.artifacts.contains_key("parcels"));
        assert_eq!(back.artifacts["parcels"].flat_tile_count, 800_000);
    }

    #[test]
    fn artifacts_serialized_in_btreemap_order() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert("parcels".into(), fixture_artifact("gold/v3/parcels"));
        artifacts.insert("admin".into(), fixture_artifact("gold/v3/admin"));
        artifacts.insert("complex".into(), fixture_artifact("gold/v3/complex"));

        let m = GoldManifest::new(
            "v3".into(),
            "https://r2.gongzzang.dev/gold/v3/{layer}/{z}/{x}/{y}.pbf".into(),
            artifacts,
        );
        let json = m.to_pretty_json().unwrap();

        let admin_pos = json.find("\"admin\"").expect("admin key");
        let complex_pos = json.find("\"complex\"").expect("complex key");
        let parcels_pos = json.find("\"parcels\"").expect("parcels key");
        assert!(admin_pos < complex_pos);
        assert!(complex_pos < parcels_pos);
    }
}
