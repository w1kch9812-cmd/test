use std::process::ExitCode;

use tracing::{error, info};

use crate::bronze;
use crate::config::Config;
use crate::runtime::load_config_or_exit;

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

pub async fn run_bronze() -> ExitCode {
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
