use std::path::PathBuf;
use std::process::ExitCode;

use tracing::{error, info};

use crate::dtmk_prepare::prepare_dtmk_inputs;
use crate::gold::build::{build_layer, BuildResult};
use crate::gold::manifest::BronzeInput;
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::{check_available, LayerKind};
use crate::gold_upload::upload_gold_if_configured;
use crate::verify_cli::run_verify;
use sp9_base_layer_config::Srs;

pub struct GoldOpts {
    pub layer: LayerKind,
    pub output_dir: PathBuf,
    /// 사용자가 직접 준비한 `GeoJSON` 입력 (positional). `--bronze-prefix` 와 mutually exclusive.
    pub inputs: Vec<PathBuf>,
    /// R2 Bronze prefix (예: `bronze/2026-05/parcel-dtmk-30563/`). 지정 시 dtmk pipeline
    /// (R2 다운 + unzip + ogr2ogr) 가 `GeoJSON` 입력 자동 생성.
    pub bronze_prefix: Option<String>,
    /// dtmk pipeline 작업 디렉터리. 기본 `<output_dir>/../dtmk-work`.
    pub work_dir: Option<PathBuf>,
    /// dtmk pipeline 다운로드 동시성 (기본 8).
    pub concurrency: usize,
    /// 입력 SHP 의 source SRS (newtype — `EPSG:<digits>` 강제).
    pub source_srs: Srs,
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
    let mut bronze_prefix: Option<String> = None;
    let mut work_dir: Option<PathBuf> = None;
    // SSOT: sp9_base_layer_config 의 default. CLI flag 로 override 가능.
    let mut concurrency: usize = sp9_base_layer_config::DTMK_DOWNLOAD_CONCURRENCY;
    // SSOT default — `Srs::new` 를 거치므로 SOURCE_SRS_VWORLD 가 invalid 면 컴파일 후
    // 첫 호출에서 fail-fast.
    let mut source_srs: Srs = Srs::new(sp9_base_layer_config::SOURCE_SRS_VWORLD)
        .map_err(|e| ArgError(format!("SOURCE_SRS_VWORLD invalid: {e}")))?;
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
            "--bronze-prefix" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--bronze-prefix needs a value".into()))?;
                bronze_prefix = Some(v.clone());
            }
            "--work-dir" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--work-dir needs a value".into()))?;
                work_dir = Some(PathBuf::from(v));
            }
            "--concurrency" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--concurrency needs a value".into()))?;
                concurrency = v
                    .parse::<usize>()
                    .map_err(|e| ArgError(format!("--concurrency parse: {e}")))?;
            }
            "--source-srs" => {
                let v = iter
                    .next()
                    .ok_or_else(|| ArgError("--source-srs needs a value".into()))?;
                source_srs = Srs::new(v.as_str())
                    .map_err(|e| ArgError(format!("--source-srs parse: {e}")))?;
            }
            other => inputs.push(PathBuf::from(other)),
        }
    }
    let layer = layer.ok_or_else(|| ArgError("--layer is required".into()))?;
    let output_dir = output_dir.ok_or_else(|| ArgError("--output is required".into()))?;
    if inputs.is_empty() && bronze_prefix.is_none() {
        return Err(ArgError(
            "either positional GeoJSON inputs or --bronze-prefix is required".into(),
        ));
    }
    if !inputs.is_empty() && bronze_prefix.is_some() {
        return Err(ArgError(
            "--bronze-prefix and positional inputs are mutually exclusive".into(),
        ));
    }
    Ok(GoldOpts {
        layer,
        output_dir,
        inputs,
        bronze_prefix,
        work_dir,
        concurrency,
        source_srs,
    })
}

// CLI dispatch — 동일 사유 (cognitive complexity allow).
pub async fn run_gold(args: Vec<String>) -> ExitCode {
    let opts = match parse_gold_opts_or_exit(&args) {
        Ok(opts) => opts,
        Err(code) => return code,
    };

    let host = Host::detect();
    log_gold_start(host, &opts);

    if let Err(code) = ensure_tippecanoe_available(host).await {
        return code;
    }

    let inputs = match resolve_gold_inputs(host, &opts).await {
        Ok(inputs) => inputs,
        Err(code) => return code,
    };

    let result = match build_gold_result(host, &opts, &inputs.geojson_inputs).await {
        Ok(result) => result,
        Err(code) => return code,
    };

    if let Err(code) = verify_gold_result(host, &result, opts.layer).await {
        return code;
    }
    log_gold_complete(&result);

    upload_gold_if_configured(&opts, &result, inputs.bronze_inputs).await
}

fn parse_gold_opts_or_exit(args: &[String]) -> Result<GoldOpts, ExitCode> {
    parse_gold_args(args).map_err(|e| {
        error!(error = %e, "gold args parse failed");
        ExitCode::from(2)
    })
}

fn log_gold_start(host: Host, opts: &GoldOpts) {
    info!(
        host = ?host,
        layer = %opts.layer.layer_name(),
        output_dir = %opts.output_dir.display(),
        positional_inputs = opts.inputs.len(),
        bronze_prefix = ?opts.bronze_prefix,
        "starting gold build"
    );
}

async fn ensure_tippecanoe_available(host: Host) -> Result<(), ExitCode> {
    let version = check_available(host).await.map_err(|e| {
        error!(error = %e, "tippecanoe not available ??set ETL_WSL_DISTRO if not Ubuntu, or apt install tippecanoe in WSL");
        ExitCode::from(2)
    })?;
    info!(version = %version, "tippecanoe available");
    Ok(())
}

#[derive(Debug)]
struct GoldInputSet {
    geojson_inputs: Vec<PathBuf>,
    bronze_inputs: Vec<BronzeInput>,
}

async fn resolve_gold_inputs(host: Host, opts: &GoldOpts) -> Result<GoldInputSet, ExitCode> {
    let (geojson_inputs, bronze_inputs) = match opts.bronze_prefix.as_deref() {
        Some(prefix) => prepare_dtmk_inputs(host, opts, prefix).await.map_err(|e| {
            error!(error = %e, "dtmk preparation failed");
            ExitCode::FAILURE
        })?,
        None => (opts.inputs.clone(), Vec::new()),
    };

    if geojson_inputs.is_empty() {
        error!("no GeoJSON inputs after preparation ??aborting");
        return Err(ExitCode::FAILURE);
    }

    Ok(GoldInputSet {
        geojson_inputs,
        bronze_inputs,
    })
}

async fn build_gold_result(
    host: Host,
    opts: &GoldOpts,
    geojson_inputs: &[PathBuf],
) -> Result<BuildResult, ExitCode> {
    let input_refs: Vec<&std::path::Path> = geojson_inputs.iter().map(PathBuf::as_path).collect();
    build_layer(host, &opts.output_dir, opts.layer, &input_refs)
        .await
        .map_err(|e| {
            error!(error = %e, "gold build failed");
            ExitCode::FAILURE
        })
}

async fn verify_gold_result(
    host: Host,
    result: &BuildResult,
    layer: LayerKind,
) -> Result<(), ExitCode> {
    run_verify(host, result, layer).await.map_err(|e| {
        error!(error = %e, "L2 verification failed");
        ExitCode::FAILURE
    })
}

fn log_gold_complete(result: &BuildResult) {
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
}
