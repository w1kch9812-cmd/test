//! Bronze 단계 — 외부 SHP/GeoJSON 다운로드 + sha256 + 로컬 저장.
//!
//! T3a 는 *로컬 파일 시스템* 까지만. R2 업로드는 T3b 에서 추가.
//!
//! 흐름:
//! 1. [`Config::sources`] 순회
//! 2. [`download_one`] — HTTP GET stream → 로컬 파일 + sha256 동시 계산
//! 3. [`SourceEntry`] 채워서 [`BronzeManifest`] 에 추가

// FU 26 — etl-base-layer 는 일회성 batch CLI. circuit-breaker wrapping 은 T3b 에서
// retry 정책 함께 검토 (월 1회 cron, 외부 dependency 우선순위 낮음).
#![allow(clippy::disallowed_types)]

use std::path::{Path, PathBuf};

use chrono::Utc;
use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, instrument};

use crate::config::{BronzeSource, Config};
use crate::manifest::{BronzeManifest, SourceEntry};

/// 단일 소스 다운로드 결과.
#[derive(Debug, Error)]
pub enum DownloadError {
    /// HTTP 통신 / 응답 에러.
    #[error("http error for {url}: {source}")]
    Http {
        /// 다운로드 URL.
        url: String,
        /// 원인.
        #[source]
        source: reqwest::Error,
    },
    /// 디스크 I/O 에러.
    #[error("io error at {path}: {source}")]
    Io {
        /// 대상 파일 경로 (string 으로 — Display 호환).
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// HTTP 응답이 200 OK 가 아님.
    #[error("unexpected status {status} for {url}")]
    UnexpectedStatus {
        /// HTTP status.
        status: u16,
        /// 다운로드 URL.
        url: String,
    },
}

/// 단일 [`BronzeSource`] 다운로드. 진행 상황은 `info!` 로 로깅.
///
/// 결과 파일 = `{bronze_dir}/{batch_label}/{filename}`.
/// sha256 은 streaming 으로 동시 계산 (파일 전체 메모리 적재 안 함).
///
/// # Errors
///
/// HTTP 실패 / 디스크 I/O 실패 / 비-2xx 응답.
#[instrument(skip(client, source), fields(id = %source.id, url = %source.url))]
pub async fn download_one(
    client: &reqwest::Client,
    bronze_dir: &Path,
    batch_label: &str,
    source: &BronzeSource,
) -> Result<SourceEntry, DownloadError> {
    let dest_dir = bronze_dir.join(batch_label);
    fs::create_dir_all(&dest_dir)
        .await
        .map_err(|e| DownloadError::Io {
            path: dest_dir.display().to_string(),
            source: e,
        })?;
    let dest: PathBuf = dest_dir.join(source.filename);

    info!(dest = %dest.display(), "downloading bronze source");

    let resp = client
        .get(&source.url)
        .send()
        .await
        .map_err(|e| DownloadError::Http {
            url: source.url.clone(),
            source: e,
        })?;
    if !resp.status().is_success() {
        return Err(DownloadError::UnexpectedStatus {
            status: resp.status().as_u16(),
            url: source.url.clone(),
        });
    }

    let mut file = fs::File::create(&dest)
        .await
        .map_err(|e| DownloadError::Io {
            path: dest.display().to_string(),
            source: e,
        })?;
    let mut hasher = Sha256::new();
    let mut bytes_total: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| DownloadError::Http {
            url: source.url.clone(),
            source: e,
        })?;
        hasher.update(&chunk);
        bytes_total += chunk.len() as u64;
        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadError::Io {
                path: dest.display().to_string(),
                source: e,
            })?;
    }
    file.flush().await.map_err(|e| DownloadError::Io {
        path: dest.display().to_string(),
        source: e,
    })?;

    let sha256 = hex::encode(hasher.finalize());
    info!(bytes = bytes_total, sha256 = %sha256, "bronze download complete");

    Ok(SourceEntry {
        url: source.url.clone(),
        filename: source.filename.to_owned(),
        bytes: bytes_total,
        sha256,
        downloaded_at: Utc::now(),
    })
}

/// 모든 [`Config::sources`] 다운로드 → [`BronzeManifest`] 빌드 → 로컬 manifest.json 저장.
///
/// # Errors
///
/// 한 source 라도 실패 → 그 즉시 반환 (이전 다운로드 파일은 남김 — 재시도 시 sha256
/// 비교로 재사용 결정).
#[instrument(skip(client, config))]
pub async fn run_bronze(
    client: &reqwest::Client,
    config: &Config,
) -> Result<BronzeManifest, DownloadError> {
    let mut manifest = BronzeManifest::new(config.batch_label.clone());

    for source in &config.sources {
        let entry = download_one(client, &config.bronze_dir, &config.batch_label, source).await?;
        manifest.insert(source.id.to_owned(), entry);
    }

    // 로컬 manifest.json 저장
    let manifest_path = config
        .bronze_dir
        .join(&config.batch_label)
        .join("manifest.json");
    let json = manifest.to_pretty_json().map_err(|e| DownloadError::Io {
        path: manifest_path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;
    fs::write(&manifest_path, json)
        .await
        .map_err(|e| DownloadError::Io {
            path: manifest_path.display().to_string(),
            source: e,
        })?;
    info!(manifest = %manifest_path.display(), "bronze manifest written");

    Ok(manifest)
}
