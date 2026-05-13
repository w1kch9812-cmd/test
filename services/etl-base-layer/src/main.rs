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
// T2 (Round 2): R2 호출은 `R2Uploader` 가 `circuit-breaker::execute` 로 wrap
// (Policy::r2_default — timeout 8s, max 1 retry, open after 5 fail in 10s, 60s cooldown).
// `clippy::disallowed_types` 는 `tokio::sync::Semaphore` 등 일부 std-lib 타입 직접 사용
// (FU 26 lint 정책) 을 위한 crate-wide allow — circuit-breaker 적용과는 무관.
#![allow(clippy::disallowed_types)]

mod bronze;
mod config;
mod error;
mod gold;
mod manifest;
mod r2_upload;
#[cfg(test)]
mod test_support;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use sp9_base_layer_config::{EnvironmentParseError, R2PublicBase, Srs, Version};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::bronze::dtmk::{self, DtmkFetchArgs, DtmkFetched, SigunguArchive};
use crate::config::{Config, ConfigError};
use crate::error::{PrepareError, UploadStepError, VerifyStepError};
use crate::gold::build::{build_layer, BuildResult};
use crate::gold::manifest::{BronzeInput, BuildLineage};
use crate::gold::promote::{self, ArtifactSpec};
use crate::gold::shp_to_geojson::{self, Ogr2OgrArgs};
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::{check_available, LayerKind};
use crate::gold::verify::{
    self, lonlat_to_tile, TileCoordError, TileExpectation, TileSpec, VerifySpec,
};
use crate::r2_upload::R2Uploader;

fn main() -> ExitCode {
    // .env 자동 로드 (dev convenience). production 에서는 .env 미존재 → silent skip.
    let _ = dotenvy::dotenv();

    // L4 Observability — Sentry SDK init *before* tokio runtime.
    // SENTRY_DSN 미설정 시 sentry::init 가 no-op (silent disabled — dev / smoke 빌드).
    // ClientInitGuard drop = pending event flush (program exit 시 자동).
    let _sentry_guard = init_sentry();
    init_tracing();

    // init failure: tokio runtime 은 OS resource limit 실패 외에는 성공 보장.
    // panic 이 정답 — 프로세스 시작 실패 = 즉시 종료가 올바른 동작.
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            error!(error = %error, "tokio runtime build failed");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(async_main())
}

// audit 2026-05-08: cognitive complexity 28/15. CLI subcommand dispatch + env load
// 직선 흐름이라 분해 시 *흐름 추적 어려워짐*. 별도 refactor (file split) 시 module 분리.
async fn async_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match parse_cli_command(&args) {
        Ok(command) => command,
        Err(error) => return unknown_subcommand_exit(&error),
    };
    wait_for_cli_task_or_shutdown(spawn_cli_task(command)).await
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Bronze,
    Gold(Vec<String>),
    Promote(Vec<String>),
    CleanupManifestBackups(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UnknownSubcommand(String);

fn parse_cli_command(args: &[String]) -> Result<CliCommand, UnknownSubcommand> {
    match args.first().map_or("bronze", String::as_str) {
        "bronze" | "" => Ok(CliCommand::Bronze),
        "gold" => Ok(CliCommand::Gold(subcommand_args(args))),
        "promote" => Ok(CliCommand::Promote(subcommand_args(args))),
        "cleanup-manifest-backups" => Ok(CliCommand::CleanupManifestBackups(subcommand_args(args))),
        other => Err(UnknownSubcommand(other.to_owned())),
    }
}

fn subcommand_args(args: &[String]) -> Vec<String> {
    args.get(1..).map_or_else(Vec::new, <[String]>::to_vec)
}

fn spawn_cli_task(command: CliCommand) -> tokio::task::JoinHandle<ExitCode> {
    match command {
        CliCommand::Bronze => tokio::spawn(run_bronze()),
        CliCommand::Gold(args) => tokio::spawn(run_gold(args)),
        CliCommand::Promote(args) => tokio::spawn(async move { run_promote_cli(&args) }),
        CliCommand::CleanupManifestBackups(args) => {
            tokio::spawn(async move { run_cleanup_backups_cli(&args) })
        }
    }
}

fn unknown_subcommand_exit(error: &UnknownSubcommand) -> ExitCode {
    error!(
        subcommand = %error.0,
        "unknown subcommand -- use `bronze` | `gold` | `promote` | `cleanup-manifest-backups`"
    );
    ExitCode::from(2)
}

async fn wait_for_cli_task_or_shutdown(task: tokio::task::JoinHandle<ExitCode>) -> ExitCode {
    // L8 — graceful shutdown handler. Ctrl+C / SIGTERM 시 즉시 abort.
    // 본 결정의 정당화는 ADR 0024 (`docs/adr/0024-etl-cancel-protocol-immediate-abort.md`):
    // L3 staging atomicity 가 partial state 를 prod 에서 차단하므로 즉시 abort 가 안전.
    // tippecanoe resume 불가 + 월 1회 cron 이라 state machine 의 cost > value.
    tokio::select! {
        biased;
        result = task => {
            task_exit_code(result)
        }
        () = shutdown_signal() => {
            warn!("shutdown signal received — aborting (L3 staging spec 가 prod 보호)");
            // 130 = bash convention for SIGINT (128 + 2).
            ExitCode::from(130)
        }
    }
}

fn task_exit_code(result: Result<ExitCode, tokio::task::JoinError>) -> ExitCode {
    match result {
        Ok(code) => code,
        Err(e) => {
            error!(error = %e, "task panicked or aborted");
            ExitCode::FAILURE
        }
    }
}

/// L8 — Ctrl+C (Unix SIGINT) + Unix SIGTERM 양쪽 listen. Windows 는 `ctrl_c` 만.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Err(error) = shutdown_signal_unix().await {
            warn!(error = %error, "unix signal handler install failed; falling back to ctrl-c");
            let _ = tokio::signal::ctrl_c().await;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

#[cfg(unix)]
async fn shutdown_signal_unix() -> Result<(), std::io::Error> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate())?;
    let mut int = signal(SignalKind::interrupt())?;
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    }
    Ok(())
}

// CLI dispatch — config 로드 / 클라이언트 빌드 / spawn / error mapping 이 한 함수에
// 모이는 게 자연스러움 (split 시 가독성 손해). 복잡도 lint 의도적 silence.
#[derive(Debug)]
enum BronzeCliError {
    Config(ExitCode),
    NoSources,
    Client(reqwest::Error),
    Fetch(bronze::BronzeError),
}

impl BronzeCliError {
    fn into_exit_code(self) -> ExitCode {
        match self {
            Self::Config(code) => code,
            Self::NoSources => bronze_no_sources_exit(),
            Self::Client(error) => bronze_client_error_exit(&error),
            Self::Fetch(error) => bronze_fetch_error_exit(&error),
        }
    }
}

fn bronze_no_sources_exit() -> ExitCode {
    error!(
        "no Bronze sources configured ??set BRONZE_PARCEL_SHP_URL / BRONZE_ADMIN_SHP_URL / BRONZE_COMPLEX_GEOJSON_URL"
    );
    ExitCode::from(2)
}

fn bronze_client_error_exit(error: &reqwest::Error) -> ExitCode {
    error!(error = %error, "reqwest client build failed");
    ExitCode::from(2)
}

fn bronze_fetch_error_exit(error: &bronze::BronzeError) -> ExitCode {
    error!(error = %error, "bronze fetch failed");
    ExitCode::FAILURE
}

async fn run_bronze() -> ExitCode {
    match run_bronze_pipeline().await {
        Ok(manifest) => {
            info!(
                sources_completed = manifest.sources.len(),
                "bronze fetch complete"
            );
            ExitCode::SUCCESS
        }
        Err(error) => error.into_exit_code(),
    }
}

async fn run_bronze_pipeline() -> Result<crate::manifest::BronzeManifest, BronzeCliError> {
    let cfg = load_config_or_exit().map_err(BronzeCliError::Config)?;
    ensure_bronze_sources(&cfg)?;
    log_bronze_start(&cfg);
    let client = build_bronze_http_client().map_err(BronzeCliError::Client)?;
    bronze::run_bronze(&client, &cfg)
        .await
        .map_err(BronzeCliError::Fetch)
}

const fn ensure_bronze_sources(cfg: &Config) -> Result<(), BronzeCliError> {
    if cfg.sources.is_empty() {
        return Err(BronzeCliError::NoSources);
    }
    Ok(())
}

fn log_bronze_start(cfg: &Config) {
    info!(
        batch_label = %cfg.batch_label,
        bronze_dir = %cfg.bronze_dir.display(),
        sources = cfg.sources.len(),
        r2_active = cfg.r2.is_some(),
        "starting bronze fetch (SP9 T3a + T3b.1)"
    );
}

fn build_bronze_http_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60 * 60))
        .build()
}

/// CLI ?듭뀡 (gold ?꾩슜).
struct GoldOpts {
    layer: LayerKind,
    output_dir: PathBuf,
    /// 사용자가 직접 준비한 `GeoJSON` 입력 (positional). `--bronze-prefix` 와 mutually exclusive.
    inputs: Vec<PathBuf>,
    /// R2 Bronze prefix (예: `bronze/2026-05/parcel-dtmk-30563/`). 지정 시 dtmk pipeline
    /// (R2 다운 + unzip + ogr2ogr) 가 `GeoJSON` 입력 자동 생성.
    bronze_prefix: Option<String>,
    /// dtmk pipeline 작업 디렉터리. 기본 `<output_dir>/../dtmk-work`.
    work_dir: Option<PathBuf>,
    /// dtmk pipeline 다운로드 동시성 (기본 8).
    concurrency: usize,
    /// 입력 SHP 의 source SRS (newtype — `EPSG:<digits>` 강제).
    source_srs: Srs,
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
async fn run_gold(args: Vec<String>) -> ExitCode {
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

#[derive(Debug, Clone)]
struct GoldUploadConfig {
    r2_cfg: crate::r2_upload::R2Config,
    version: Version,
}

async fn upload_gold_if_configured(
    opts: &GoldOpts,
    result: &BuildResult,
    bronze_inputs: Vec<BronzeInput>,
) -> ExitCode {
    let cfg = match load_config_or_exit() {
        Ok(cfg) => cfg,
        Err(code) => return code,
    };
    let Some(upload_cfg) = (match resolve_gold_upload_config(&cfg) {
        Ok(upload_cfg) => upload_cfg,
        Err(error) => return upload_config_error_exit(&error),
    }) else {
        log_local_only_gold_result();
        return ExitCode::SUCCESS;
    };
    let request = UploadGoldRequest {
        r2_cfg: &upload_cfg.r2_cfg,
        version: &upload_cfg.version,
        layer: opts.layer,
        build: result,
        bronze_inputs,
        source_srs: &opts.source_srs,
    };
    upload_result_exit(upload_gold_to_r2(request).await)
}

fn resolve_gold_upload_config(cfg: &Config) -> Result<Option<GoldUploadConfig>, UploadStepError> {
    let Some(r2_cfg) = cfg.r2.clone() else {
        return Ok(None);
    };
    let version = resolve_upload_version(cfg)?;
    Ok(Some(GoldUploadConfig { r2_cfg, version }))
}

fn upload_config_error_exit(error: &UploadStepError) -> ExitCode {
    error!(error = %error, "GOLD_VERSION resolution failed");
    ExitCode::from(2)
}

fn upload_result_exit(result: Result<(), UploadStepError>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            error!(error = %error, "R2 upload failed");
            ExitCode::FAILURE
        }
    }
}

fn log_local_only_gold_result() {
    info!("R2_* env not set ??local-only mode (build artifact ready at flat_tiles_dir)");
}

fn dtmk_work_dir(opts: &GoldOpts) -> PathBuf {
    opts.work_dir.clone().unwrap_or_else(|| {
        opts.output_dir
            .parent()
            .map_or_else(|| PathBuf::from("./var/dtmk-work"), |p| p.join("dtmk-work"))
    })
}

fn bronze_inputs_from_archives(archives: &[SigunguArchive]) -> Vec<BronzeInput> {
    archives
        .iter()
        .map(|a| BronzeInput {
            r2_key: a.r2_key.clone(),
            bytes: a.zip_bytes,
            etag: None,
            sha256: a.sha256.clone(),
        })
        .collect()
}

fn target_web_srs() -> Result<Srs, PrepareError> {
    Srs::new(sp9_base_layer_config::TARGET_SRS_WEB)
        .map_err(|e| PrepareError::Config(format!("TARGET_SRS_WEB invalid: {e}")))
}

async fn prepare_dtmk_inputs(
    host: Host,
    opts: &GoldOpts,
    bronze_prefix: &str,
) -> Result<(Vec<PathBuf>, Vec<BronzeInput>), PrepareError> {
    let cfg = Config::from_env()?;
    let uploader = dtmk_uploader_from_config(&cfg)?;
    let work_dir = dtmk_work_dir(opts);
    info!(work_dir = %work_dir.display(), "dtmk work dir");

    let fetched = fetch_dtmk_inputs(&uploader, bronze_prefix, &work_dir, opts.concurrency).await?;
    let bronze_inputs = bronze_inputs_from_archives(&fetched.archives);

    ensure_ogr2ogr_available(host).await?;
    let geojson_dir = ensure_geojson_dir(&work_dir).await?;
    let geojsons = convert_archives_to_geojsons(host, fetched.archives, &geojson_dir, opts).await?;
    log_conversion_complete(geojsons.len(), bronze_inputs.len());
    Ok((geojsons, bronze_inputs))
}

fn dtmk_uploader_from_config(cfg: &Config) -> Result<R2Uploader, PrepareError> {
    let r2_cfg = cfg.r2.clone().ok_or(PrepareError::R2NotConfigured)?;
    Ok(R2Uploader::new(r2_cfg))
}

async fn fetch_dtmk_inputs(
    uploader: &R2Uploader,
    bronze_prefix: &str,
    work_dir: &std::path::Path,
    concurrency: usize,
) -> Result<DtmkFetched, PrepareError> {
    let fetched = dtmk::fetch(
        uploader,
        &DtmkFetchArgs {
            prefix: bronze_prefix,
            work_dir,
            concurrency,
        },
    )
    .await?;
    info!(
        archives = fetched.archives.len(),
        downloaded = fetched.newly_downloaded,
        extracted = fetched.newly_extracted,
        "dtmk fetch done"
    );
    Ok(fetched)
}

async fn ensure_ogr2ogr_available(host: Host) -> Result<(), PrepareError> {
    let version = shp_to_geojson::check_available(host)
        .await
        .map_err(|e| PrepareError::Ogr2OgrUnavailable(format!("{e}")))?;
    info!(version = %version, "ogr2ogr available");
    Ok(())
}

async fn ensure_geojson_dir(work_dir: &std::path::Path) -> Result<PathBuf, PrepareError> {
    let geojson_dir = work_dir.join("geojson");
    tokio::fs::create_dir_all(&geojson_dir)
        .await
        .map_err(|source| PrepareError::Io {
            path: geojson_dir.display().to_string(),
            source,
        })?;
    Ok(geojson_dir)
}

type ConversionTask = tokio::task::JoinHandle<Result<PathBuf, String>>;

struct ArchiveConversion {
    host: Host,
    archive: SigunguArchive,
    geojson_dir: PathBuf,
    source_srs: Srs,
    target_srs: Srs,
}

async fn convert_archives_to_geojsons(
    host: Host,
    archives: Vec<SigunguArchive>,
    geojson_dir: &std::path::Path,
    opts: &GoldOpts,
) -> Result<Vec<PathBuf>, PrepareError> {
    let tasks = spawn_conversion_tasks(host, archives, geojson_dir, opts).await?;
    collect_conversion_tasks(tasks).await
}

async fn spawn_conversion_tasks(
    host: Host,
    archives: Vec<SigunguArchive>,
    geojson_dir: &std::path::Path,
    opts: &GoldOpts,
) -> Result<Vec<ConversionTask>, PrepareError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(opts.concurrency.max(1)));
    let target_srs = target_web_srs()?;
    let mut tasks = Vec::with_capacity(archives.len());
    for archive in archives {
        let conversion = ArchiveConversion {
            host,
            archive,
            geojson_dir: geojson_dir.to_path_buf(),
            source_srs: opts.source_srs.clone(),
            target_srs: target_srs.clone(),
        };
        tasks.push(spawn_conversion_task(Arc::clone(&semaphore), conversion).await?);
    }
    Ok(tasks)
}

async fn spawn_conversion_task(
    semaphore: Arc<tokio::sync::Semaphore>,
    conversion: ArchiveConversion,
) -> Result<ConversionTask, PrepareError> {
    let permit = semaphore.acquire_owned().await?;
    Ok(tokio::spawn(async move {
        convert_archive_to_geojson(permit, conversion).await
    }))
}

async fn convert_archive_to_geojson(
    permit: tokio::sync::OwnedSemaphorePermit,
    conversion: ArchiveConversion,
) -> Result<PathBuf, String> {
    let _permit = permit;
    let out = conversion
        .geojson_dir
        .join(format!("{}.geojson", archive_stem(&conversion.archive)));
    if geojson_already_ready(&out).await {
        return Ok(out);
    }
    let args = Ogr2OgrArgs {
        input_shp: &conversion.archive.shp_path,
        output_geojson: &out,
        source_srs: conversion.source_srs.as_str(),
        target_srs: conversion.target_srs.as_str(),
    };
    shp_to_geojson::run(conversion.host, &args)
        .await
        .map_err(|e| format!("{}: {e}", conversion.archive.shp_path.display()))?;
    Ok(out)
}

fn archive_stem(archive: &SigunguArchive) -> String {
    archive
        .shp_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("shp")
        .to_owned()
}

async fn geojson_already_ready(path: &std::path::Path) -> bool {
    matches!(tokio::fs::metadata(path).await, Ok(metadata) if metadata.len() > 0)
}

async fn collect_conversion_tasks(
    tasks: Vec<ConversionTask>,
) -> Result<Vec<PathBuf>, PrepareError> {
    let mut geojsons = Vec::with_capacity(tasks.len());
    for task in tasks {
        let path = task.await?.map_err(PrepareError::ShpConversion)?;
        geojsons.push(path);
    }
    geojsons.sort();
    Ok(geojsons)
}

fn log_conversion_complete(geojson_count: usize, bronze_input_count: usize) {
    info!(
        geojson_count = geojson_count,
        bronze_inputs = bronze_input_count,
        "ogr2ogr conversion complete; L10 lineage captured"
    );
}

fn run_promote_cli(_args: &[String]) -> ExitCode {
    error!("{}", manifest_write_path_disabled_message("promote"));
    ExitCode::from(2)
}

/// Legacy manifest backup cleanup subcommand (ADR 0028 + runbook § 6).
///
/// Consumer-only handover 이후 항상 실패한다. manifest backup lifecycle 은
/// platform-core Catalog 가 담당한다.
fn run_cleanup_backups_cli(_args: &[String]) -> ExitCode {
    error!(
        "{}",
        manifest_write_path_disabled_message("cleanup-manifest-backups")
    );
    ExitCode::from(2)
}

/// L2 Verification — *default-on*, SSOT landmarks 자동 검증.
///
/// SSOT: `sp9_base_layer_config::VERIFY_LANDMARKS` 의 모든 (pnu, lat, lon) 가 maxzoom
/// tile 에 등장해야 함. parcels layer 만 적용 (admin/complex 는 PNU 컬럼 없음).
///
/// 추가 env override:
/// - `VERIFY_MIN_BYTES` — 파일 size 최소. 미설정 시 SSOT default
///   (`NATIONWIDE_PMTILES_MIN_BYTES`) 적용 — 항상 강제 (silent build fail 감지).
/// - `VERIFY_DISABLE=1` — dev / micro-fixture 빌드 명시적 disable. CI 에선 절대 set 금지.
// audit 2026-05-08: cognitive complexity 26/15. layer-별 verification + size + sample
// PNU 검증 직선 흐름. 분해 시 verification 흐름 흩어져 가독성 ↓.
fn tile_specs_for_layer(layer: LayerKind) -> Result<Vec<TileSpec>, VerifyStepError> {
    if !matches!(layer, LayerKind::Parcels) {
        return Ok(Vec::new());
    }

    let max_z = layer.zoom_range().1;
    sp9_base_layer_config::VERIFY_LANDMARKS
        .iter()
        .map(|landmark| tile_spec_for_landmark(landmark, max_z))
        .collect()
}

fn tile_spec_for_landmark(
    landmark: &sp9_base_layer_config::VerifyLandmark,
    max_z: u8,
) -> Result<TileSpec, VerifyStepError> {
    let (x, y) = lonlat_to_tile(landmark.lon, landmark.lat, max_z).map_err(|e| {
        log_landmark_tile_error(landmark, &e);
        VerifyStepError::TileCoord(e)
    })?;
    log_landmark_scheduled(landmark, max_z, x, y);
    Ok(TileSpec {
        z: max_z,
        x,
        y,
        expectations: vec![TileExpectation::PropertyEquals {
            key: "pnu".to_owned(),
            value: landmark.pnu.to_owned(),
        }],
    })
}

fn log_landmark_tile_error(
    landmark: &sp9_base_layer_config::VerifyLandmark,
    error: &TileCoordError,
) {
    error!(error = %error, landmark = landmark.label, "invalid landmark tile coordinates");
}

fn log_landmark_scheduled(landmark: &sp9_base_layer_config::VerifyLandmark, z: u8, x: u32, y: u32) {
    info!(
        landmark = landmark.label,
        pnu = landmark.pnu,
        tile = format!("{z}/{x}/{y}"),
        "verify landmark scheduled (JSON property check)",
    );
}

fn verify_disabled() -> bool {
    std::env::var("VERIFY_DISABLE").ok().as_deref() == Some("1")
}

fn verify_min_file_bytes() -> u64 {
    std::env::var("VERIFY_MIN_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(sp9_base_layer_config::NATIONWIDE_PMTILES_MIN_BYTES)
}

async fn run_verify(
    host: Host,
    build: &BuildResult,
    layer: LayerKind,
) -> Result<(), VerifyStepError> {
    if verify_disabled() {
        log_verify_disabled();
        return Ok(());
    }

    let min_bytes = verify_min_file_bytes();

    let tile_specs = tile_specs_for_layer(layer)?;
    let spec = VerifySpec {
        pmtiles: &build.output_path,
        layer_name: layer.layer_name(),
        min_file_bytes: min_bytes,
        tile_specs: &tile_specs,
    };
    let result = verify::run(host, &spec).await?;
    info!(
        sha256 = %result.sha256,
        file_bytes = result.file_bytes,
        tiles_passed = result.tiles_passed,
        "L2 verification passed",
    );
    Ok(())
}

fn log_verify_disabled() {
    warn!("VERIFY_DISABLE=1 ??verification skipped (dev / micro-fixture only)");
}

/// L3 Atomicity — flat tile R2 batch upload + staging spec 박제. **manifest 미발행**.
///
/// 본 함수는 `gold` subcommand 의 R2 단계. platform-core Catalog 가 모든 layer staging
/// spec 검증 후 manifest pointer 를 publish 한다.
///
/// Key 레이아웃:
/// - flat tile: `<gold_prefix>/<version>/<layer>/{z}/{x}/{y}.pbf` (immutable, 1년 cache).
/// - `TileJSON`: `<gold_prefix>/<version>/<layer>.json` (5분 cache — 비활성화 가능).
/// - staging spec: `<gold_prefix>/staging/<version>/<layer>.spec.json` (no-cache).
///
/// 매뉴얼 manifest 발행 안 함 — platform-core Catalog 책임.
// 6 args 는 본 함수의 책임 범위 (R2 cfg / version / layer / build / lineage 자료들).
// 분해해도 helper 가 더 어색 → 의도적 allow.
// audit 2026-05-08: cognitive complexity 33/15. R2 batch upload + 메타 + lineage 직선
// 흐름. 분해 시 atomicity 흐름 흩어져 risk. 별도 refactor 시 step 별 모듈 분리.
#[derive(Debug)]
struct UploadGoldRequest<'a> {
    r2_cfg: &'a crate::r2_upload::R2Config,
    version: &'a Version,
    layer: LayerKind,
    build: &'a BuildResult,
    bronze_inputs: Vec<BronzeInput>,
    source_srs: &'a Srs,
}

const LOCAL_GOLD_VERSION: &str = "v_local";

fn resolve_upload_version(cfg: &Config) -> Result<Version, UploadStepError> {
    cfg.gold_version.clone().map_or_else(
        || {
            Version::new(LOCAL_GOLD_VERSION).map_err(|error| {
                UploadStepError::DefaultGoldVersionInvalid {
                    raw: LOCAL_GOLD_VERSION.to_owned(),
                    detail: error.to_string(),
                }
            })
        },
        Ok,
    )
}

#[derive(Debug, Clone)]
struct LineageEnv {
    git_sha: String,
    build_environment: String,
    source_license: Option<String>,
    source_url: Option<String>,
    correlation_id: Option<String>,
}

impl LineageEnv {
    fn from_env() -> Self {
        Self {
            git_sha: std::env::var("GIT_SHA").unwrap_or_else(|_| "unknown".to_owned()),
            build_environment: std::env::var("ETL_ENVIRONMENT")
                .unwrap_or_else(|_| "dev".to_owned()),
            source_license: nonempty_env_var("ETL_SOURCE_LICENSE"),
            source_url: nonempty_env_var("ETL_SOURCE_URL"),
            correlation_id: nonempty_env_var("CORRELATION_ID")
                .or_else(|| nonempty_env_var("GITHUB_RUN_ID")),
        }
    }
}

fn build_artifact_spec(
    request: &UploadGoldRequest<'_>,
    pmtiles_sha256: String,
    env: LineageEnv,
) -> ArtifactSpec {
    ArtifactSpec {
        key_prefix: request
            .r2_cfg
            .gold_layer_prefix(request.version, request.layer.layer_name()),
        pmtiles_bytes: request.build.output_bytes,
        pmtiles_sha256,
        row_count: request.build.feature_count,
        flat_tile_count: request.build.flat_tile_count,
        flat_tiles_total_bytes: request.build.flat_tiles_total_bytes,
        lineage: BuildLineage {
            tippecanoe_version: sp9_base_layer_config::TIPPECANOE_VERSION.to_owned(),
            git_sha: env.git_sha,
            built_at: chrono::Utc::now(),
            bronze_inputs: request.bronze_inputs.clone(),
            source_srs: request.source_srs.as_str().to_owned(),
            layer_name: request.layer.layer_name().to_owned(),
            build_environment: env.build_environment,
            source_license: env.source_license,
            source_url: env.source_url,
            correlation_id: env.correlation_id,
        },
    }
}

async fn upload_gold_to_r2(request: UploadGoldRequest<'_>) -> Result<(), UploadStepError> {
    let uploader = R2Uploader::new(request.r2_cfg.clone());
    let key_prefix = gold_key_prefix(&request);
    info!(
        version = %request.version,
        layer = %request.layer.layer_name(),
        key_prefix = %key_prefix,
        "R2 batch upload start"
    );

    upload_flat_tiles(&uploader, &request, &key_prefix).await?;
    let spec = build_staging_artifact_spec(&request).await?;
    publish_staging_spec(&uploader, &request, &spec).await?;
    publish_tilejson(&uploader, &request).await?;
    Ok(())
}

fn gold_key_prefix(request: &UploadGoldRequest<'_>) -> String {
    request
        .r2_cfg
        .gold_layer_prefix(request.version, request.layer.layer_name())
}

async fn upload_flat_tiles(
    uploader: &R2Uploader,
    request: &UploadGoldRequest<'_>,
    key_prefix: &str,
) -> Result<(), UploadStepError> {
    let upload = uploader
        .put_directory(&request.build.flat_tiles_dir, key_prefix, 100)
        .await?;
    info!(
        uploaded = upload.uploaded,
        bytes = upload.total_bytes,
        "R2 batch upload done"
    );
    Ok(())
}

async fn build_staging_artifact_spec(
    request: &UploadGoldRequest<'_>,
) -> Result<ArtifactSpec, UploadStepError> {
    let sha256 = verify::compute_sha256(&request.build.output_path).await?;
    Ok(build_artifact_spec(request, sha256, LineageEnv::from_env()))
}

async fn publish_staging_spec(
    uploader: &R2Uploader,
    request: &UploadGoldRequest<'_>,
    spec: &ArtifactSpec,
) -> Result<(), UploadStepError> {
    promote::write_staging_spec(uploader, request.version, request.layer, spec).await?;
    info!("staging spec published ??platform-core Catalog owns manifest pointer flip");
    Ok(())
}

async fn publish_tilejson(
    uploader: &R2Uploader,
    request: &UploadGoldRequest<'_>,
) -> Result<(), UploadStepError> {
    let public_url_base =
        read_r2_public_base().map_err(|detail| UploadStepError::PublicUrlMissing { detail })?;
    let tilejson = build_tilejson(
        request.r2_cfg,
        request.version,
        request.layer,
        &public_url_base,
    );
    let tilejson_key = request
        .r2_cfg
        .tilejson_key(request.version, request.layer.layer_name());
    uploader
        .put_object_json(&tilejson_key, &tilejson, "public, max-age=300")
        .await?;
    info!(tilejson_key = %tilejson_key, "TileJSON published");
    Ok(())
}

fn build_tilejson(
    r2_cfg: &crate::r2_upload::R2Config,
    version: &Version,
    layer: LayerKind,
    public_base: &R2PublicBase,
) -> serde_json::Value {
    let tiles_url = r2_cfg.tiles_url_template(public_base, version, layer.layer_name());
    let (min_z, max_z) = layer.zoom_range();
    serde_json::json!({
        "tilejson": "3.0.0",
        "name": layer.layer_name(),
        "description": format!("gongzzang gold {version} {}", layer.layer_name()),
        "tiles": [tiles_url],
        "minzoom": min_z,
        "maxzoom": max_z,
        "vector_layers": [{
            "id": layer.layer_name(),
            "minzoom": min_z,
            "maxzoom": max_z,
            "fields": match layer {
                LayerKind::Parcels => serde_json::json!({ "pnu": "String" }),
                LayerKind::Admin | LayerKind::Complex => {
                    serde_json::json!({ "code": "String", "name": "String" })
                }
            }
        }]
    })
}

/// ADR 0029 + 0030 — `Config::from_env` 의 fail-fast 를 `ExitCode` 로 매핑. 모든
/// subcommand 의 진입점에서 사용 — typed `ConfigError` 들이 명시적 stderr + exit 2.
fn load_config_or_exit() -> Result<Config, ExitCode> {
    Config::from_env().map_err(|e| {
        log_config_error(&e);
        ExitCode::from(2)
    })
}

fn log_config_error(error: &ConfigError) {
    match error {
        ConfigError::Environment(parse_err) => log_environment_config_error(parse_err),
        ConfigError::InvalidGoldVersion { raw, detail } => log_invalid_gold_version(raw, detail),
        ConfigError::PartialR2Namespace {
            prefix,
            present,
            missing,
        } => log_partial_r2_namespace(prefix, present, missing),
    }
}

fn log_environment_config_error(error: &EnvironmentParseError) {
    error!(
        error = %error,
        "ETL_ENVIRONMENT required (ADR 0029) — set to one of: local | staging | production"
    );
}

fn log_invalid_gold_version(raw: &str, detail: &str) {
    error!(
        raw = %raw,
        detail = %detail,
        "GOLD_VERSION invalid (ADR 0035 typed err) — must match ^v[a-z0-9_-]+$"
    );
}

fn log_partial_r2_namespace(prefix: &str, present: &[String], missing: &[String]) {
    error!(
        prefix = %prefix,
        present = ?present,
        missing = ?missing,
        "R2 namespace credentials partial (ADR 0035) — atomic 4-of-4 required (credential mix 차단)"
    );
}

/// 환경변수가 set 되어 있고 trim 후 비어있지 않으면 `Some(value)`, 아니면 `None`.
/// Round 3 P1 — license / source URL / `correlation_id` 같은 *옵션 lineage* 항목용.
fn nonempty_env_var(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty())
}

/// `R2_PUBLIC_URL_BASE` env → [`R2PublicBase`] 검증된 newtype.
///
/// 미설정 / 빈 문자열 / scheme 위반 / host 부재 모두 fail-fast — placeholder URL 발행 0.
/// Codex Round 6 finding #7 — production 환경에서는 `http://` 거부 (TLS 강제).
/// dev / staging 은 localhost 등 http 허용.
fn read_r2_public_base() -> Result<R2PublicBase, String> {
    let raw = std::env::var("R2_PUBLIC_URL_BASE")
        .map_err(|_| "R2_PUBLIC_URL_BASE env is not set".to_owned())?;
    if raw.trim().is_empty() {
        return Err("R2_PUBLIC_URL_BASE is empty".to_owned());
    }
    let base = R2PublicBase::new(raw).map_err(|e| e.to_string())?;
    if sp9_base_layer_config::Environment::is_production_from_env()
        && base.as_str().to_ascii_lowercase().starts_with("http://")
    {
        return Err(
            "R2_PUBLIC_URL_BASE must use https:// in production (ADR 0035 + finding #7)".to_owned(),
        );
    }
    Ok(base)
}

fn init_tracing() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    // L4: prod 는 ETL_LOG_FORMAT=json (CloudWatch / Datadog 자동 파싱). dev = pretty (default).
    let json_mode = std::env::var("ETL_LOG_FORMAT").as_deref() == Ok("json");

    // sentry-tracing layer — error/warn level 의 tracing event 자동 Sentry breadcrumb 변환.
    // SENTRY_DSN 미설정 시 init_sentry 가 None 반환 → sentry::Hub 가 no-op → layer 도 무동작.
    let sentry_layer = sentry_tracing::layer().enable_span_attributes();

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(sentry_layer);
    if json_mode {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true).json())
            .init();
    } else {
        registry
            .with(tracing_subscriber::fmt::layer().with_target(true))
            .init();
    }
}

/// L4 — Sentry SDK init. `SENTRY_DSN` 미설정 시 no-op (silent disabled).
/// release / environment / `git_sha` 자동 박제 → Sentry UI 의 release tracking 활성.
fn init_sentry() -> Option<sentry::ClientInitGuard> {
    let dsn = std::env::var("SENTRY_DSN")
        .ok()
        .filter(|v| !v.trim().is_empty())?;
    let release = std::env::var("GIT_SHA").ok().map(Into::into);
    // ADR 0035 — `ETL_ENVIRONMENT` SSOT only. backward-compat `ETL_BUILD_ENV` 제거.
    let environment: std::borrow::Cow<'static, str> = std::env::var("ETL_ENVIRONMENT")
        .unwrap_or_else(|_| "dev".to_owned())
        .into();
    // Round 3 P1 — traces_sample_rate env-driven (이전에 0.0 hardcode → SLO 측정 불가).
    // ETL 월 1회 cron 이라 traces=1.0 도 비용 무관, 단 dev / CI smoke 는 0.0 default.
    // production workflow 가 `SENTRY_TRACES_SAMPLE_RATE=1.0` 명시 set.
    let traces_sample_rate: f32 = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0)
        .clamp(0.0, 1.0);
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release,
            environment: Some(environment),
            // 100% sampling — ETL 은 월 1회 cron 이라 비용 무관, 모든 에러 보고.
            sample_rate: 1.0,
            traces_sample_rate,
            ..Default::default()
        },
    ));

    // Round 3 P1 — correlation_id 를 Sentry global scope tag 로 박제. 모든 에러 / span
    // 이 본 ID 와 cross-reference 가능 (Sentry UI 의 search filter, log aggregator 등).
    if let Some(corr_id) =
        nonempty_env_var("CORRELATION_ID").or_else(|| nonempty_env_var("GITHUB_RUN_ID"))
    {
        sentry::configure_scope(|scope| {
            scope.set_tag("correlation_id", &corr_id);
        });
    }

    Some(guard)
}

fn manifest_write_path_disabled_message(action: &str) -> String {
    format!(
        "Gongzzang `{action}` is disabled: static vector tile manifest ownership moved to platform-core Catalog. Gongzzang is consumer only; read /catalog/v1/vector-tiles/manifest instead."
    )
}

#[cfg(test)]
mod platform_core_handover_tests {
    use super::manifest_write_path_disabled_message;

    #[test]
    fn manifest_write_path_points_to_platform_core_owner() {
        let message = manifest_write_path_disabled_message("promote");

        assert!(message.contains("platform-core Catalog"));
        assert!(message.contains("/catalog/v1/vector-tiles/manifest"));
        assert!(message.contains("consumer only"));
    }
}

#[cfg(test)]
mod etl_main_contract_tests {
    use super::*;
    use crate::bronze::dtmk::SigunguArchive;
    use crate::gold::verify::TileExpectation;

    #[test]
    fn cli_parser_preserves_platform_core_handover_subcommands() {
        let promote_args = vec!["promote".to_owned(), "--dry-run".to_owned()];
        let promote = parse_cli_command(&promote_args);
        assert!(matches!(promote, Ok(CliCommand::Promote(args)) if args == ["--dry-run"]));

        let cleanup_args = vec!["cleanup-manifest-backups".to_owned()];
        let cleanup = parse_cli_command(&cleanup_args);
        assert!(matches!(
            cleanup,
            Ok(CliCommand::CleanupManifestBackups(args)) if args.is_empty()
        ));
    }

    #[test]
    fn dtmk_work_dir_defaults_to_output_parent_sibling() -> Result<(), Box<dyn std::error::Error>> {
        let opts = GoldOpts {
            layer: LayerKind::Parcels,
            output_dir: PathBuf::from("var/gold/v_test"),
            inputs: Vec::new(),
            bronze_prefix: Some("bronze/2026-05/parcel-dtmk-30563/".to_owned()),
            work_dir: None,
            concurrency: 8,
            source_srs: Srs::new("EPSG:5186")?,
        };

        assert_eq!(dtmk_work_dir(&opts), PathBuf::from("var/gold/dtmk-work"));
        Ok(())
    }

    #[test]
    fn bronze_lineage_is_derived_from_fetched_archives() -> Result<(), Box<dyn std::error::Error>> {
        let archives = vec![SigunguArchive {
            r2_key: "bronze/2026-05/a.zip".to_owned(),
            zip_path: PathBuf::from("zips/a.zip"),
            shp_path: PathBuf::from("shp/a.shp"),
            zip_bytes: 42,
            sha256: "abc123".to_owned(),
        }];

        let inputs = bronze_inputs_from_archives(&archives);

        assert_eq!(inputs.len(), 1);
        let Some(input) = inputs.first() else {
            return Err("expected one lineage input".into());
        };
        assert_eq!(input.r2_key, "bronze/2026-05/a.zip");
        assert_eq!(input.bytes, 42);
        assert_eq!(input.etag, None);
        assert_eq!(input.sha256, "abc123");
        Ok(())
    }

    #[test]
    fn parcels_verification_plan_uses_landmark_tiles_at_max_zoom(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let specs = tile_specs_for_layer(LayerKind::Parcels)?;

        assert_eq!(specs.len(), sp9_base_layer_config::VERIFY_LANDMARKS.len());
        let Some(first) = specs.first() else {
            return Err("expected at least one landmark spec".into());
        };
        assert_eq!(first.z, LayerKind::Parcels.zoom_range().1);
        assert_eq!(first.expectations.len(), 1);
        assert!(matches!(
            &first.expectations[0],
            TileExpectation::PropertyEquals { key, value }
                if key == "pnu" && value == sp9_base_layer_config::VERIFY_LANDMARKS[0].pnu
        ));
        Ok(())
    }

    #[test]
    fn upload_metadata_uses_platform_core_staging_prefix_and_lineage(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let version = Version::new("v_test")?;
        let source_srs = Srs::new("EPSG:5186")?;
        let r2_cfg = crate::r2_upload::R2Config {
            account_id: "account".to_owned(),
            access_key: "access".to_owned(),
            secret_key: "secret".to_owned(),
            bucket: "bucket".to_owned(),
            bronze_prefix: "bronze".to_owned(),
            gold_prefix: "gold".to_owned(),
        };
        let build = BuildResult {
            output_path: PathBuf::from("var/gold/parcels.pmtiles"),
            output_bytes: 100,
            flat_tiles_dir: PathBuf::from("var/gold/parcels"),
            flat_tile_count: 7,
            flat_tiles_total_bytes: 256,
            feature_count: Some(3),
        };
        let env = LineageEnv {
            git_sha: "abc".to_owned(),
            build_environment: "production".to_owned(),
            source_license: Some("license".to_owned()),
            source_url: Some("https://source.example".to_owned()),
            correlation_id: Some("run-1".to_owned()),
        };

        let request = UploadGoldRequest {
            r2_cfg: &r2_cfg,
            version: &version,
            layer: LayerKind::Parcels,
            build: &build,
            bronze_inputs: Vec::new(),
            source_srs: &source_srs,
        };
        let spec = build_artifact_spec(&request, "sha256".to_owned(), env);

        assert_eq!(spec.key_prefix, "gold/v_test/parcels");
        assert_eq!(spec.pmtiles_bytes, 100);
        assert_eq!(spec.pmtiles_sha256, "sha256");
        assert_eq!(spec.row_count, Some(3));
        assert_eq!(spec.lineage.layer_name, "parcels");
        assert_eq!(spec.lineage.source_srs, "EPSG:5186");
        assert_eq!(spec.lineage.git_sha, "abc");
        assert_eq!(spec.lineage.correlation_id.as_deref(), Some("run-1"));
        Ok(())
    }
}
