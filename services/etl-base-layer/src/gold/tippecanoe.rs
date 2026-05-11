//! tippecanoe spawn — `GeoJSON` 파일들 → 단일 `PMTiles` 빌드.
//!
//! 본 모듈은 `tippecanoe` binary 를 실행. binary 자체는 dev WSL 에 빌드됨
//! (`/usr/local/bin/tippecanoe`) 또는 CI Ubuntu 에서 felt/tippecanoe make.
//!
//! Layer 별 zoom 스펙 (ADR 0016 §):
//! - **parcels** Z14-17 — 매물 클릭 단위, 가까이서만 visible.
//! - **admin**   Z6-12  — 행정구역 outline, 멀리서 visible.
//! - **complex** Z0-16  — 산업단지 boundary, **모든 zoom 에서 visible** (사용자 SSS 요구).
//!   → low-zoom 에 tippecanoe `--coalesce-smallest-as-needed` 가 sub-pixel polygon merge.
//!
//! flag 셋은 [gongzzang-design-lab build-pmtiles.ts] 검증된 값과 동일:
//! `-P --no-feature-limit --no-tile-size-limit --drop-smallest-as-needed`
//! `--simplification=10 --extend-zooms-if-still-dropping --attribute-type=pnu:string`.

#![allow(clippy::doc_markdown)]

use std::path::Path;

use thiserror::Error;
use tracing::{info, instrument};

use sp9_base_layer_config::Layer as Sp9Layer;
use super::spawn::{build_command, Arg, Host, SpawnError};

/// tippecanoe 빌드 한 번 = 한 layer.
#[derive(Debug, Clone, Copy)]
pub enum LayerKind {
    /// 필지 (parcels) Z14-17, layer 이름 `parcels`.
    Parcels,
    /// 행정구역 (admin) Z6-12, layer 이름 `admin`.
    Admin,
    /// 산업단지 (complex) Z0-16, layer 이름 `complex`. 모든 zoom 에서 visible.
    Complex,
}

impl LayerKind {
    /// 모든 variant — `sp9_base_layer_config::Layer::ALL` 로부터 derive.
    /// **SSOT**: 새 layer 추가 시 `sp9_base_layer_config::Layer` 에만 추가하면 됨.
    /// `From<Sp9Layer>` 가 exhaustive match 라 compiler 가 누락 차단.
    /// **주의**: ETL matrix / promote 검증은 [`Self::active_vec`] 사용 — 본 iterator 는
    /// inactive layer (admin/complex) 도 포함. registry / 전수 검증 용도.
    #[allow(dead_code)] // active_vec() 가 동일 path 사용 — registry 용도 보존
    pub fn all() -> impl Iterator<Item = Self> {
        Sp9Layer::ALL.iter().map(|l| Self::from(*l))
    }

    /// 모든 variant 의 owned vec — registry / 전수 iterate 가 필요한 callsite.
    /// **주의**: ETL matrix / promote 검증은 [`Self::active_vec`] 사용 (admin/complex 같은
    /// inactive layer 제외). 본 함수는 *registry* 용도 (Layer enum 의 모든 variant 표시).
    #[must_use]
    #[allow(dead_code)] // active_vec() 가 ETL path 의 main caller — 본 함수는 registry 보존
    pub fn all_vec() -> Vec<Self> {
        Self::all().collect()
    }

    /// **현재 ETL build-active** layer 의 owned vec — Round 4 stop-hook fix.
    /// `Sp9Layer::is_active_in_etl()` SSOT 통과한 variant 만. promote 의 staging spec
    /// 검증 / matrix iteration 이 본 함수 사용 — admin/complex 같은 inactive layer 의
    /// `MissingLineage` false-positive 차단 (ADR 0027).
    #[must_use]
    pub fn active_vec() -> Vec<Self> {
        Sp9Layer::ALL
            .iter()
            .filter(|l| l.is_active_in_etl())
            .map(|l| Self::from(*l))
            .collect()
    }

    /// PMTiles 안의 layer 이름 (프론트 `addLayer({ "source-layer": ... })` 에 매칭).
    /// **SSOT** — 프론트 `LAYER_IDS` 가 본 enum 의 reflection.
    #[must_use]
    pub const fn layer_name(self) -> &'static str {
        match self {
            Self::Parcels => "parcels",
            Self::Admin => "admin",
            Self::Complex => "complex",
        }
    }

    /// PMTiles 빌드 zoom range `(min, max)` — tippecanoe `-Z`/`-z` 인자 + manifest 박제.
    /// **SSOT** — 프론트 source 의 minzoom/maxzoom 이 본 값을 따라야 함 (manifest fetch).
    #[must_use]
    pub const fn zoom_range(self) -> (u8, u8) {
        match self {
            Self::Parcels => (14, 17),
            Self::Admin => (6, 12),
            // 산업단지: 사용자 명시 요구 — "모든 zoom level 에서 visible" (SSS).
            // tippecanoe 가 z0-5 에서 sub-pixel polygon coalesce 처리.
            Self::Complex => (0, 16),
        }
    }

    /// 프론트 `addLayer({ minzoom })` 권장값 — *render* 시작 zoom.
    /// PMTiles `min_zoom` 보다 *클* 수 있음 (e.g. parcels tile 14 부터 있지만 render 는 16+).
    #[must_use]
    pub const fn render_min_zoom(self) -> u8 {
        match self {
            Self::Parcels => 16,
            // admin: outline 은 z0 부터 visible. complex (산업단지): 사용자 요구 — 모든 zoom 에서
            // render. 둘 다 0 이라 같은 arm.
            Self::Admin | Self::Complex => 0,
        }
    }

    /// 프론트 `addLayer({ maxzoom })` 권장값 (render 종료). `None` = mapbox-gl default 24.
    #[must_use]
    pub const fn render_max_zoom(self) -> Option<u8> {
        match self {
            Self::Admin => Some(16),
            _ => None,
        }
    }

    /// CDN `Cache-Control: max-age=<seconds>` — layer 별 차별화 (gongzzang-develop 차용).
    /// flat tile 은 immutable (URL versioning 으로 무효화) → 1년.
    /// 향후 layer 별 차등 (e.g. complex 일 6시간) 가능성 위해 `self` 인자 보존.
    #[must_use]
    #[allow(clippy::unused_self)]
    pub const fn cache_max_age_seconds(self) -> u32 {
        // 31_536_000s = 365일. immutable + URL versioning 패턴 (ADR 0021 § Tier A).
        31_536_000
    }
}
/// SSOT 브리지 — `sp9_base_layer_config::Layer` → `LayerKind` 자동 변환.
/// `Layer::ALL` 이 추가되면 컴파일러가 이 match 에서 누락 variant 를 차단.
impl From<Sp9Layer> for LayerKind {
    fn from(l: Sp9Layer) -> Self {
        match l {
            Sp9Layer::Parcels => Self::Parcels,
            Sp9Layer::Admin => Self::Admin,
            Sp9Layer::Complex => Self::Complex,
        }
    }
}

/// tippecanoe 실행 설정.
#[derive(Debug, Clone)]
pub struct TippecanoeArgs<'a> {
    /// layer kind — zoom range / layer name 결정.
    pub kind: LayerKind,
    /// 입력 GeoJSON 파일들 (1개 이상).
    pub inputs: &'a [&'a Path],
    /// 출력 .pmtiles 경로.
    pub output: &'a Path,
}

/// tippecanoe 결과.
#[derive(Debug, Clone)]
pub struct TippecanoeResult {
    /// 출력 파일 크기 (bytes) — sanity 검증 (너무 작거나 크면 실패).
    pub output_bytes: u64,
    /// tippecanoe 가 `--metadata-json` 에 박제한 실제 feature 수.
    /// tippecanoe 버전이 해당 필드를 지원하지 않으면 `None`.
    pub feature_count: Option<u64>,
}

/// tippecanoe 에러.
#[derive(Debug, Error)]
pub enum TippecanoeError {
    /// command 빌드 단계 (program 이름 비어있음 등).
    #[error("spawn build failed: {0}")]
    Build(#[from] SpawnError),
    /// spawn / wait / I/O 에러.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// non-zero exit code — stderr 가 함께 캡처됨.
    #[error("tippecanoe exited with {code}: {stderr}")]
    Failed {
        /// exit code (signal kill 시 -1).
        code: i32,
        /// stderr 마지막 4KB (전체 캡처는 너무 큼).
        stderr: String,
    },
    /// 입력 inputs 가 비어있음.
    #[error("no input files provided")]
    NoInputs,
    /// 출력 파일이 안 만들어짐 (tippecanoe 가 silent fail).
    #[error("output file {path} not created")]
    OutputMissing {
        /// 기대한 출력 경로.
        path: String,
    },
}

/// tippecanoe binary 가 실행 가능한지 빠르게 검사 (`--version`).
///
/// 환경 점검용 — 실 빌드 직전 호출하면 친절한 에러 가능.
///
/// Round 5+ (Codex audit): SSOT `TIPPECANOE_GIT_SHA` 와 *실제 설치 SHA* 비교 검사.
/// dev WSL 환경에서 `scripts/setup-dev-tippecanoe.sh` 가 박제한 `.sp9-tippecanoe-sha`
/// 파일 검사 → mismatch 시 warning 로그 (production CI 는 workflow 가 직접 SHA pin
/// 빌드라 본 검사 skip 자연 통과).
///
/// # Errors
///
/// spawn 실패 / non-zero exit.
pub async fn check_available(host: Host) -> Result<String, TippecanoeError> {
    let mut cmd = build_command(host, "tippecanoe", &[Arg::Lit("--version")])?;
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(TippecanoeError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    // tippecanoe 는 --version 을 stderr 로 출력하기도 함 — 양쪽 합쳐서 반환.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    // SSOT SHA 검사 (best-effort). 미일치 시 warning — local dev 의 capability
    // detection trick 완전 제거 path 의 첫 단계. ADR 0028 + 0029 후속.
    check_ssot_sha();

    Ok(combined.trim().to_owned())
}

/// SSOT `TIPPECANOE_GIT_SHA` 와 dev tooling 박제 SHA 비교. mismatch 시 warning.
///
/// 박제 파일 (`/usr/local/bin/.sp9-tippecanoe-sha`) 는 `scripts/setup-dev-tippecanoe.sh`
/// 가 생성. 파일 부재 = dev 환경이 SSOT 스크립트 미실행 (예: upstream tippecanoe 직접
/// apt install). 이 경우도 warning — operator 가 setup script 실행하도록 유도.
fn check_ssot_sha() {
    const SHA_FILE: &str = "/usr/local/bin/.sp9-tippecanoe-sha";
    let Ok(raw) = std::fs::read_to_string(SHA_FILE) else {
        // 파일 부재 — dev 환경 미설정 가능성. CI 는 workflow 가 직접 SHA 빌드라
        // 본 파일 없음. 즉 trace 만 debug level (CI noise 회피).
        tracing::debug!(
            sha_file = SHA_FILE,
            ssot_sha = sp9_base_layer_config::TIPPECANOE_GIT_SHA,
            "tippecanoe SHA file 부재 — CI/Container 환경이면 정상, dev 환경이면 scripts/setup-dev-tippecanoe.sh 실행 권장"
        );
        return;
    };
    let installed = raw.trim();
    if installed == sp9_base_layer_config::TIPPECANOE_GIT_SHA {
        tracing::info!(sha = installed, "tippecanoe SHA matches SSOT");
    } else {
        tracing::warn!(
            installed,
            ssot = sp9_base_layer_config::TIPPECANOE_GIT_SHA,
            "tippecanoe SHA mismatch with SSOT — capability drift 위험. 실행: scripts/setup-dev-tippecanoe.sh"
        );
    }
}

/// tippecanoe 실행. `args.inputs` 의 GeoJSON 들을 한 PMTiles 로 빌드.
///
/// flag 는 [`reference_flags`] 가 결정 — design-lab 의 검증된 셋과 동일.
///
/// # Errors
///
/// spawn 실패 / non-zero exit / output 미생성.
#[instrument(skip(host, args), fields(layer = %args.kind.layer_name(), output = %args.output.display()))]
pub async fn run(
    host: Host,
    args: &TippecanoeArgs<'_>,
) -> Result<TippecanoeResult, TippecanoeError> {
    if args.inputs.is_empty() {
        return Err(TippecanoeError::NoInputs);
    }

    let (min_z, max_z) = args.kind.zoom_range();
    let layer_name = args.kind.layer_name();
    let min_z_str = min_z.to_string();
    let max_z_str = max_z.to_string();

    let mut spawn_args: Vec<Arg<'_>> = vec![
        Arg::Lit("-o"),
        Arg::Path(args.output),
        Arg::Lit("-l"),
        Arg::Lit(layer_name),
        Arg::Lit("-P"),
        Arg::Lit("-Z"),
        Arg::Lit(&min_z_str),
        Arg::Lit("-z"),
        Arg::Lit(&max_z_str),
        // SSS 화 (사용자 needs: 폴리곤 망가짐/비틀림 0, 사라짐 0, 생긴거 그대로):
        // - simplification=1: 최소 simplification (default 12 → 1, epsilon ~2mm 수준).
        //   maxzoom 에서는 항상 0 — 정확히 원본 (tippecanoe invariant).
        // - coalesce-smallest-as-needed: 작은 polygon 'drop' → 'merge' (사라짐 0)
        // - detect-shared-borders: 인접 polygon boundary 정확히 일치 (틈 0, 겹침 0)
        // - no-tiny-polygon-reduction: 저줌 (z0-5) 에서도 작은 polygon 이 *점* 으로
        //   reduce 안 됨 — 산단 "모든 zoom 에서 visible" 요구사항.
        // - maximum-tile-bytes 4MB: detail 보존, default 500KB 보다 8x
        Arg::Lit("--no-feature-limit"),
        Arg::Lit("--no-tile-size-limit"),
        Arg::Lit("--no-tiny-polygon-reduction"),
        Arg::Lit("--force"),
        Arg::Lit("--coalesce-smallest-as-needed"),
        Arg::Lit("--detect-shared-borders"),
        Arg::Lit("--simplification=1"),
        Arg::Lit("--maximum-tile-bytes=4000000"),
        Arg::Lit("--extend-zooms-if-still-dropping"),
        Arg::Lit("--attribute-type=pnu:string"),
    ];
    for input in args.inputs {
        spawn_args.push(Arg::Path(input));
    }

    info!(
        inputs = args.inputs.len(),
        min_zoom = min_z,
        max_zoom = max_z,
        "tippecanoe starting"
    );

    // --metadata-json: tippecanoe (felt fork) 가 빌드 메타(feature_count) 를 별도 파일
    // 로 박제. SSOT SHA pin 의 tippecanoe 빌드만 본 flag 지원 — local dev WSL 의
    // 공식 tippecanoe 2.80 같은 변종은 미지원 (Round 5 verify smoke 발견).
    //
    // Capability detection — `--help` 출력에서 `--metadata-json` 존재 검사 후 추가.
    // 미지원 시 graceful skip — `feature_count` 가 `None` 으로 lineage 박제 (이미
    // honest absence semantic).
    let metadata_file = args.output.with_extension("tippecanoe-meta.json");
    let supports_metadata_json = check_supports_flag(host, "--metadata-json").await;
    if supports_metadata_json {
        spawn_args.push(Arg::Lit("--metadata-json"));
        // metadata_file_str 은 String 이라 Arg::Lit lifetime 불일치 — Arg::Path 사용.
        spawn_args.push(Arg::Path(&metadata_file));
    } else {
        info!("tippecanoe missing --metadata-json (likely upstream build, not felt fork) — feature_count will be None");
    }

    let mut cmd = build_command(host, "tippecanoe", &spawn_args)?;
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // 마지막 4KB 만 — 큰 입력 시 stderr 가 매우 길 수 있음.
        let trimmed = if stderr.len() > 4096 {
            stderr[stderr.len() - 4096..].to_owned()
        } else {
            stderr.into_owned()
        };
        return Err(TippecanoeError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: trimmed,
        });
    }

    // 검증 — 출력 파일 존재 + 크기.
    let meta =
        tokio::fs::metadata(args.output)
            .await
            .map_err(|_| TippecanoeError::OutputMissing {
                path: args.output.display().to_string(),
            })?;
    let output_bytes = meta.len();

    // --metadata-json 에서 feature_count 추출. capability detection 결과 false 면
    // metadata file 자체가 안 생성됨 → `extract_feature_count` 가 `None` 반환 (정상).
    let feature_count = if supports_metadata_json {
        extract_feature_count(&metadata_file).await
    } else {
        None
    };
    // 임시 메타파일 정리 — 실패해도 무시 (best-effort cleanup).
    let _ = tokio::fs::remove_file(&metadata_file).await;

    info!(bytes = output_bytes, ?feature_count, "tippecanoe complete");
    Ok(TippecanoeResult { output_bytes, feature_count })
}

/// tippecanoe 가 특정 flag 를 지원하는지 검사. `--help` 출력에서 flag 문자열 grep.
///
/// Round 5 verify smoke 발견: SSOT SHA pin (felt fork) 의 tippecanoe 빌드는
/// `--metadata-json` 지원, local dev 환경의 official tippecanoe 2.80 은 미지원.
/// 본 helper 는 capability detection — production CI 에서는 항상 `true` (SSOT 빌드).
async fn check_supports_flag(host: Host, flag: &str) -> bool {
    let Ok(mut cmd) = build_command(host, "tippecanoe", &[Arg::Lit("--help")]) else {
        return false;
    };
    let Ok(output) = cmd.output().await else {
        return false;
    };
    // tippecanoe 의 --help 는 stderr 로 나가는 빌드도 있고 stdout 으로 나가는 빌드도 있음.
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    combined.contains(flag)
}

/// `--metadata-json` 파일에서 feature count 를 읽어 반환.
/// tippecanoe 가 `{"layers":[{"features": N}]}` 형태로 박제. 필드 부재 / 파싱 실패 = `None`.
async fn extract_feature_count(metadata_file: &std::path::Path) -> Option<u64> {
    let bytes = tokio::fs::read(metadata_file).await.ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    // tippecanoe metadata schema: top-level `"features"` 또는 `"layers"[0]["features"]`.
    if let Some(n) = v.get("features").and_then(serde_json::Value::as_u64) {
        return Some(n);
    }
    // 일부 버전: `"vector_layers"` or `"tilestats" -> "layers"[0]["count"]`.
    if let Some(layers) = v
        .get("tilestats")
        .and_then(|ts| ts.get("layers"))
        .and_then(|l| l.as_array())
    {
        let total: u64 = layers
            .iter()
            .filter_map(|layer| layer.get("count").and_then(serde_json::Value::as_u64))
            .sum();
        if total > 0 {
            return Some(total);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn layer_kind_metadata() {
        assert_eq!(LayerKind::Parcels.layer_name(), "parcels");
        assert_eq!(LayerKind::Parcels.zoom_range(), (14, 17));
        assert_eq!(LayerKind::Admin.layer_name(), "admin");
        assert_eq!(LayerKind::Admin.zoom_range(), (6, 12));
        assert_eq!(LayerKind::Complex.layer_name(), "complex");
        assert_eq!(LayerKind::Complex.zoom_range(), (0, 16));
        assert_eq!(LayerKind::Complex.render_min_zoom(), 0);
    }

    #[test]
    fn all_vec_matches_sp9_layer_all_in_count_and_name() {
        // SSOT 보증 — `LayerKind::all_vec()` 가 `Sp9Layer::ALL` 의 reflection.
        // Sp9Layer 에 새 variant 추가 시 `From<Sp9Layer> for LayerKind` 의 exhaustive
        // match 가 컴파일러 에러로 차단 — 본 test 는 *layer_name 일관성* 추가 검증.
        let kinds = LayerKind::all_vec();
        let layers = Sp9Layer::ALL;
        assert_eq!(kinds.len(), layers.len(), "count drift between LayerKind and Sp9Layer");
        for (kind, layer) in kinds.iter().zip(layers.iter()) {
            assert_eq!(
                kind.layer_name(),
                layer.name(),
                "name drift: LayerKind={kind:?} vs Sp9Layer={layer:?}",
            );
        }
    }

    /// Round 4 stop-hook fix — `active_vec()` 는 `is_active_in_etl()=true` 만. promote
    /// 가 본 함수 통과해야 inactive layer (admin/complex) 의 staging spec 미박제로 인한
    /// `MissingLineage` false-positive 차단.
    #[test]
    fn active_vec_excludes_inactive_layers() {
        let active = LayerKind::active_vec();
        // ADR 0027 — 현재 active = parcels 만.
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].layer_name(), "parcels");
    }

    /// SSOT alignment — `LayerKind::active_vec` 와 `Sp9Layer::is_active_in_etl` 이
    /// 동일한 active set 을 반환해야 함. 둘이 drift 하면 promote / workflow matrix /
    /// SSOT JSON 출력 셋이 어긋남.
    #[test]
    fn active_vec_matches_sp9_is_active_in_etl() {
        let active_names: Vec<&str> = LayerKind::active_vec()
            .iter()
            .map(|k| k.layer_name())
            .collect();
        let expected: Vec<&str> = Sp9Layer::ALL
            .iter()
            .filter(|l| l.is_active_in_etl())
            .map(|l| l.name())
            .collect();
        assert_eq!(active_names, expected);
    }

    #[tokio::test]
    async fn no_inputs_returns_error() {
        let out = PathBuf::from("/tmp/x.pmtiles");
        let args = TippecanoeArgs {
            kind: LayerKind::Parcels,
            inputs: &[],
            output: &out,
        };
        let err = run(Host::Native, &args).await.unwrap_err();
        assert!(matches!(err, TippecanoeError::NoInputs));
    }
}
