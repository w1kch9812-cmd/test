//! SP9 Base Layer **SSOT** — 한 곳에 박제, 모든 사용처가 본 crate import.
//!
//! ## SSOT 원칙 (AGENTS.md § 8)
//!
//! 본 crate 가 SP9 base layer infra 의 *유일한* truth source. 사용처:
//! - **Rust** — `services/etl-base-layer/` 가 import 후 const 직접 사용.
//! - **GitHub Actions YAML** — `cargo run --bin sp9-config-print -- env` 호출 후
//!   `>> $GITHUB_ENV` 또는 `eval "$(...)"` 로 모든 const 를 shell env 화.
//! - **Dockerfile.etl** — `--build-arg TIPPECANOE_VERSION=$(cargo run --bin sp9-config-print -- key tippecanoe_version)`
//!   으로 워크플로 측에서 inject.
//!
//! ## 본 crate 가 박제하는 것
//!
//! 1. **외부 도구 버전** — `tippecanoe`, `GDAL`, Rust toolchain.
//! 2. **데이터 소스 식별자** — V-World dtmk dsId, source/target SRS.
//! 3. **R2 키 레이아웃** — `dtmk_bronze_prefix(batch)` 헬퍼.
//! 4. **layer 메타** — `Layer` enum + `Self::ALL` (workflow matrix 도 본 reflection 사용).
//! 5. **Verify landmarks** — 강남 PNU + lat/lon (테스트/검증의 known-good 좌표).
//!
//! ## 본 crate 가 박제하지 *않는* 것 (의도적)
//!
//! - R2 자격 — env-driven (secrets, not constants).
//! - layer 별 zoom range — `etl-base-layer` 의 `LayerKind` 에 박제 (build-time SSOT,
//!   본 crate 는 *infra* SSOT, layer geometry 정책은 etl crate 의 책임).
//! - CI runner 사양 — workflow YAML 직접 (SSOT crate 가 GH Actions 사양까지 강제하면
//!   coupling 과다).

#![forbid(unsafe_code)]
#![allow(clippy::doc_markdown)]

pub mod types;
pub use types::{R2PublicBase, Srs, TypeError, Version};

use serde::{Deserialize, Serialize};

/// `tippecanoe` git tag (sanity 표시용). 진짜 결정성은 [`TIPPECANOE_GIT_SHA`].
/// upgrade = 본 const + `TIPPECANOE_GIT_SHA` 둘 다 commit.
pub const TIPPECANOE_VERSION: &str = "2.79.0";

/// `tippecanoe` 의 *commit SHA* (tag 보다 강한 immutable identifier).
///
/// tag 는 GitHub 측에서 force-push 가능 → SHA pin 이 진짜 SSS-grade reproducibility.
/// build 시 `git fetch --depth 1 origin <SHA>` + `git checkout <SHA>` 로 정확 commit fetch.
/// 갱신: `curl -s https://api.github.com/repos/felt/tippecanoe/git/refs/tags/<ver>`.
pub const TIPPECANOE_GIT_SHA: &str = "68ab8dcc229f95b8b25877697d5e8d66783af503";

/// `GDAL` apt 의 *exact* version (Ubuntu 22.04 jammy security 표준).
/// wildcard (`3.4.*`) 는 빌드마다 다른 patch 가져옴 → 결정성 깨짐.
/// 갱신: `apt-cache madison gdal-bin` on jammy.
pub const GDAL_VERSION_PIN: &str = "3.4.1-1build4";

/// Rust toolchain version — `rust-toolchain.toml` 의 channel 과 동일.
/// workflow / Dockerfile 이 이 const 를 echo 해 base image tag (`rust:<v>-slim-bookworm`) 결정.
pub const RUST_TOOLCHAIN_VERSION: &str = "1.88";

/// V-World 연속지적도 dataset ID. `parcel-dtmk-<dsId>` 형 prefix segment.
pub const DTMK_DS_ID: u32 = 30563;

/// V-World SHP source SRS — dtmk SHP 의 .prj 가 박제하는 좌표계.
/// `ogr2ogr` 의 `-s_srs` flag 에 사용. `.prj` 를 신뢰한다면 생략 가능하지만, 일부
/// 공공데이터 SHP 는 .prj 누락 → 안전을 위해 명시.
pub const SOURCE_SRS_VWORLD: &str = "EPSG:5186";

/// Web Mercator target SRS — mapbox-gl / Naver Maps 표준.
pub const TARGET_SRS_WEB: &str = "EPSG:4326";

/// dtmk 다운로드 동시성 default (V-World rate limit + 디스크 throughput sweet spot).
pub const DTMK_DOWNLOAD_CONCURRENCY: usize = 8;

/// PMTiles 빌드 결과의 최소 byte 크기 (전국 빌드 sanity).
/// 100MB 미만 = silent build fail 의심 → verify 단계에서 차단.
pub const NATIONWIDE_PMTILES_MIN_BYTES: u64 = 104_857_600;

/// SP9 base layer 의 정식 layer 식별자. `etl-base-layer::LayerKind` 의 SSOT.
///
/// `LayerKind` 가 zoom range 등 build-time geometry 정책을 owner — 본 enum 은 *infra*
/// 측의 동일 식별자 (workflow matrix 동적 생성 + manifest schema 검증 용).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// 필지 (parcels) — z14-17.
    Parcels,
    /// 행정구역 (admin) — z6-12.
    Admin,
    /// 산업단지 (complex) — z0-16 (사용자 요구: 모든 zoom).
    Complex,
}

impl Layer {
    /// 모든 variants — workflow matrix 자동 generate. **SSOT** — `etl-base-layer::LayerKind`
    /// 가 본 배열을 reflection (`From<Layer> for LayerKind` 가 exhaustive match → 새 variant
    /// 추가 시 컴파일러가 차단). `LayerKind::all_vec()` 가 본 배열을 base 로 동적 생성.
    pub const ALL: &'static [Self] = &[Self::Parcels, Self::Admin, Self::Complex];

    /// lowercase 이름 (`"parcels"`/`"admin"`/`"complex"`). PMTiles 안 source-layer + R2 prefix.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Parcels => "parcels",
            Self::Admin => "admin",
            Self::Complex => "complex",
        }
    }

    /// 본 layer 가 *현재 ETL build-active* 인지. Round 4 #2 (P0) — admin/complex 가
    /// 별도 source 미준비 (V-World 행정구역 dataset / 공공데이터포털 산업단지 SHP).
    /// **SSS-DEBT**: ADR 0027 박제 후 source 가 결정되면 본 함수의 분기를 `true` 로
    /// 변경. 그 전에는 workflow matrix 가 본 함수를 통과한 *active* layer 만 빌드 →
    /// admin/complex 가 parcels prefix 임시 재사용하던 trick 차단.
    #[must_use]
    pub const fn is_active_in_etl(self) -> bool {
        match self {
            Self::Parcels => true,
            // 별도 source 미준비 — ADR 0027.
            Self::Admin | Self::Complex => false,
        }
    }
}

/// dtmk Bronze R2 prefix 빌드 — `bronze/<batch>/parcel-dtmk-30563/`.
///
/// `batch` 는 `YYYY-MM` 형식 (예: `2026-05`). 본 함수가 *유일한* prefix 빌더 —
/// Python (`scraper-py/dtmk_vworld.py`) 도 본 crate 의 `sp9-config-print` binary
/// 출력으로 부터 같은 string 을 얻어 일관성 보장.
#[must_use]
pub fn dtmk_bronze_prefix(batch: &str) -> String {
    format!("bronze/{batch}/parcel-dtmk-{DTMK_DS_ID}/")
}

/// Verify 의 known landmark — known-good PNU + 좌표 + 의미.
///
/// 빌드 후 spot-check: 본 PNU 가 maxzoom tile (lat/lon → tile 좌표) 안에 *반드시*
/// 등장해야 함. 누락 = build silent fail.
///
/// `&'static str` 사용 → const 표현 가능. JSON 출력만 필요 (workflow 가 읽음) —
/// `Deserialize` 미구현으로 borrowed lifetime 충돌 회피.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct VerifyLandmark {
    /// PNU 19자리.
    pub pnu: &'static str,
    /// 위도 (WGS84).
    pub lat: f64,
    /// 경도 (WGS84).
    pub lon: f64,
    /// 사람 가독 라벨 (로그/에러 메시지용).
    pub label: &'static str,
}

/// Verify 의 known landmarks. parcels layer 의 invariant — 모든 landmark 가 빌드 결과에 존재.
///
/// 추가 방법: 새 `(pnu, lat, lon, label)` 튜플 push. workflow / verify 의 다른 변경 X
/// — verify 모듈이 본 배열을 iterate.
pub const VERIFY_LANDMARKS: &[VerifyLandmark] = &[VerifyLandmark {
    pnu: "1168010100107370000",
    lat: 37.51,
    lon: 127.04,
    label: "강남구 대치동",
}];

/// `print` binary 의 JSON 출력 schema — workflow 가 jq 로 read-only 소비.
#[derive(Debug, Serialize)]
pub struct ConfigSnapshot {
    /// `TIPPECANOE_VERSION`.
    pub tippecanoe_version: String,
    /// `TIPPECANOE_GIT_SHA` — 실 결정성의 정수.
    pub tippecanoe_git_sha: String,
    /// `GDAL_VERSION_PIN`.
    pub gdal_version_pin: String,
    /// `RUST_TOOLCHAIN_VERSION`.
    pub rust_toolchain_version: String,
    /// `DTMK_DS_ID`.
    pub dtmk_ds_id: u32,
    /// `SOURCE_SRS_VWORLD`.
    pub source_srs_vworld: String,
    /// `TARGET_SRS_WEB`.
    pub target_srs_web: String,
    /// `DTMK_DOWNLOAD_CONCURRENCY`.
    pub dtmk_download_concurrency: usize,
    /// `NATIONWIDE_PMTILES_MIN_BYTES`.
    pub nationwide_pmtiles_min_bytes: u64,
    /// 모든 layer 의 lowercase 이름 (registry / Rust LayerKind reflection 용).
    pub layers: Vec<String>,
    /// **현재 ETL build-active** layer 만 (Round 4 #2). workflow matrix 가 본 출력만
    /// 소비 — `is_active_in_etl()` 가 false 인 layer 는 build skip (silent partial 차단).
    pub active_layers: Vec<String>,
    /// known landmarks (verify 사용).
    pub verify_landmarks: Vec<VerifyLandmark>,
}

impl ConfigSnapshot {
    /// 모든 const 를 한 snapshot 에 박제 — `print` binary 가 본 함수 호출.
    #[must_use]
    pub fn current() -> Self {
        Self {
            tippecanoe_version: TIPPECANOE_VERSION.to_owned(),
            tippecanoe_git_sha: TIPPECANOE_GIT_SHA.to_owned(),
            gdal_version_pin: GDAL_VERSION_PIN.to_owned(),
            rust_toolchain_version: RUST_TOOLCHAIN_VERSION.to_owned(),
            dtmk_ds_id: DTMK_DS_ID,
            source_srs_vworld: SOURCE_SRS_VWORLD.to_owned(),
            target_srs_web: TARGET_SRS_WEB.to_owned(),
            dtmk_download_concurrency: DTMK_DOWNLOAD_CONCURRENCY,
            nationwide_pmtiles_min_bytes: NATIONWIDE_PMTILES_MIN_BYTES,
            layers: Layer::ALL.iter().map(|l| l.name().to_owned()).collect(),
            active_layers: Layer::ALL
                .iter()
                .filter(|l| l.is_active_in_etl())
                .map(|l| l.name().to_owned())
                .collect(),
            verify_landmarks: VERIFY_LANDMARKS.to_vec(),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn dtmk_prefix_format() {
        assert_eq!(
            dtmk_bronze_prefix("2026-05"),
            "bronze/2026-05/parcel-dtmk-30563/"
        );
    }

    #[test]
    fn layer_names_match_workflow_matrix() {
        // workflow matrix 의 [parcels, admin, complex] 와 *반드시* 일치.
        let names: Vec<&str> = Layer::ALL.iter().map(|l| l.name()).collect();
        assert_eq!(names, vec!["parcels", "admin", "complex"]);
    }

    /// Round 4 #2 (ADR 0027) — admin/complex 가 source 미준비 라 ETL build-active 0.
    /// `is_active_in_etl()` 의 분기를 변경할 때 본 test 도 반드시 함께 갱신 — workflow
    /// matrix 와 SSOT contract.
    #[test]
    fn active_layers_excludes_unready_sources() {
        let active: Vec<&str> = Layer::ALL
            .iter()
            .filter(|l| l.is_active_in_etl())
            .map(|l| l.name())
            .collect();
        assert_eq!(
            active,
            vec!["parcels"],
            "ADR 0027 — admin/complex 는 source 결정될 때까지 ETL matrix 제외"
        );
        assert!(Layer::Parcels.is_active_in_etl());
        assert!(!Layer::Admin.is_active_in_etl());
        assert!(!Layer::Complex.is_active_in_etl());
    }

    #[test]
    fn snapshot_active_layers_subset_of_layers() {
        let snap = ConfigSnapshot::current();
        for active in &snap.active_layers {
            assert!(
                snap.layers.contains(active),
                "active_layers must be subset of layers: {active} not in {:?}",
                snap.layers
            );
        }
    }

    #[test]
    #[allow(clippy::const_is_empty)] // const compile-time check 그대로 의도.
    fn verify_landmarks_nonempty_and_valid() {
        assert!(
            !VERIFY_LANDMARKS.is_empty(),
            "must have at least one landmark for verify"
        );
        for lm in VERIFY_LANDMARKS {
            assert_eq!(lm.pnu.len(), 19, "PNU must be 19 digits: {}", lm.label);
            assert!(
                lm.lat.is_finite() && lm.lat.abs() < 90.0,
                "lat sanity: {}",
                lm.label
            );
            assert!(
                lm.lon.is_finite() && lm.lon.abs() < 180.0,
                "lon sanity: {}",
                lm.label
            );
        }
    }

    #[test]
    fn snapshot_serializes_with_all_keys() {
        let snap = ConfigSnapshot::current();
        let json = serde_json::to_value(&snap).unwrap();
        for key in &[
            "tippecanoe_version",
            "tippecanoe_git_sha",
            "gdal_version_pin",
            "rust_toolchain_version",
            "dtmk_ds_id",
            "source_srs_vworld",
            "target_srs_web",
            "dtmk_download_concurrency",
            "nationwide_pmtiles_min_bytes",
            "layers",
            "verify_landmarks",
        ] {
            assert!(json.get(key).is_some(), "missing snapshot key: {key}");
        }
    }

    #[test]
    fn tippecanoe_git_sha_is_40_char_hex() {
        // SSS-grade 검증: SHA 가 잘못 박혀있으면 build 가 silent 다른 commit fetch 가능.
        assert_eq!(TIPPECANOE_GIT_SHA.len(), 40, "git SHA must be 40-char hex");
        assert!(
            TIPPECANOE_GIT_SHA
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "git SHA must be lowercase hex"
        );
    }

    #[test]
    fn gdal_version_is_exact_not_wildcard() {
        // `3.4.*` 같은 wildcard 는 빌드마다 다른 patch — 결정성 깨짐.
        assert!(
            !GDAL_VERSION_PIN.contains('*'),
            "GDAL pin must be exact version (not wildcard): {GDAL_VERSION_PIN}"
        );
        assert!(
            GDAL_VERSION_PIN.contains('-'),
            "GDAL pin should include Debian build suffix (e.g. -1build4)"
        );
    }
}
