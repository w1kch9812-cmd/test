//! ogr2ogr spawn — SHP (EPSG:5179, 한국 국토표준) → `GeoJSON` (EPSG:4326).
//!
//! production cron (T6) 에서 공공데이터포털 SHP 입력 시 사용. 로컬 smoke 는 V-World
//! `GeoJSON` 이 이미 EPSG:4326 이라 본 모듈 우회 가능.
//!
//! ogr2ogr (GDAL) 호출 형식:
//! ```sh
//! ogr2ogr -f `GeoJSON` -t_srs EPSG:4326 -s_srs EPSG:5179 \
//!         output.geojson input.shp
//! ```
//!
//! `-skipfailures` 는 의도적 미사용 — 좌표 변환 실패 1건도 빌드 abort 하는 게 안전.
//!
//! T3b.2 로컬 smoke 는 V-World `GeoJSON` 으로 시작 (이미 EPSG:4326) → 본 모듈은 미사용.
//! T6 production cron 의 SHP 입력에서 활성. `dead_code` 경고는 그래서 의도적 silence.

#![allow(dead_code)]

use std::path::Path;

use thiserror::Error;
use tracing::{info, instrument};

use super::spawn::{build_command, Arg, Host, SpawnError};

/// ogr2ogr 변환 설정.
///
/// `source_srs` 는 명시 — SHP 의 .prj 가 누락된 케이스가 공공데이터포털에 존재.
#[derive(Debug, Clone)]
pub struct Ogr2OgrArgs<'a> {
    /// 입력 SHP (또는 .shp.zip 압축 풀린 경로).
    pub input_shp: &'a Path,
    /// 출력 `GeoJSON`.
    pub output_geojson: &'a Path,
    /// source SRS (예: `EPSG:5179`). 공공데이터포털 표준.
    pub source_srs: &'a str,
    /// target SRS (보통 `EPSG:4326`).
    pub target_srs: &'a str,
}

/// ogr2ogr 에러.
#[derive(Debug, Error)]
pub enum Ogr2OgrError {
    /// command 빌드 단계.
    #[error("spawn build failed: {0}")]
    Build(#[from] SpawnError),
    /// spawn / wait / I/O.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// non-zero exit.
    #[error("ogr2ogr exited with {code}: {stderr}")]
    Failed {
        /// exit code.
        code: i32,
        /// stderr 마지막 2KB.
        stderr: String,
    },
    /// 출력 파일이 안 만들어짐.
    #[error("output {path} not created")]
    OutputMissing {
        /// 기대한 출력 경로.
        path: String,
    },
}

/// ogr2ogr 가용성 검사 (`--version`).
///
/// # Errors
///
/// spawn 실패 / non-zero exit.
pub async fn check_available(host: Host) -> Result<String, Ogr2OgrError> {
    let mut cmd = build_command(host, "ogr2ogr", &[Arg::Lit("--version")])?;
    let output = cmd.output().await?;
    if !output.status.success() {
        return Err(Ogr2OgrError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// ogr2ogr 실행 — SHP → `GeoJSON`.
///
/// # Errors
///
/// spawn 실패 / non-zero exit / output 미생성.
#[instrument(
    skip(host, args),
    fields(
        input = %args.input_shp.display(),
        output = %args.output_geojson.display(),
        srs = format!("{}->{}", args.source_srs, args.target_srs),
    ),
)]
pub async fn run(host: Host, args: &Ogr2OgrArgs<'_>) -> Result<(), Ogr2OgrError> {
    info!("ogr2ogr starting");

    let mut cmd = build_command(
        host,
        "ogr2ogr",
        &[
            Arg::Lit("-f"),
            Arg::Lit("GeoJSON"),
            Arg::Lit("-t_srs"),
            Arg::Lit(args.target_srs),
            Arg::Lit("-s_srs"),
            Arg::Lit(args.source_srs),
            // 동일 출력 경로 덮어쓰기 허용.
            Arg::Lit("-overwrite"),
            Arg::Path(args.output_geojson),
            Arg::Path(args.input_shp),
        ],
    )?;
    let output = cmd.output().await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let trimmed = if stderr.len() > 2048 {
            stderr[stderr.len() - 2048..].to_owned()
        } else {
            stderr.into_owned()
        };
        return Err(Ogr2OgrError::Failed {
            code: output.status.code().unwrap_or(-1),
            stderr: trimmed,
        });
    }

    // 출력 검증.
    if tokio::fs::metadata(args.output_geojson).await.is_err() {
        return Err(Ogr2OgrError::OutputMissing {
            path: args.output_geojson.display().to_string(),
        });
    }
    info!("ogr2ogr complete");
    Ok(())
}
