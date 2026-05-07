//! 공짱 `PMTiles` base layer ETL — Bronze SHP 다운로드 + Gold `PMTiles` 빌드.
//!
//! ## 서브커맨드
//!
//! - `etl-base-layer` (또는 `etl-base-layer bronze`) — Bronze 다운로드 (T3a + T3b.1 R2)
//! - `etl-base-layer gold --layer parcels --output <gold_dir> <input.geojson>...` — `tippecanoe` 빌드 (T3b.2)
//!
//! ## Bronze 실행
//!
//! ```sh
//! BRONZE_PARCEL_SHP_URL=https://www.data.go.kr/.../parcel.shp.zip \
//! BRONZE_DIR=./var/bronze \
//! cargo run -p etl-base-layer -- bronze
//! ```
//!
//! ## Gold 실행 (로컬, R2 미사용)
//!
//! ```sh
//! cargo run -p etl-base-layer -- gold \
//!     --layer parcels \
//!     --output ./var/gold \
//!     ./var/sample/gangnam.geojson
//! ```
//!
//! Windows dev 환경에서 자동으로 `wsl.exe -d Ubuntu -- tippecanoe ...` 로 라우팅.
//! 다른 distro 면 `ETL_WSL_DISTRO=<name>` 환경변수.

#![forbid(unsafe_code)]
// main.rs: init failure panic 은 정답이라 expect/unwrap 허용.
#![allow(clippy::expect_used, clippy::unwrap_used)]
// FU 26 — etl-base-layer 는 일회성 batch CLI. circuit-breaker wrapping 은 T3b.2 에서
// retry 정책 함께 검토 (월 1회 cron 이라 외부 dependency 우선순위 낮음).
#![allow(clippy::disallowed_types)]

mod bronze;
mod config;
mod gold;
mod manifest;
mod r2_upload;

use std::path::PathBuf;
use std::process::ExitCode;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::gold::build::build_layer;
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::{check_available, LayerKind};

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let subcommand = args.first().map_or("bronze", String::as_str);

    match subcommand {
        "bronze" | "" => run_bronze().await,
        "gold" => run_gold(&args[1..]).await,
        other => {
            error!(subcommand = %other, "unknown subcommand — use `bronze` or `gold`");
            ExitCode::from(2)
        }
    }
}

// CLI dispatch — config 로드 / 클라이언트 빌드 / spawn / error mapping 이 한 함수에
// 모이는 게 자연스러움 (split 시 가독성 손해). 복잡도 lint 의도적 silence.
#[allow(clippy::cognitive_complexity)]
async fn run_bronze() -> ExitCode {
    let cfg = Config::from_env();

    if cfg.sources.is_empty() {
        error!(
            "no Bronze sources configured — set BRONZE_PARCEL_SHP_URL / BRONZE_ADMIN_SHP_URL / BRONZE_COMPLEX_GEOJSON_URL"
        );
        return ExitCode::from(2);
    }

    info!(
        batch_label = %cfg.batch_label,
        bronze_dir = %cfg.bronze_dir.display(),
        sources = cfg.sources.len(),
        r2_active = cfg.r2.is_some(),
        "starting bronze fetch (SP9 T3a + T3b.1)"
    );

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "reqwest client build failed");
            return ExitCode::from(2);
        }
    };

    match bronze::run_bronze(&client, &cfg).await {
        Ok(manifest) => {
            info!(
                sources_completed = manifest.sources.len(),
                "bronze fetch complete"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, "bronze fetch failed");
            ExitCode::FAILURE
        }
    }
}

/// CLI 옵션 (gold 전용).
struct GoldOpts {
    layer: LayerKind,
    output_dir: PathBuf,
    inputs: Vec<PathBuf>,
}

#[derive(Debug)]
struct ArgError(String);

impl std::fmt::Display for ArgError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn parse_layer(s: &str) -> Result<LayerKind, ArgError> {
    match s {
        "parcels" => Ok(LayerKind::Parcels),
        "admin" => Ok(LayerKind::Admin),
        "complex" => Ok(LayerKind::Complex),
        other => Err(ArgError(format!(
            "unknown layer `{other}` — must be parcels | admin | complex"
        ))),
    }
}

fn parse_gold_args(args: &[String]) -> Result<GoldOpts, ArgError> {
    let mut layer: Option<LayerKind> = None;
    let mut output_dir: Option<PathBuf> = None;
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--layer" | "-l" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--layer needs a value".into()))?;
                layer = Some(parse_layer(v)?);
            }
            "--output" | "-o" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--output needs a value".into()))?;
                output_dir = Some(PathBuf::from(v));
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }
    Ok(GoldOpts {
        layer: layer.ok_or_else(|| ArgError("--layer is required".into()))?,
        output_dir: output_dir.ok_or_else(|| ArgError("--output is required".into()))?,
        inputs,
    })
}

// CLI dispatch — 동일 사유 (cognitive complexity allow).
#[allow(clippy::cognitive_complexity)]
async fn run_gold(args: &[String]) -> ExitCode {
    let opts = match parse_gold_args(args) {
        Ok(o) => o,
        Err(e) => {
            error!(error = %e, "gold args parse failed");
            return ExitCode::from(2);
        }
    };

    if opts.inputs.is_empty() {
        error!("gold: at least one input GeoJSON path is required");
        return ExitCode::from(2);
    }

    let host = Host::detect();
    info!(
        host = ?host,
        layer = %opts.layer.layer_name(),
        output_dir = %opts.output_dir.display(),
        inputs = opts.inputs.len(),
        "starting gold build"
    );

    // 사전 체크 — tippecanoe 가 호출 가능한지.
    match check_available(host).await {
        Ok(version) => info!(version = %version, "tippecanoe available"),
        Err(e) => {
            error!(error = %e, "tippecanoe not available — set ETL_WSL_DISTRO if not Ubuntu, or apt install tippecanoe in WSL");
            return ExitCode::from(2);
        }
    }

    // 입력 path → &Path borrow vec.
    let input_refs: Vec<&std::path::Path> = opts.inputs.iter().map(PathBuf::as_path).collect();

    match build_layer(host, &opts.output_dir, opts.layer, &input_refs).await {
        Ok(result) => {
            // 정수 MB — display 용이라 정밀도 손실 무관. KB 단위는 너무 노이지.
            let pmtiles_mb = result.output_bytes / 1_048_576;
            let flat_mb = result.flat_tiles_total_bytes / 1_048_576;
            info!(
                pmtiles_path = %result.output_path.display(),
                pmtiles_bytes = result.output_bytes,
                pmtiles_mb = pmtiles_mb,
                flat_tiles_dir = %result.flat_tiles_dir.display(),
                flat_tile_count = result.flat_tile_count,
                flat_tiles_mb = flat_mb,
                "gold build complete (PMTiles + ADR 0021 flat tiles)"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, "gold build failed");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}
