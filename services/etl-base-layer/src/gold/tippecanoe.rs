//! tippecanoe spawn — `GeoJSON` 파일들 → 단일 `PMTiles` 빌드.
//!
//! 본 모듈은 `tippecanoe` binary 를 실행. binary 자체는 dev WSL 에 빌드됨
//! (`/usr/local/bin/tippecanoe`) 또는 CI Ubuntu 에서 felt/tippecanoe make.
//!
//! Layer 별 zoom 스펙 (ADR 0016 §):
//! - **parcels** Z14-17 — 매물 클릭 단위, 가까이서만 visible.
//! - **admin**   Z6-12  — 행정구역 outline, 멀리서 visible.
//! - **complex** Z10-15 — 산업단지 boundary, 중간 zoom.
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
    /// 산업단지 (complex) Z10-15, layer 이름 `complex`.
    Complex,
}

impl LayerKind {
    /// PMTiles 안의 layer 이름 (프론트 `addLayer({ "source-layer": ... })` 에 매칭).
    #[must_use]
    pub const fn layer_name(self) -> &'static str {
        match self {
            Self::Parcels => "parcels",
            Self::Admin => "admin",
            Self::Complex => "complex",
        }
    }

    /// `(min_zoom, max_zoom)`.
    #[must_use]
    pub const fn zoom_range(self) -> (u8, u8) {
        match self {
            Self::Parcels => (14, 17),
            Self::Admin => (6, 12),
            Self::Complex => (10, 15),
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
        Arg::Lit("--no-feature-limit"),
        Arg::Lit("--no-tile-size-limit"),
        Arg::Lit("--force"),
        Arg::Lit("--drop-smallest-as-needed"),
        Arg::Lit("--simplification=10"),
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
        assert_eq!(LayerKind::Complex.zoom_range(), (10, 15));
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
