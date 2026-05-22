use std::path::{Path, PathBuf};
use std::sync::Arc;

use sp9_base_layer_config::Srs;
use tracing::info;

use crate::bronze::dtmk::{self, DtmkFetchArgs, DtmkFetched, SigunguArchive};
use crate::config::Config;
use crate::error::PrepareError;
use crate::gold::manifest::BronzeInput;
use crate::gold::shp_to_geojson::{self, Ogr2OgrArgs};
use crate::gold::spawn::Host;
use crate::gold_cli::GoldOpts;
use crate::r2_upload::R2Uploader;

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

pub async fn prepare_dtmk_inputs(
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
    work_dir: &Path,
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

async fn ensure_geojson_dir(work_dir: &Path) -> Result<PathBuf, PrepareError> {
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
    geojson_dir: &Path,
    opts: &GoldOpts,
) -> Result<Vec<PathBuf>, PrepareError> {
    let tasks = spawn_conversion_tasks(host, archives, geojson_dir, opts).await?;
    collect_conversion_tasks(tasks).await
}

async fn spawn_conversion_tasks(
    host: Host,
    archives: Vec<SigunguArchive>,
    geojson_dir: &Path,
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

async fn geojson_already_ready(path: &Path) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gold::tippecanoe::LayerKind;

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
}
