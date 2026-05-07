//! Gold manifest — 클라이언트가 R2 에서 *어떤 버전* 의 vector tile 을 fetch 할지 결정.
//!
//! ADR 0016 hot-swap + ADR 0021 (``PMTiles`` 분해 → flat tile) 패턴:
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

/// L10 Data Lineage — 단일 layer 빌드의 *모든* lineage. promote 단계에서 모인 후
/// `GoldManifest.artifacts[layer].lineage` 로 박제. provenance 추적 불가능 → 가능.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildLineage {
    /// `tippecanoe` 빌드에 사용된 git tag (`sp9-base-layer-config::TIPPECANOE_VERSION` reflection).
    pub tippecanoe_version: String,
    /// 빌드 머신의 git SHA (workflow 의 `${{ github.sha }}`). dev 빌드는 `unknown`.
    pub git_sha: String,
    /// 빌드 시작 시각 (UTC).
    pub built_at: DateTime<Utc>,
    /// 입력 Bronze archive 들 — 각 R2 key + sha256 + bytes. ETL 입력의 fingerprinting.
    pub bronze_inputs: Vec<BronzeInput>,
    /// `--source-srs` flag 값 (예: `EPSG:5186`).
    pub source_srs: String,
    /// `LayerKind::layer_name()` (`parcels` / `admin` / `complex`).
    pub layer_name: String,
    /// 빌드 환경 — `production` / `staging` / `dev` 등 (env-driven).
    pub build_environment: String,
}

/// Bronze archive 의 lineage entry — promote 단계의 검증 input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BronzeInput {
    /// R2 object key (예: `bronze/2026-05/parcel-dtmk-30563/LSMD_CONT_LDREG_충북_충주시.zip`).
    pub r2_key: String,
    /// 객체 size (bytes).
    pub bytes: u64,
    /// R2 `ETag` — single-part PUT 시 MD5, multipart 시 합성. 일관 fingerprint.
    /// `None` = R2 `list_objects` 응답에서 누락 (드물지만 가능).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

/// 단일 Gold 아티팩트 — 한 layer 의 빌드/검증 메타 + 프론트 동적 소비 hint.
///
/// **SSOT 정책**: 본 schema 가 *runtime SSOT*. ETL 의 [`crate::gold::tippecanoe::LayerKind`]
/// enum 이 build-time SSOT 이고, 본 manifest 가 그 reflection 을 클라이언트로 propagate.
/// 프론트는 본 메타를 fetch 후 `addSource`/`addLayer` 의 zoom/cache 등을 동적 결정 —
/// *hardcode 0*.
///
/// ``PMTiles`` 파일은 build artifact (`sha256` / `row_count` 검증 용도), R2 에는 분해된
/// flat `.pbf` 만 업로드 (ADR 0021).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldArtifact {
    /// flat tile prefix — `gold/<version>/<layer>` (URL template 에 `{layer}` 치환됨).
    pub key: String,
    /// vector tile 안의 `source-layer` 이름 (= `LayerKind::layer_name`).
    pub source_layer: String,
    /// ``PMTiles`` 파일 크기 (bytes) — build artifact, R2 미업로드.
    pub pmtiles_bytes: u64,
    /// ``PMTiles`` sha256 — 빌드 결정성 검증.
    pub pmtiles_sha256: String,
    /// 빌드 완료 시각 (UTC).
    pub built_at: DateTime<Utc>,
    /// feature 행 수 — `row_count_delta_pct < 5%` smoke 검증 기준.
    pub row_count: u64,
    /// ADR 0021 — flat tile (.pbf) 개수.
    pub flat_tile_count: u64,
    /// ADR 0021 — flat tile 합계 bytes.
    pub flat_tiles_total_bytes: u64,
    /// `PMTiles` 빌드 zoom 하한 (= `LayerKind::zoom_range().0`).
    /// 프론트 `addSource({ minzoom })` 가 본 값을 따름.
    pub tile_min_zoom: u8,
    /// `PMTiles` 빌드 zoom 상한 (= `LayerKind::zoom_range().1`).
    /// 프론트 `addSource({ maxzoom })` 가 본 값을 따름.
    pub tile_max_zoom: u8,
    /// 프론트 layer render 시작 zoom (= `LayerKind::render_min_zoom`).
    /// `addLayer({ minzoom })` 가 본 값을 따름. `tile_min_zoom` 보다 클 수 있음.
    pub render_min_zoom: u8,
    /// 프론트 layer render 종료 zoom (= `LayerKind::render_max_zoom`).
    /// `None` 시 mapbox-gl default 24.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render_max_zoom: Option<u8>,
    /// CDN `Cache-Control: max-age=<seconds>` — R2 PUT 메타 + manifest 박제.
    pub cache_max_age_seconds: u32,
    /// L10 lineage — 본 layer 의 build provenance (tippecanoe SHA + git SHA + bronze inputs).
    /// 미설정 (legacy 빌드 호환) 가능 → `Option`.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub lineage: Option<BuildLineage>,
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
    /// 직전 활성 버전 (rollback target hint). 첫 publish 시 `None`.
    /// 본 필드 + `gold/manifest.<previous_version>.json` 백업으로 즉시 rollback 가능.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub previous_version: Option<String>,
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
            previous_version: None,
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
        // source_layer 는 key 의 마지막 segment — `gold/v3/admin` → `admin`. 그 결과 JSON
        // 안에서 같은 layer 이름이 *map key* 와 *source_layer 값* 으로 동시 등장하지만,
        // 둘 다 본 layer 의 이름이라 알파벳 순 검증에 영향 없음.
        let source_layer = key.rsplit('/').next().unwrap_or(key).to_owned();
        GoldArtifact {
            key: key.into(),
            source_layer,
            pmtiles_bytes: 1_234,
            pmtiles_sha256: "abc".into(),
            built_at: Utc.with_ymd_and_hms(2026, 5, 6, 10, 0, 0).unwrap(),
            row_count: 1_400_000_000,
            flat_tile_count: 800_000,
            flat_tiles_total_bytes: 8_000_000_000,
            tile_min_zoom: 14,
            tile_max_zoom: 17,
            render_min_zoom: 16,
            render_max_zoom: None,
            cache_max_age_seconds: 31_536_000,
            lineage: None,
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
