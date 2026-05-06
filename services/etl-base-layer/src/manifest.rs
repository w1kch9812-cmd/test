//! Bronze manifest JSON — 외부 데이터 archive 의 audit 보조.
//!
//! 매 배치마다 R2 `<YYYY-MM>/manifest.json` 으로 저장됨. SP9 T3a 는 manifest *생성*
//! 만 다룸 — R2 업로드는 T3b (별도 세션).
//!
//! Gold manifest (`gongzzang-static/manifest.json`) 는 별도 모듈 (T3b).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 한 소스 (parcel / admin / industrial-complex) 의 captured artifact 메타.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    /// 다운로드 URL (외부 SSOT 추적).
    pub url: String,
    /// 로컬/R2 파일명.
    pub filename: String,
    /// 파일 크기 (bytes).
    pub bytes: u64,
    /// SHA-256 hex digest — 외부 데이터 변경 감지.
    pub sha256: String,
    /// 다운로드 완료 시각 (UTC).
    pub downloaded_at: DateTime<Utc>,
}

/// Bronze 배치 manifest — 한 `<YYYY-MM>` archive 의 헤더.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BronzeManifest {
    /// 배치 라벨 (예: `2026-05`).
    pub batch_label: String,
    /// 배치 시작 시각 (UTC).
    pub batch_started_at: DateTime<Utc>,
    /// 소스 ID → entry. `BTreeMap` 으로 안정적 직렬화 순서 보장 (sha256 비교 용이).
    pub sources: BTreeMap<String, SourceEntry>,
}

impl BronzeManifest {
    /// 빈 manifest (`sources` 비어있음).
    #[must_use]
    pub fn new(batch_label: String) -> Self {
        Self {
            batch_label,
            batch_started_at: Utc::now(),
            sources: BTreeMap::new(),
        }
    }

    /// 소스 entry 추가/덮어쓰기.
    pub fn insert(&mut self, id: String, entry: SourceEntry) {
        self.sources.insert(id, entry);
    }

    /// JSON pretty 직렬화 — 로컬 파일/R2 업로드 양쪽에 사용.
    ///
    /// # Errors
    ///
    /// `serde_json` 직렬화 실패 시 [`serde_json::Error`].
    pub fn to_pretty_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use chrono::TimeZone;

    use super::*;

    #[test]
    fn empty_manifest_roundtrips() {
        let m = BronzeManifest::new("2026-05".into());
        let json = m.to_pretty_json().unwrap();
        let back: BronzeManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back.batch_label, "2026-05");
        assert!(back.sources.is_empty());
    }

    #[test]
    fn insert_and_serialize_stable_order() {
        let at = Utc.with_ymd_and_hms(2026, 5, 6, 10, 0, 0).unwrap();
        let mut m = BronzeManifest::new("2026-05".into());
        m.insert(
            "parcel".into(),
            SourceEntry {
                url: "https://www.data.go.kr/...".into(),
                filename: "parcel.shp.zip".into(),
                bytes: 524_288,
                sha256: "abc123".into(),
                downloaded_at: at,
            },
        );
        m.insert(
            "admin".into(),
            SourceEntry {
                url: "https://www.data.go.kr/admin".into(),
                filename: "admin.shp.zip".into(),
                bytes: 32_768,
                sha256: "def456".into(),
                downloaded_at: at,
            },
        );
        let json = m.to_pretty_json().unwrap();
        // BTreeMap 순서 — `admin` 이 `parcel` 보다 먼저.
        let admin_pos = json.find("\"admin\"").expect("admin present");
        let parcel_pos = json.find("\"parcel\"").expect("parcel present");
        assert!(
            admin_pos < parcel_pos,
            "BTreeMap should serialize admin before parcel"
        );
    }
}
