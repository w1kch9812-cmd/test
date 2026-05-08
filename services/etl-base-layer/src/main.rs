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
// FU 26 — etl-base-layer 는 일회성 batch CLI. circuit-breaker wrapping 은 T3b.2 에서
// retry 정책 함께 검토 (월 1회 cron 이라 외부 dependency 우선순위 낮음).
#![allow(clippy::disallowed_types)]

mod bronze;
mod config;
mod error;
mod gold;
mod manifest;
mod r2_upload;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::bronze::dtmk::{self, DtmkFetchArgs};
use crate::config::Config;
use crate::error::{PrepareError, UploadStepError, VerifyStepError};
use crate::gold::build::{build_layer, BuildResult};
use crate::gold::manifest::{BronzeInput, BuildLineage};
use crate::gold::promote::{self, ArtifactSpec, PromoteArgs};
use crate::gold::shp_to_geojson::{self, Ogr2OgrArgs};
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::{check_available, LayerKind};
use crate::gold::verify::{self, lonlat_to_tile, TileCoordError, TileExpectation, TileSpec, VerifySpec};
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
    #[allow(clippy::expect_used)]
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime build");
    runtime.block_on(async_main())
}

// audit 2026-05-08: cognitive complexity 28/15. CLI subcommand dispatch + env load
// 직선 흐름이라 분해 시 *흐름 추적 어려워짐*. 별도 refactor (file split) 시 module 분리.
#[allow(clippy::cognitive_complexity)]
async fn async_main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let subcommand = args.first().map_or("bronze", String::as_str);

    // L8 — graceful shutdown handler. Ctrl+C / SIGTERM 시 즉시 abort 보다 *current
    // step abort + cleanup* 이 더 좋지만 ETL 의 모든 step 이 atomic spec staging 이라
    // (L3) 즉시 abort 도 안전. 본 핸들러는 user-facing 메시지 + 130 (SIGINT) exit code.
    let task = match subcommand {
        "bronze" | "" => tokio::spawn(run_bronze()),
        "gold" => tokio::spawn(run_gold(args[1..].to_vec())),
        "promote" => tokio::spawn(run_promote_cli(args[1..].to_vec())),
        other => {
            error!(subcommand = %other, "unknown subcommand — use `bronze` | `gold` | `promote`");
            return ExitCode::from(2);
        }
    };

    tokio::select! {
        biased;
        result = task => {
            match result {
                Ok(code) => code,
                Err(e) => {
                    error!(error = %e, "task panicked or aborted");
                    ExitCode::FAILURE
                }
            }
        }
        () = shutdown_signal() => {
            warn!("shutdown signal received — aborting (L3 staging spec 가 prod 보호)");
            // 130 = bash convention for SIGINT (128 + 2).
            ExitCode::from(130)
        }
    }
}

/// L8 — Ctrl+C (Unix SIGINT) + Unix SIGTERM 양쪽 listen. Windows 는 `ctrl_c` 만.
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        let mut int = signal(SignalKind::interrupt()).expect("install SIGINT handler");
        tokio::select! {
            _ = term.recv() => {}
            _ = int.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
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
    /// 사용자가 직접 준비한 `GeoJSON` 입력 (positional). `--bronze-prefix` 와 mutually exclusive.
    inputs: Vec<PathBuf>,
    /// R2 Bronze prefix (예: `bronze/2026-05/parcel-dtmk-30563/`). 지정 시 dtmk pipeline
    /// (R2 다운 + unzip + ogr2ogr) 가 `GeoJSON` 입력 자동 생성.
    bronze_prefix: Option<String>,
    /// dtmk pipeline 작업 디렉터리. 기본 `<output_dir>/../dtmk-work`.
    work_dir: Option<PathBuf>,
    /// dtmk pipeline 다운로드 동시성 (기본 8).
    concurrency: usize,
    /// 입력 SHP 의 source SRS (V-World dtmk = `EPSG:5186`, 공공데이터포털 일부 = `EPSG:5179`).
    source_srs: String,
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
    let mut source_srs: String = sp9_base_layer_config::SOURCE_SRS_VWORLD.to_owned();
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
                source_srs.clone_from(v);
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
#[allow(clippy::cognitive_complexity)]
async fn run_gold(args: Vec<String>) -> ExitCode {
    let opts = match parse_gold_args(&args) {
        Ok(o) => o,
        Err(e) => {
            error!(error = %e, "gold args parse failed");
            return ExitCode::from(2);
        }
    };

    let host = Host::detect();
    info!(
        host = ?host,
        layer = %opts.layer.layer_name(),
        output_dir = %opts.output_dir.display(),
        positional_inputs = opts.inputs.len(),
        bronze_prefix = ?opts.bronze_prefix,
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

    // dtmk path: R2 → unzip → ogr2ogr → GeoJSON inputs. positional path 와 disjoint.
    // bronze_inputs 도 함께 return — L10 lineage 박제용.
    let (geojson_inputs, bronze_inputs): (Vec<PathBuf>, Vec<BronzeInput>) =
        if let Some(prefix) = opts.bronze_prefix.as_deref() {
            match prepare_dtmk_inputs(host, &opts, prefix).await {
                Ok(v) => v,
                Err(e) => {
                    error!(error = %e, "dtmk preparation failed");
                    return ExitCode::FAILURE;
                }
            }
        } else {
            (opts.inputs.clone(), Vec::new())
        };

    if geojson_inputs.is_empty() {
        error!("no GeoJSON inputs after preparation — aborting");
        return ExitCode::FAILURE;
    }

    // 입력 path → &Path borrow vec.
    let input_refs: Vec<&std::path::Path> = geojson_inputs.iter().map(PathBuf::as_path).collect();

    let result = match build_layer(host, &opts.output_dir, opts.layer, &input_refs).await {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "gold build failed");
            return ExitCode::FAILURE;
        }
    };

    // L2 Verification — env-driven invariant 검증.
    // VERIFY_GANGNAM_PNU=1168010100107370000 / VERIFY_MIN_BYTES=10485760 (10MB).
    // 미설정 시 skip (개발 / smoke 빌드 — CI 에서만 강제).
    if let Err(e) = run_verify(host, &result, opts.layer).await {
        error!(error = %e, "L2 verification failed");
        return ExitCode::FAILURE;
    }
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

    // ADR 0021 § ETL pipeline — R2 가 설정되어 있으면 flat tile + manifest publish.
    let cfg = Config::from_env();
    if let Some(r2_cfg) = cfg.r2 {
        let version = cfg.gold_version.as_deref().unwrap_or("v_local").to_owned();
        match upload_gold_to_r2(
            &r2_cfg,
            &version,
            opts.layer,
            &result,
            bronze_inputs,
            &opts.source_srs,
        )
        .await
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                error!(error = %e, "R2 upload failed");
                ExitCode::FAILURE
            }
        }
    } else {
        info!("R2_* env not set → local-only mode (build artifact ready at flat_tiles_dir)");
        ExitCode::SUCCESS
    }
}

/// dtmk pipeline — R2 Bronze prefix → 로컬 unzip → ogr2ogr → `GeoJSON` 모음.
///
/// 1. `R2Uploader` 생성 (Config 의 R2 자격이 *반드시* 설정돼 있어야 함).
/// 2. [`bronze::dtmk::fetch`] — 273 시군구 zip 다운 + unzip.
/// 3. 각 .shp → `<work_dir>/geojson/<stem>.geojson` (ogr2ogr, idempotent skip).
/// 4. `GeoJSON` path 들 반환 — `build_layer` 가 tippecanoe 입력으로 사용.
#[allow(clippy::cognitive_complexity)]
async fn prepare_dtmk_inputs(
    host: Host,
    opts: &GoldOpts,
    bronze_prefix: &str,
) -> Result<(Vec<PathBuf>, Vec<BronzeInput>), PrepareError> {
    let cfg = Config::from_env();
    let r2_cfg = cfg
        .r2
        .clone()
.ok_or(PrepareError::R2NotConfigured)?;
    let uploader = R2Uploader::new(r2_cfg);

    let work_dir = opts.work_dir.clone().unwrap_or_else(|| {
        opts.output_dir
            .parent()
            .map_or_else(|| PathBuf::from("./var/dtmk-work"), |p| p.join("dtmk-work"))
    });
    info!(work_dir = %work_dir.display(), "dtmk work dir");

    let fetched = dtmk::fetch(
        &uploader,
        &DtmkFetchArgs {
            prefix: bronze_prefix,
            work_dir: &work_dir,
            concurrency: opts.concurrency,
        },
    )
    .await?;
    info!(
        archives = fetched.archives.len(),
        downloaded = fetched.newly_downloaded,
        extracted = fetched.newly_extracted,
        "dtmk fetch done"
    );

    // L10 lineage — bronze 입력의 fingerprint 박제. dtmk fetch 결과의 *진짜 SHA-256*
    // (다운로드 시점 streaming 계산) 를 r2_key 별로 모아 BronzeInput 으로 변환.
    // ETag (R2 MD5) 는 추가 hint (cryptographic 강도 약함).
    let bronze_inputs: Vec<BronzeInput> = fetched
        .archives
        .iter()
        .map(|a| BronzeInput {
            r2_key: a.r2_key.clone(),
            bytes: a.zip_bytes,
            etag: None,
            sha256: a.sha256.clone(),
        })
        .collect();

    // ogr2ogr 사전 체크.
    match shp_to_geojson::check_available(host).await {
        Ok(v) => info!(version = %v, "ogr2ogr available"),
        Err(e) => return Err(PrepareError::Ogr2OgrUnavailable(format!("{e}"))),
    }

    let geojson_dir = work_dir.join("geojson");
    tokio::fs::create_dir_all(&geojson_dir).await.map_err(|source| PrepareError::Io {
        path: geojson_dir.display().to_string(),
        source,
    })?;

    // ogr2ogr 동시 — 시군구 별 1 spawn. 디스크 + CPU 부담 → concurrency cap.
    let mut tasks: Vec<tokio::task::JoinHandle<Result<PathBuf, String>>> = Vec::new();
    let semaphore = Arc::new(tokio::sync::Semaphore::new(opts.concurrency.max(1)));
    for arch in fetched.archives {
        let permit = Arc::clone(&semaphore).acquire_owned().await.map_err(|_| PrepareError::SemaphoreClosed)?;
        let geojson_dir = geojson_dir.clone();
        let source_srs = opts.source_srs.clone();
        let host_c = host;
        let task = tokio::spawn(async move {
            let _permit = permit; // hold until done.
            let stem = arch
                .shp_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("shp")
                .to_owned();
            let out = geojson_dir.join(format!("{stem}.geojson"));
            // idempotent skip — 출력 .geojson 가 이미 비어있지 않으면 재사용.
            let skip = matches!(tokio::fs::metadata(&out).await, Ok(m) if m.len() > 0);
            if skip {
                return Ok(out);
            }
            let args = Ogr2OgrArgs {
                input_shp: &arch.shp_path,
                output_geojson: &out,
                source_srs: &source_srs,
                target_srs: "EPSG:4326",
            };
            shp_to_geojson::run(host_c, &args)
                .await
                .map_err(|e| format!("{}: {e}", arch.shp_path.display()))?;
            Ok(out)
        });
        tasks.push(task);
    }

    let mut geojsons: Vec<PathBuf> = Vec::with_capacity(tasks.len());
    for t in tasks {
        let p = t.await?.map_err(PrepareError::ShpConversion)?;
        geojsons.push(p);
    }
    geojsons.sort();
    info!(
        geojson_count = geojsons.len(),
        bronze_inputs = bronze_inputs.len(),
        "ogr2ogr conversion complete; L10 lineage captured"
    );
    Ok((geojsons, bronze_inputs))
}

/// L3 Atomicity — `promote` subcommand. 모든 layer staging spec 검증 후 manifest atomic flip + CDN purge.
///
/// CLI: `etl-base-layer promote --version <ver>`. layer 목록은 SSOT 의 `Layer::ALL` 자동.
/// 환경변수: `R2_*` 자격 + `R2_PUBLIC_URL_BASE` (manifest URL 의 base) + (선택) `CLOUDFLARE_*`.
// audit 2026-05-08: cognitive complexity 61/15. CLI args parse + 검증 + atomic flip
// + CDN purge 직선 흐름. 분해 시 *atomicity 보장* 흐름 흩어져 risk. 별도 refactor.
#[allow(clippy::cognitive_complexity)]
async fn run_promote_cli(args: Vec<String>) -> ExitCode {
    let mut version: Option<String> = None;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-v" => {
                let Some(v) = iter.next() else {
                    error!("--version needs a value");
                    return ExitCode::from(2);
                };
                version = Some(v.clone());
            }
            other => {
                error!(arg = %other, "unknown promote arg");
                return ExitCode::from(2);
            }
        }
    }
    let Some(version) = version else {
        error!("--version is required");
        return ExitCode::from(2);
    };

    let cfg = Config::from_env();
    let Some(r2_cfg) = cfg.r2 else {
        error!("R2_* env not set — promote requires R2 access");
        return ExitCode::FAILURE;
    };
    let public_base = match std::env::var("R2_PUBLIC_URL_BASE") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            error!(
                "R2_PUBLIC_URL_BASE env required for promote (manifest tiles_url_template host)"
            );
            return ExitCode::FAILURE;
        }
    };
    let uploader = R2Uploader::new(r2_cfg);

    info!(version = %version, "promote start (atomic manifest flip)");
    match promote::run(
        &uploader,
        &PromoteArgs {
            version: &version,
            layers: LayerKind::ALL,
            public_url_base: &public_base,
        },
    )
    .await
    {
        Ok(result) => {
            info!(
                version = %result.current_version,
                manifest_key = %result.manifest_key,
                cdn_purged = ?result.cdn_purged,
                "promote complete — manifest atomic flip done"
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            error!(error = %e, "promote failed — prod manifest unchanged (degrade gracefully)");
            ExitCode::FAILURE
        }
    }
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
#[allow(clippy::cognitive_complexity)]
async fn run_verify(
    host: Host,
    build: &BuildResult,
    layer: LayerKind,
) -> Result<(), VerifyStepError> {
    if std::env::var("VERIFY_DISABLE").ok().as_deref() == Some("1") {
        warn!("VERIFY_DISABLE=1 — verification skipped (dev / micro-fixture only)");
        return Ok(());
    }

    let min_bytes: u64 = std::env::var("VERIFY_MIN_BYTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(sp9_base_layer_config::NATIONWIDE_PMTILES_MIN_BYTES);

    // parcels 만 PNU 검증. admin/complex 는 file size + sha256 만.
    let max_z = layer.zoom_range().1;
    let mut tile_specs: Vec<TileSpec> = Vec::new();
    if matches!(layer, LayerKind::Parcels) {
        for landmark in sp9_base_layer_config::VERIFY_LANDMARKS {
            let (gx, gy) = match lonlat_to_tile(landmark.lon, landmark.lat, max_z) {
                Ok(coords) => coords,
                Err(TileCoordError::ZoomTooLarge(z)) => {
                    error!(zoom = z, "SSOT landmark zoom out of range — check LayerKind::zoom_range");
                    return Err(VerifyStepError::TileCoord(
                        TileCoordError::ZoomTooLarge(z)
                    ));
                }
                Err(e) => {
                    error!(error = %e, landmark = landmark.label, "invalid landmark coordinates");
                    return Err(VerifyStepError::TileCoord(e));
                }
            };
            tile_specs.push(TileSpec {
                z: max_z,
                x: gx,
                y: gy,
                expectations: vec![TileExpectation::PropertyEquals {
                    key: "pnu".to_owned(),
                    value: landmark.pnu.to_owned(),
                }],
            });
            info!(
                landmark = landmark.label,
                pnu = landmark.pnu,
                tile = format!("{max_z}/{gx}/{gy}"),
                "verify landmark scheduled (JSON property check)",
            );
        }
    }

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

/// L3 Atomicity — flat tile R2 batch upload + staging spec 박제. **manifest 미발행**.
///
/// 본 함수는 `gold` subcommand 의 R2 단계. promote subcommand 가 모든 layer staging
/// spec 검증 후에만 manifest atomic publish (`gold/manifest.json`).
///
/// Key 레이아웃:
/// - flat tile: `<gold_prefix>/<version>/<layer>/{z}/{x}/{y}.pbf` (immutable, 1년 cache).
/// - `TileJSON`: `<gold_prefix>/<version>/<layer>.json` (5분 cache — 비활성화 가능).
/// - staging spec: `<gold_prefix>/staging/<version>/<layer>.spec.json` (no-cache).
///
/// 매뉴얼 manifest 발행 안 함 — promote 단계 책임.
// 6 args 는 본 함수의 책임 범위 (R2 cfg / version / layer / build / lineage 자료들).
// 분해해도 helper 가 더 어색 → 의도적 allow.
// audit 2026-05-08: cognitive complexity 33/15. R2 batch upload + 메타 + lineage 직선
// 흐름. 분해 시 atomicity 흐름 흩어져 risk. 별도 refactor 시 step 별 모듈 분리.
#[allow(clippy::too_many_arguments, clippy::cognitive_complexity)]
async fn upload_gold_to_r2(
    r2_cfg: &crate::r2_upload::R2Config,
    version: &str,
    layer: LayerKind,
    build: &BuildResult,
    bronze_inputs: Vec<BronzeInput>,
    source_srs: &str,
) -> Result<(), UploadStepError> {
    let uploader = R2Uploader::new(r2_cfg.clone());
    let key_prefix = r2_cfg.gold_layer_prefix(version, layer.layer_name());
    info!(version, layer = %layer.layer_name(), key_prefix = %key_prefix, "R2 batch upload start");

    let upload = uploader
        .put_directory(&build.flat_tiles_dir, &key_prefix, 100)
        .await?;
    info!(
        uploaded = upload.uploaded,
        bytes = upload.total_bytes,
        "R2 batch upload done"
    );

    // PMTiles file sha256 — streaming (큰 파일 메모리 적재 0).
    let sha256 = verify::compute_sha256(&build.output_path).await?;

    // L10 lineage — git SHA / build env / bronze inputs 박제.
    let lineage = BuildLineage {
        tippecanoe_version: sp9_base_layer_config::TIPPECANOE_VERSION.to_owned(),
        git_sha: std::env::var("GIT_SHA").unwrap_or_else(|_| "unknown".to_owned()),
        built_at: chrono::Utc::now(),
        bronze_inputs,
        source_srs: source_srs.to_owned(),
        layer_name: layer.layer_name().to_owned(),
        build_environment: std::env::var("ETL_BUILD_ENV").unwrap_or_else(|_| "dev".to_owned()),
    };

    let spec = ArtifactSpec {
        key_prefix: key_prefix.clone(),
        pmtiles_bytes: build.output_bytes,
        pmtiles_sha256: sha256,
        row_count: build.feature_count, // tippecanoe --metadata-json 에서 추출 (P0.3).
        flat_tile_count: build.flat_tile_count,
        flat_tiles_total_bytes: build.flat_tiles_total_bytes,
        lineage,
    };
    promote::write_staging_spec(&uploader, version, layer, &spec).await?;
    info!("staging spec published — promote subcommand 가 manifest atomic flip");

    // TileJSON 은 layer 단위 self-describe — promote 와 무관하게 layer 빌드 직후 publish OK
    // (URL 안 version 이 박혀있어 client 가 overwrite 안 됨).
    // P0.2: R2_PUBLIC_URL_BASE 미설정 시 fail-fast — placeholder URL 절대 발행 금지.
    let public_url_base = std::env::var("R2_PUBLIC_URL_BASE")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or(UploadStepError::PublicUrlMissing)?;
    let tilejson = build_tilejson(r2_cfg, version, layer, &public_url_base);
    let tilejson_key = r2_cfg.tilejson_key(version, layer.layer_name());
    uploader
        .put_object_json(&tilejson_key, &tilejson, "public, max-age=300")
        .await?;
    info!(tilejson_key = %tilejson_key, "TileJSON published");
    Ok(())
}

/// Mapbox `TileJSON` 3.0.0 spec 직렬화. layer 메타 (zoom range / tiles url / `vector_layers`).
fn build_tilejson(
    r2_cfg: &crate::r2_upload::R2Config,
    version: &str,
    layer: LayerKind,
    public_base: &str,
) -> serde_json::Value {
    let tiles_url = r2_cfg.tiles_url_template(public_base, version, layer.layer_name());
    let (min_z, max_z) = layer.zoom_range();
    serde_json::json!({
        "tilejson": "3.0.0",
        "name": layer.layer_name(),
        "description": format!("gongzzang gold v{version} {}", layer.layer_name()),
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
    let environment: std::borrow::Cow<'static, str> = std::env::var("ETL_BUILD_ENV")
        .unwrap_or_else(|_| "dev".to_owned())
        .into();
    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release,
            environment: Some(environment),
            // 100% sampling — ETL 은 월 1회 cron 이라 비용 무관, 모든 에러 보고.
            sample_rate: 1.0,
            traces_sample_rate: 0.0,
            ..Default::default()
        },
    ));
    Some(guard)
}
