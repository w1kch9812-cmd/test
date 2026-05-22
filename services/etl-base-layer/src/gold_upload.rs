use std::process::ExitCode;

use sp9_base_layer_config::{R2PublicBase, Srs, Version};
use tracing::{error, info};

use crate::config::Config;
use crate::error::UploadStepError;
use crate::gold::build::BuildResult;
use crate::gold::manifest::{BronzeInput, BuildLineage};
use crate::gold::promote::{self, ArtifactSpec};
use crate::gold::tippecanoe::LayerKind;
use crate::gold::verify;
use crate::gold_cli::GoldOpts;
use crate::r2_upload::R2Uploader;
use crate::runtime::{load_config_or_exit, nonempty_env_var, read_r2_public_base};

struct GoldUploadConfig {
    r2_cfg: crate::r2_upload::R2Config,
    version: Version,
}

pub async fn upload_gold_if_configured(
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
    info!("R2_* env not set — local-only mode (build artifact ready at flat_tiles_dir)");
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
