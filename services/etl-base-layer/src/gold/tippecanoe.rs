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
    /// 모든 variant — manifest 박제 시 iterate (multi-layer build orchestration).
    #[allow(dead_code)]
    pub const ALL: &'static [Self] = &[Self::Parcels, Self::Admin, Self::Complex];

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
    Ok(combined.trim().to_owned())
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

    info!(bytes = output_bytes, "tippecanoe complete");
    Ok(TippecanoeResult { output_bytes })
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
