//! Gold manifest — 클라이언트가 R2 에서 *어떤 버전* 의 `PMTiles` 를 fetch 할지 결정.
//!
//! ADR 0016 hot-swap 패턴:
//! 1. 새 빌드 → `gold/<version>/parcels.pmtiles` 등 업로드
//! 2. smoke 테스트 (강남 PNU + row count Δ < 5%)
//! 3. **검증 통과 후에만** `gold/manifest.json` 의 `current_version` 갱신
//! 4. 클라이언트는 manifest 조회 → 그 버전 fetch (CDN cache `no-cache` 권장)
//!
//! 실패 시 manifest 변경 없음 → 클라가 이전 버전 그대로 사용 (degrade gracefully).
//!
//! T3b.1 = struct + JSON serde 까지. activate 동작 (실제 PUT) 은 T3b.2 에서.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 단일 Gold 아티팩트 (`PMTiles` 또는 JSON 인덱스) 의 메타.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldArtifact {
    /// R2 객체 key (예: `gold/v3/parcels.pmtiles`).
    pub key: String,
    /// 파일 크기 (bytes).
    pub bytes: u64,
    /// SHA-256 hex digest — 빌드 결정성 검증.
    pub sha256: String,
    /// 빌드 완료 시각 (UTC).
    pub built_at: DateTime<Utc>,
    /// 행 수 (`PMTiles` 의 경우 feature 개수, JSON 인덱스의 경우 항목 수).
    /// 변동률 검증 (`row_count_delta_pct < 5%`) 의 기준.
    pub row_count: u64,
}

/// Gold manifest — 매월 빌드 결과 + 활성 버전 포인터.
///
/// `gold/manifest.json` (CDN `Cache-Control: no-cache, max-age=0` 권장) 으로 업로드.
/// 클라이언트는 manifest fetch → `current_version` 으로 prefix 결정 → `PMTiles` fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldManifest {
    /// 활성 버전 (예: `v3`). 빌드/검증 통과 후 hot-swap.
    pub current_version: String,
    /// 활성 버전의 빌드 시각.
    pub current_activated_at: DateTime<Utc>,
    /// 활성 버전의 아티팩트들 (`parcels` / `admin` / `complex` 등 → 메타).
    /// `BTreeMap` — 안정적 직렬화 순서 (sha256 비교 용이).
    pub artifacts: BTreeMap<String, GoldArtifact>,
    /// 매니페스트 자체의 갱신 시각 (활성 시점과 동일하지만 별개 필드로 보존).
    pub manifest_updated_at: DateTime<Utc>,
}

impl GoldManifest {
    /// 새 manifest. activate 직전에 호출.
    /// T3b.1 = 데이터 모델 + serde 만. activate 호출은 T3b.2.
    #[allow(dead_code)]
    #[must_use]
    pub fn new(version: String, artifacts: BTreeMap<String, GoldArtifact>) -> Self {
        let now = Utc::now();
        Self {
            current_version: version,
            current_activated_at: now,
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
            bytes: 1_234,
            sha256: "abc".into(),
            built_at: Utc.with_ymd_and_hms(2026, 5, 6, 10, 0, 0).unwrap(),
            row_count: 1_400_000_000,
        }
    }

    #[test]
    fn manifest_roundtrips() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert(
            "parcels".into(),
            fixture_artifact("gold/v3/parcels.pmtiles"),
        );
        artifacts.insert("admin".into(), fixture_artifact("gold/v3/admin.pmtiles"));

        let m = GoldManifest::new("v3".into(), artifacts);
        let json = m.to_pretty_json().unwrap();
        let back: GoldManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.current_version, "v3");
        assert_eq!(back.artifacts.len(), 2);
        assert!(back.artifacts.contains_key("parcels"));
    }

    #[test]
    fn artifacts_serialized_in_btreemap_order() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert(
            "parcels".into(),
            fixture_artifact("gold/v3/parcels.pmtiles"),
        );
        artifacts.insert("admin".into(), fixture_artifact("gold/v3/admin.pmtiles"));
        artifacts.insert(
            "complex".into(),
            fixture_artifact("gold/v3/complex.pmtiles"),
        );

        let m = GoldManifest::new("v3".into(), artifacts);
        let json = m.to_pretty_json().unwrap();

        let admin_pos = json.find("\"admin\"").expect("admin key");
        let complex_pos = json.find("\"complex\"").expect("complex key");
        let parcels_pos = json.find("\"parcels\"").expect("parcels key");
        // BTreeMap 알파벳 순: admin < complex < parcels.
        assert!(admin_pos < complex_pos);
        assert!(complex_pos < parcels_pos);
    }
}
