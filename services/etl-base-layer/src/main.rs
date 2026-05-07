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

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::gold::build::{build_layer, BuildResult};
use crate::gold::manifest::{GoldArtifact, GoldManifest};
use crate::gold::spawn::Host;
use crate::gold::tippecanoe::{check_available, LayerKind};
use crate::r2_upload::R2Uploader;

#[tokio::main]
async fn main() -> ExitCode {
    // .env 자동 로드 (dev convenience). production 에서는 .env 미존재 → silent skip.
    let _ = dotenvy::dotenv();
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

    let result = match build_layer(host, &opts.output_dir, opts.layer, &input_refs).await {
        Ok(r) => r,
        Err(e) => {
            error!(error = %e, "gold build failed");
            return ExitCode::FAILURE;
        }
    };
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
        let version = cfg
            .gold_version
            .as_deref()
            .unwrap_or("v_local")
            .to_owned();
        match upload_gold_to_r2(&r2_cfg, &version, opts.layer, &result).await {
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

/// ADR 0021 — flat tile 디렉터리 + manifest 를 R2 publish.
///
/// Key 레이아웃: `<gold_prefix>/<version>/<layer>/{z}/{x}/{y}.pbf` + `<gold_prefix>/manifest.json`.
async fn upload_gold_to_r2(
    r2_cfg: &crate::r2_upload::R2Config,
    version: &str,
    layer: LayerKind,
    build: &BuildResult,
) -> Result<(), Box<dyn std::error::Error>> {
    let uploader = R2Uploader::new(r2_cfg.clone());
    let key_prefix = format!("{}/{}/{}", r2_cfg.gold_prefix, version, layer.layer_name());
    info!(version, layer = %layer.layer_name(), key_prefix = %key_prefix, "R2 batch upload start");

    let upload = uploader
        .put_directory(&build.flat_tiles_dir, &key_prefix, 100)
        .await?;
    info!(
        uploaded = upload.uploaded,
        bytes = upload.total_bytes,
        "R2 batch upload done"
    );

    // PMTiles file sha256 — manifest 의 row_count 검증 기준 (간단히 file size 만 박제, 후속에서 sha256).
    let pmtiles_bytes = tokio::fs::read(&build.output_path).await?;
    let sha256 = format!("{:x}", Sha256::digest(&pmtiles_bytes));

    let (tile_min_zoom, tile_max_zoom) = layer.zoom_range();
    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        layer.layer_name().to_owned(),
        GoldArtifact {
            key: key_prefix.clone(),
            source_layer: layer.layer_name().to_owned(),
            pmtiles_bytes: build.output_bytes,
            pmtiles_sha256: sha256,
            built_at: chrono::Utc::now(),
            row_count: 0, // tippecanoe 출력 metadata 의 feature 수 (후속 박제)
            flat_tile_count: build.flat_tile_count,
            flat_tiles_total_bytes: build.flat_tiles_total_bytes,
            tile_min_zoom,
            tile_max_zoom,
            render_min_zoom: layer.render_min_zoom(),
            render_max_zoom: layer.render_max_zoom(),
            cache_max_age_seconds: layer.cache_max_age_seconds(),
        },
    );

    // tiles_url_template 의 host 는 R2 public URL — 사용자가 dashboard 에서 활성한
    // r2.dev subdomain 또는 custom domain 에 따라 다름. 환경변수 R2_PUBLIC_URL_BASE 로
    // override 가능 (미설정 시 placeholder 박제 — 사용자가 manifest 직접 수정).
    let raw_base = std::env::var("R2_PUBLIC_URL_BASE")
        .unwrap_or_else(|_| "https://<r2-public-host>/".to_owned());
    let base = if raw_base.ends_with('/') {
        raw_base
    } else {
        let mut s = raw_base;
        s.push('/');
        s
    };
    // 'literal placeholder' (`{layer}`, `{z}`, `{x}`, `{y}`) 는 mapbox-gl 의 tile URL
    // template 표준 — Rust format!{} 와 충돌해서 push_str 으로 안전 concat.
    let mut tiles_url_template = String::with_capacity(128);
    tiles_url_template.push_str(&base);
    tiles_url_template.push_str(&r2_cfg.gold_prefix);
    tiles_url_template.push('/');
    tiles_url_template.push_str(version);
    // mapbox-gl tile URL template placeholders — clippy nursery 의 false positive 회피.
    #[allow(clippy::literal_string_with_formatting_args)]
    {
        tiles_url_template.push_str("/{layer}/{z}/{x}/{y}.pbf");
    }

    let manifest = GoldManifest::new(version.to_owned(), tiles_url_template, artifacts);
    let manifest_key = format!("{}/manifest.json", r2_cfg.gold_prefix);
    uploader
        .put_object_json(&manifest_key, &manifest, "no-cache, max-age=0")
        .await?;
    info!(manifest_key = %manifest_key, "manifest published");

    // ADR 0021 SSS 화 — Mapbox TileJSON spec publish (https://github.com/mapbox/tilejson-spec).
    // 프론트는 `addSource({ type: "vector", url: "...parcels.json" })` 한 줄 → mapbox-gl
    // 자동 fetch + minzoom/maxzoom/tiles 적용. 우리 manifest fetch 코드 0.
    let tilejson = build_tilejson(r2_cfg, version, layer);
    let tilejson_key = format!("{}/{}/{}.json", r2_cfg.gold_prefix, version, layer.layer_name());
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
) -> serde_json::Value {
    let public_base = std::env::var("R2_PUBLIC_URL_BASE")
        .unwrap_or_else(|_| "https://<r2-public-host>/".to_owned());
    let base = if public_base.ends_with('/') {
        public_base
    } else {
        format!("{public_base}/")
    };
    let tiles_url = format!(
        "{base}{prefix}/{version}/{layer_name}/{{z}}/{{x}}/{{y}}.pbf",
        prefix = r2_cfg.gold_prefix,
        layer_name = layer.layer_name(),
    );
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
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}
