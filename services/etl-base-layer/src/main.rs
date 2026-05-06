//! 공짱 `PMTiles` base layer ETL — Bronze SHP 다운로드 단계 (SP9 T3a).
//!
//! 실행:
//! ```sh
//! BRONZE_PARCEL_SHP_URL=https://www.data.go.kr/.../parcel.shp.zip \
//! BRONZE_DIR=./var/bronze \
//! cargo run -p etl-base-layer
//! ```
//!
//! 다음 단계 (T3b):
//! - SHP → `GeoJSON` 변환 (`ogr2ogr` spawn)
//! - `tippecanoe` spawn → `PMTiles` 생성
//! - R2 업로드 (Bronze + Gold)
//! - manifest hot-swap

#![forbid(unsafe_code)]
// main.rs: init failure panic 은 정답이라 expect/unwrap 허용.
#![allow(clippy::expect_used, clippy::unwrap_used)]
// FU 26 — etl-base-layer 는 일회성 batch CLI. circuit-breaker wrapping 은 T3b 에서
// retry 정책 함께 검토 (월 1회 cron 이라 외부 dependency 우선순위 낮음).
#![allow(clippy::disallowed_types)]

mod bronze;
mod config;
mod manifest;

use std::process::ExitCode;

use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use crate::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();

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
        "starting bronze fetch (SP9 T3a)"
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

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,etl_base_layer=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .init();
}
