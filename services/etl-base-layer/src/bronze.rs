//! Bronze 단계 — 외부 SHP/GeoJSON 다운로드 + sha256 + 로컬 저장 + R2 archive 업로드.
//!
//! 흐름:
//! 1. [`Config::sources`] 순회
//! 2. [`download_one`] — HTTP GET stream → 로컬 파일 + sha256 동시 계산
//! 3. R2 활성 시 (`Config::r2 = Some`) — 로컬 파일 → R2 `<bronze_prefix>/<batch_label>/<filename>`
//! 4. [`SourceEntry`] 채워서 [`BronzeManifest`] 에 추가
//! 5. 전체 완료 후 manifest.json — 로컬 + R2 양쪽
//!
//! R2 미설정 시 (`Config::r2 = None`) → 로컬 전용 (T3a 호환). 검증 / dev 환경.

// FU 26 — etl-base-layer 는 일회성 batch CLI. circuit-breaker wrapping 은 T3b.2 에서
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
use crate::r2_upload::{R2Uploader, UploadError};

/// Bronze 단계 에러 — 다운로드 / 디스크 / R2 업로드 통합.
#[derive(Debug, Error)]
pub enum BronzeError {
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
    /// R2 업로드 실패.
    #[error("r2 upload failed: {0}")]
    Upload(#[from] UploadError),
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
) -> Result<(SourceEntry, PathBuf), BronzeError> {
    let dest_dir = bronze_dir.join(batch_label);
    fs::create_dir_all(&dest_dir)
        .await
        .map_err(|e| BronzeError::Io {
            path: dest_dir.display().to_string(),
            source: e,
        })?;
    let dest: PathBuf = dest_dir.join(source.filename);

    info!(dest = %dest.display(), "downloading bronze source");

    let resp = client
        .get(&source.url)
        .send()
        .await
        .map_err(|e| BronzeError::Http {
            url: source.url.clone(),
            source: e,
        })?;
    if !resp.status().is_success() {
        return Err(BronzeError::UnexpectedStatus {
            status: resp.status().as_u16(),
            url: source.url.clone(),
        });
    }

    let mut file = fs::File::create(&dest).await.map_err(|e| BronzeError::Io {
        path: dest.display().to_string(),
        source: e,
    })?;
    let mut hasher = Sha256::new();
    let mut bytes_total: u64 = 0;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| BronzeError::Http {
            url: source.url.clone(),
            source: e,
        })?;
        hasher.update(&chunk);
        bytes_total += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|e| BronzeError::Io {
            path: dest.display().to_string(),
            source: e,
        })?;
    }
    file.flush().await.map_err(|e| BronzeError::Io {
        path: dest.display().to_string(),
        source: e,
    })?;

    let sha256 = hex::encode(hasher.finalize());
    info!(bytes = bytes_total, sha256 = %sha256, "bronze download complete");

    let entry = SourceEntry {
        url: source.url.clone(),
        filename: source.filename.to_owned(),
        bytes: bytes_total,
        sha256,
        downloaded_at: Utc::now(),
    };
    Ok((entry, dest))
}

/// 모든 [`Config::sources`] 다운로드 → [`BronzeManifest`] 빌드 → 로컬 + R2 업로드.
///
/// R2 비활성 (`Config::r2 = None`) 이면 로컬만. 활성 시에는 각 다운로드 직후
/// `<bronze_prefix>/<batch_label>/<filename>` key 로 R2 PUT, 그리고 모든 source
/// 완료 후 `<bronze_prefix>/<batch_label>/manifest.json` 도 PUT.
///
/// # Errors
///
/// 한 source 라도 실패 → 그 즉시 반환 (이전 다운로드 파일 + R2 객체는 남김 —
/// 재시도 시 sha256 비교로 재사용 결정).
#[instrument(skip(client, config))]
pub async fn run_bronze(
    client: &reqwest::Client,
    config: &Config,
) -> Result<BronzeManifest, BronzeError> {
    let mut manifest = BronzeManifest::new(config.batch_label.clone());

    let r2_uploader = config.r2.as_ref().map(|cfg| R2Uploader::new(cfg.clone()));
    if r2_uploader.is_some() {
        info!("R2 upload mode active");
    } else {
        info!("R2 disabled — local-only mode (set R2_* env vars to enable)");
    }

    for source in &config.sources {
        let (entry, local_path) =
            download_one(client, &config.bronze_dir, &config.batch_label, source).await?;
        if let Some(uploader) = r2_uploader.as_ref() {
            let key = bronze_object_key(uploader, &config.batch_label, &entry.filename);
            let content_type = guess_content_type(&entry.filename);
            uploader
                .put_object_file(&key, &local_path, content_type)
                .await?;
        }
        manifest.insert(source.id.to_owned(), entry);
    }

    // 로컬 manifest.json 저장.
    let manifest_path = config
        .bronze_dir
        .join(&config.batch_label)
        .join("manifest.json");
    let json = manifest.to_pretty_json().map_err(|e| BronzeError::Io {
        path: manifest_path.display().to_string(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidData, e),
    })?;
    fs::write(&manifest_path, json)
        .await
        .map_err(|e| BronzeError::Io {
            path: manifest_path.display().to_string(),
            source: e,
        })?;
    info!(manifest = %manifest_path.display(), "bronze manifest written (local)");

    // R2 manifest 업로드.
    if let Some(uploader) = r2_uploader.as_ref() {
        let key = bronze_manifest_key(uploader, &config.batch_label);
        // manifest 는 자주 갱신되지 않지만 audit 추적용 — `no-cache` 로.
        uploader
            .put_object_json(&key, &manifest, "no-cache, max-age=0")
            .await?;
        info!(key = %key, "bronze manifest written (R2)");
    }

    Ok(manifest)
}

/// `<bronze_prefix>/<batch_label>/<filename>` key 생성.
fn bronze_object_key(uploader: &R2Uploader, batch_label: &str, filename: &str) -> String {
    format!(
        "{}/{}/{}",
        uploader.config().bronze_prefix,
        batch_label,
        filename
    )
}

/// `<bronze_prefix>/<batch_label>/manifest.json` key 생성.
fn bronze_manifest_key(uploader: &R2Uploader, batch_label: &str) -> String {
    format!(
        "{}/{}/manifest.json",
        uploader.config().bronze_prefix,
        batch_label
    )
}

/// 파일명 확장자 → R2 객체 `Content-Type`.
///
/// 클라이언트 fetch 시 그대로 노출됨 — 잘못 설정하면 브라우저가 download 강제될 수 있음.
/// 대소문자 무관 (`Path::extension` + `eq_ignore_ascii_case`).
fn guess_content_type(filename: &str) -> &'static str {
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("zip") => "application/zip",
        Some("geojson" | "json") => "application/json",
        // .pmtiles / 알 수 없는 확장자 모두 binary stream.
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use crate::r2_upload::R2Config;

    fn fake_uploader() -> R2Uploader {
        R2Uploader::with_endpoint_override(
            R2Config {
                account_id: "a".into(),
                access_key: "k".into(),
                secret_key: "s".into(),
                bucket: "b".into(),
                bronze_prefix: "bronze".into(),
                gold_prefix: "gold".into(),
            },
            "http://127.0.0.1:1".into(),
        )
    }

    #[test]
    fn bronze_object_key_matches_layout() {
        let u = fake_uploader();
        assert_eq!(
            bronze_object_key(&u, "2026-05", "parcel.shp.zip"),
            "bronze/2026-05/parcel.shp.zip"
        );
    }

    #[test]
    fn bronze_manifest_key_matches_layout() {
        let u = fake_uploader();
        assert_eq!(
            bronze_manifest_key(&u, "2026-05"),
            "bronze/2026-05/manifest.json"
        );
    }

    #[test]
    fn content_type_routing() {
        assert_eq!(guess_content_type("parcel.shp.zip"), "application/zip");
        assert_eq!(guess_content_type("complex.geojson"), "application/json");
        assert_eq!(guess_content_type("manifest.json"), "application/json");
        assert_eq!(
            guess_content_type("parcels.pmtiles"),
            "application/octet-stream"
        );
        assert_eq!(guess_content_type("unknown"), "application/octet-stream");
    }
}
