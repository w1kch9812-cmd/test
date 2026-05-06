//! Gold 빌드 오케스트레이터 — 입력 `GeoJSON` 들 → tippecanoe → 단일 `PMTiles`.
//!
//! 본 단계가 의도적으로 thin 한 이유: ogr2ogr 변환은 호출자가 결정 (V-World `GeoJSON`
//! 은 이미 EPSG:4326 → 우회 가능, 공공데이터포털 SHP 만 SHP→GeoJSON step 필요).
//!
//! T3b.2 로컬 모드:
//! - R2 업로드 disabled
//! - 출력 `<gold_dir>/<layer>.pmtiles` (예: `./var/gold/v_local/parcels.pmtiles`)
//!
//! T6 production 모드 (T3b.2 종료 후 별도 commit):
//! - 빌드 후 R2 업로드 + Gold manifest activate (T3b.1 [`R2Uploader`] 재사용)

use std::path::{Path, PathBuf};

use thiserror::Error;
use tracing::{info, instrument};

use super::spawn::Host;
use super::tippecanoe::{self, LayerKind, TippecanoeArgs, TippecanoeError};

/// Gold 빌드 한 번 결과.
#[derive(Debug, Clone)]
pub struct BuildResult {
    /// 출력 경로 (`<gold_dir>/<layer>.pmtiles`).
    pub output_path: PathBuf,
    /// 출력 파일 크기.
    pub output_bytes: u64,
}

/// Gold 빌드 에러.
#[derive(Debug, Error)]
pub enum BuildError {
    /// tippecanoe 단계 실패.
    #[error("tippecanoe: {0}")]
    Tippecanoe(#[from] TippecanoeError),
    /// `gold_dir` 생성 실패 / 출력 검증 실패.
    #[error("io error at {path}: {source}")]
    Io {
        /// 대상 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
}

/// 한 layer 빌드 — `inputs` 의 `GeoJSON` 들을 합쳐 단일 `PMTiles` 로.
///
/// 출력 파일명은 `<layer>.pmtiles` 로 고정 — 프론트
/// `pmtilesSourceUrl("parcels.pmtiles")` 와 매칭.
///
/// # Errors
///
/// 디렉터리 생성 실패 / tippecanoe 실패 / 출력 검증 실패.
#[instrument(
    skip(host, gold_dir, inputs),
    fields(layer = %kind.layer_name(), gold_dir = %gold_dir.display(), inputs = inputs.len()),
)]
pub async fn build_layer(
    host: Host,
    gold_dir: &Path,
    kind: LayerKind,
    inputs: &[&Path],
) -> Result<BuildResult, BuildError> {
    tokio::fs::create_dir_all(gold_dir)
        .await
        .map_err(|source| BuildError::Io {
            path: gold_dir.display().to_string(),
            source,
        })?;

    let output_path = gold_dir.join(format!("{}.pmtiles", kind.layer_name()));

    let args = TippecanoeArgs {
        kind,
        inputs,
        output: &output_path,
    };
    let result = tippecanoe::run(host, &args).await?;

    info!(
        output = %output_path.display(),
        bytes = result.output_bytes,
        "Gold layer build complete"
    );

    Ok(BuildResult {
        output_path,
        output_bytes: result.output_bytes,
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn empty_inputs_returns_no_inputs_error() {
        let dir = PathBuf::from("./var/test_gold_empty");
        let err = build_layer(Host::Native, &dir, LayerKind::Parcels, &[])
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            BuildError::Tippecanoe(TippecanoeError::NoInputs)
        ));
    }
}
