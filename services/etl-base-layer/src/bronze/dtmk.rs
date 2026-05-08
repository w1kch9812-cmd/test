//! V-World **dtmk** Bronze 자산을 ETL 입력으로 *재소비* — R2 list → 다운 → unzip.
//!
//! ## 배경
//!
//! [ADR 0022](../../../../../../docs/adr/0022-bronze-scraping-isolated-python-service.md)
//! 에 따라 Python `services/scraper-py/dtmk_vworld.py` 가 V-World 사이트 로그인 +
//! 273 시군구 *연속지적도* SHP zip (`LSMD_CONT_LDREG_<sigungu>.zip`) 을 R2 의
//! `<bronze_prefix>/<batch>/parcel-dtmk-<dsId>/` 아래에 적재. 본 모듈은 그
//! Bronze 자산을 ETL 측에서 *재소비* (Rust 가 R2 → 로컬 → unzip → ogr2ogr 입력).
//!
//! Rust 가 Python 을 *spawn 하지 않는* 이유는 [ADR 0025](../../../../../../docs/adr/0025-bronze-scraping-workflow-orchestrator-not-rust-spawn.md)
//! 박제. GitHub Actions workflow 가 Phase 1 (Python bronze) → Phase 2 (Rust gold) →
//! Phase 3 (Rust promote) 로 split — 본 모듈은 Phase 2 의 Bronze 재소비 단계.
//!
//! ## 흐름
//!
//! 1. R2 [`R2Uploader::list_objects`] — `<prefix>/*.zip` 목록
//! 2. 각 zip → `<work_dir>/zips/<filename>` 다운 (idempotent: 같은 size 면 skip)
//! 3. 각 zip → `<work_dir>/extracted/<stem>/` 안의 .shp/.dbf/.shx/.prj/.cpg 파일들
//!    (`tokio::task::spawn_blocking` — `zip` crate 가 sync I/O)
//! 4. [`DtmkFetched`] — 각 시군구의 .shp 절대 경로 모음 + 검증 메타
//!
//! ## 동시성
//!
//! 다운로드는 `concurrency` (default 8) 만큼 동시. R2 가 GET 요청 limit 이 매우
//! 관대하지만 (10k/s+) 로컬 디스크 throughput 이 병목 → 8 정도가 sweet spot.
//! Unzip 은 `spawn_blocking` 풀 (default tokio worker count) 에서 자연 직렬화.
//!
//! ## 멱등성
//!
//! - 다운: 같은 key + 같은 size 의 로컬 파일이 있으면 skip.
//! - Unzip: 추출 dir 안에 .shp 가 이미 있으면 skip (재실행 시간 절약).
//! - 따라서 일부 다운/추출만 끝낸 상태에서 ETL 가 실패해도 재실행이 cheap.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::stream::{self, StreamExt};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::{info, instrument, warn};

use crate::r2_upload::{R2Uploader, UploadError};

/// dtmk Bronze fetch 단계 에러.
#[derive(Debug, Error)]
pub enum DtmkError {
    /// R2 list/get 실패 또는 로컬 I/O 실패.
    #[error("r2: {0}")]
    R2(#[from] UploadError),
    /// 로컬 디스크 I/O.
    #[error("io {path}: {source}")]
    Io {
        /// 대상 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// zip 압축 해제 실패.
    #[error("unzip {archive}: {detail}")]
    Unzip {
        /// 대상 zip 경로.
        archive: String,
        /// 원인 string.
        detail: String,
    },
    /// 압축 해제 결과에 .shp 파일이 없음 (zip 내용 비정상).
    #[error("no .shp inside {archive} after extraction")]
    MissingShp {
        /// 대상 zip 경로.
        archive: String,
    },
    /// `tokio::task::JoinError` (`spawn_blocking` 실패).
    #[error("blocking task join: {0}")]
    Join(#[from] tokio::task::JoinError),
    /// R2 prefix 에 .zip 객체가 0 개.
    #[error("empty prefix {prefix} — no .zip objects in R2")]
    EmptyPrefix {
        /// 검색한 prefix.
        prefix: String,
    },
}

/// 단일 시군구의 추출 결과.
#[derive(Debug, Clone)]
pub struct SigunguArchive {
    /// R2 key (예: `bronze/2026-05/parcel-dtmk-30563/LSMD_CONT_LDREG_충북_충주시.zip`).
    pub r2_key: String,
    /// 다운로드된 zip 의 로컬 경로 (`<work_dir>/zips/<filename>`). audit 추적 + re-extract 용도.
    #[allow(dead_code)]
    pub zip_path: PathBuf,
    /// zip 안에서 추출된 .shp 의 절대 경로. ogr2ogr 의 입력으로 사용.
    pub shp_path: PathBuf,
    /// zip 파일 size (bytes).
    pub zip_bytes: u64,
    /// L10 lineage — zip 파일 SHA-256 hex (다운로드 직후 streaming 계산).
    /// R2 `ETag` (MD5 / multipart 합성) 보다 강한 fingerprint — manifest 의 진짜 input fingerprint.
    pub sha256: String,
}

/// dtmk Bronze fetch 결과 — 다음 단계 (ogr2ogr) 가 소비할 입력 메타.
///
/// `prefix` / `total_zip_bytes` 는 audit log + Gold manifest lineage (T6) 박제 용도.
#[derive(Debug, Clone)]
pub struct DtmkFetched {
    /// R2 prefix (예: `bronze/2026-05/parcel-dtmk-30563/`).
    #[allow(dead_code)]
    pub prefix: String,
    /// 시군구 별 archive (`r2_key` ASCII 정렬 — 빌드 결정성).
    pub archives: Vec<SigunguArchive>,
    /// 다운된 zip 합계 bytes (skip 포함).
    #[allow(dead_code)]
    pub total_zip_bytes: u64,
    /// 새로 다운된 zip 수 (skip 제외).
    pub newly_downloaded: u64,
    /// 새로 추출된 zip 수 (skip 제외).
    pub newly_extracted: u64,
}

/// dtmk fetch 옵션.
#[derive(Debug, Clone)]
pub struct DtmkFetchArgs<'a> {
    /// R2 prefix (`bronze/<batch>/parcel-dtmk-<ds_id>/`). 끝 `/` 자동 정규화.
    pub prefix: &'a str,
    /// 로컬 작업 디렉터리. 하위에 `zips/` + `extracted/` 자동 생성.
    pub work_dir: &'a Path,
    /// 동시 다운로드 수 (default 권장 8).
    pub concurrency: usize,
}

/// R2 → 로컬 → unzip 파이프라인. 결과는 ogr2ogr 가 직접 소비 가능한 .shp 경로 모음.
///
/// # Errors
///
/// R2 API 실패 / 디스크 실패 / unzip 실패 / .shp 누락.
// list/download/extract 가 한 흐름 — 분해해도 helper signature 가 ergonomic 손해.
#[allow(clippy::too_many_lines)]
#[instrument(skip(uploader, args), fields(prefix = %args.prefix, work_dir = %args.work_dir.display()))]
pub async fn fetch(
    uploader: &R2Uploader,
    args: &DtmkFetchArgs<'_>,
) -> Result<DtmkFetched, DtmkError> {
    let prefix = args.prefix.trim_end_matches('/').to_owned();
    let zips_dir = args.work_dir.join("zips");
    let extracted_dir = args.work_dir.join("extracted");
    tokio::fs::create_dir_all(&zips_dir)
        .await
        .map_err(|source| DtmkError::Io {
            path: zips_dir.display().to_string(),
            source,
        })?;
    tokio::fs::create_dir_all(&extracted_dir)
        .await
        .map_err(|source| DtmkError::Io {
            path: extracted_dir.display().to_string(),
            source,
        })?;

    // 1. List R2 objects under prefix.
    let listed = uploader.list_objects(&format!("{prefix}/")).await?;
    let mut zip_objects: Vec<_> = listed
        .into_iter()
        .filter(|o| o.key.to_ascii_lowercase().ends_with(".zip"))
        .collect();
    zip_objects.sort_by(|a, b| a.key.cmp(&b.key));
    if zip_objects.is_empty() {
        return Err(DtmkError::EmptyPrefix { prefix });
    }
    info!(
        zip_count = zip_objects.len(),
        "R2 dtmk prefix listed — starting download"
    );

    // 2. Concurrent download (idempotent skip).
    let downloaded = Arc::new(AtomicU64::new(0));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let downloaded_c = Arc::clone(&downloaded);
    let total_bytes_c = Arc::clone(&total_bytes);

    let download_results: Vec<Result<(crate::r2_upload::RemoteObject, PathBuf), DtmkError>> =
        stream::iter(zip_objects.clone().into_iter())
            .map(move |obj| {
                let uploader = uploader.clone();
                let zips_dir = zips_dir.clone();
                let downloaded = Arc::clone(&downloaded_c);
                let total_bytes = Arc::clone(&total_bytes_c);
                async move {
                    let filename = obj.key.rsplit('/').next().unwrap_or(&obj.key).to_owned();
                    let dest = zips_dir.join(&filename);
                    let needs_download = match tokio::fs::metadata(&dest).await {
                        Ok(meta) => meta.len() != obj.size,
                        Err(_) => true,
                    };
                    if needs_download {
                        let bytes = uploader.download_to_file(&obj.key, &dest).await?;
                        downloaded.fetch_add(1, Ordering::Relaxed);
                        total_bytes.fetch_add(bytes, Ordering::Relaxed);
                    } else {
                        total_bytes.fetch_add(obj.size, Ordering::Relaxed);
                    }
                    Ok::<_, DtmkError>((obj, dest))
                }
            })
            .buffer_unordered(args.concurrency.max(1))
            .collect()
            .await;

    let mut zip_pairs: Vec<(crate::r2_upload::RemoteObject, PathBuf)> = Vec::new();
    for r in download_results {
        zip_pairs.push(r?);
    }
    zip_pairs.sort_by(|a, b| a.0.key.cmp(&b.0.key));

    // L10 lineage — 각 zip 의 SHA-256 streaming 계산. ETag (R2 MD5) 대비 cryptographic
    // strong fingerprint. 273 시군구 × 50MB ≈ 14GB → sha256 500MB/s = ~30s 추가 (acceptable).
    let mut zip_shas: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (obj, path) in &zip_pairs {
        let sha = compute_file_sha256(path).await?;
        zip_shas.insert(obj.key.clone(), sha);
    }
    let zip_shas = Arc::new(zip_shas);

    info!(
        downloaded = downloaded.load(Ordering::Relaxed),
        total_bytes = total_bytes.load(Ordering::Relaxed),
        skipped = zip_pairs.len() as u64 - downloaded.load(Ordering::Relaxed),
        "downloads done",
    );

    // 3. Unzip (sync, in spawn_blocking).
    let extracted_count = Arc::new(AtomicU64::new(0));
    let extracted_count_c = Arc::clone(&extracted_count);
    let extract_results: Vec<Result<SigunguArchive, DtmkError>> =
        stream::iter(zip_pairs.into_iter())
            .map(move |(obj, zip_path)| {
                let extracted_dir = extracted_dir.clone();
                let extracted_count = Arc::clone(&extracted_count_c);
                let zip_shas = Arc::clone(&zip_shas);
                async move {
                    // <stem> = LSMD_CONT_LDREG_충북_충주시 (filename 에서 .zip 제외).
                    let stem = zip_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_owned();
                    let target = extracted_dir.join(&stem);
                    let needs_extract = !matches!(find_shp_in_dir(&target).await, Ok(Some(_)));
                    if needs_extract {
                        let zp = zip_path.clone();
                        let tg = target.clone();
                        tokio::task::spawn_blocking(move || extract_zip_sync(&zp, &tg)).await??;
                        extracted_count.fetch_add(1, Ordering::Relaxed);
                    }
                    let shp_path =
                        find_shp_in_dir(&target)
                            .await?
                            .ok_or_else(|| DtmkError::MissingShp {
                                archive: zip_path.display().to_string(),
                            })?;
                    let sha256 = zip_shas.get(&obj.key).cloned().unwrap_or_default();
                    Ok::<_, DtmkError>(SigunguArchive {
                        r2_key: obj.key,
                        zip_path,
                        shp_path,
                        zip_bytes: obj.size,
                        sha256,
                    })
                }
            })
            .buffer_unordered(args.concurrency.max(1))
            .collect()
            .await;

    let mut archives: Vec<SigunguArchive> = Vec::new();
    for r in extract_results {
        archives.push(r?);
    }
    archives.sort_by(|a, b| a.r2_key.cmp(&b.r2_key));

    let newly_downloaded = downloaded.load(Ordering::Relaxed);
    let newly_extracted = extracted_count.load(Ordering::Relaxed);
    info!(
        archives = archives.len(),
        newly_downloaded, newly_extracted, "dtmk fetch complete",
    );

    Ok(DtmkFetched {
        prefix,
        archives,
        total_zip_bytes: total_bytes.load(Ordering::Relaxed),
        newly_downloaded,
        newly_extracted,
    })
}

/// 파일 SHA-256 streaming (큰 zip 메모리 적재 0). L10 lineage fingerprint.
async fn compute_file_sha256(path: &Path) -> Result<String, DtmkError> {
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|source| DtmkError::Io {
            path: path.display().to_string(),
            source,
        })?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).await.map_err(|source| DtmkError::Io {
            path: path.display().to_string(),
            source,
        })?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// 디렉터리 walk 에서 첫 번째 `.shp` 절대 경로 반환. 디렉터리 없으면 `Ok(None)`.
async fn find_shp_in_dir(dir: &Path) -> Result<Option<PathBuf>, DtmkError> {
    let dir = dir.to_path_buf();
    let res = tokio::task::spawn_blocking(move || -> Result<Option<PathBuf>, std::io::Error> {
        if !dir.exists() {
            return Ok(None);
        }
        for entry in walkdir::WalkDir::new(&dir).into_iter().flatten() {
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_ascii_lowercase)
                    .as_deref()
                    == Some("shp")
            {
                return Ok(Some(entry.path().to_path_buf()));
            }
        }
        Ok(None)
    })
    .await?
    .map_err(|source| DtmkError::Io {
        path: "find_shp".into(),
        source,
    })?;
    Ok(res)
}

/// sync zip 추출 — `spawn_blocking` 안에서 호출. zip-slip path traversal 방어.
fn extract_zip_sync(zip_path: &Path, target_dir: &Path) -> Result<(), DtmkError> {
    std::fs::create_dir_all(target_dir).map_err(|source| DtmkError::Io {
        path: target_dir.display().to_string(),
        source,
    })?;
    let file = std::fs::File::open(zip_path).map_err(|source| DtmkError::Io {
        path: zip_path.display().to_string(),
        source,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| DtmkError::Unzip {
        archive: zip_path.display().to_string(),
        detail: format!("{e}"),
    })?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| DtmkError::Unzip {
            archive: zip_path.display().to_string(),
            detail: format!("entry {i}: {e}"),
        })?;
        // zip-slip 방어 — `enclosed_name` 이 null 이면 skip (악성 traversal path).
        let Some(rel) = entry.enclosed_name() else {
            warn!(
                archive = %zip_path.display(),
                name = entry.name(),
                "skipping suspicious entry (zip-slip)"
            );
            continue;
        };
        let out_path = target_dir.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|source| DtmkError::Io {
                path: out_path.display().to_string(),
                source,
            })?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|source| DtmkError::Io {
                    path: parent.display().to_string(),
                    source,
                })?;
            }
            let mut out = std::fs::File::create(&out_path).map_err(|source| DtmkError::Io {
                path: out_path.display().to_string(),
                source,
            })?;
            std::io::copy(&mut entry, &mut out).map_err(|source| DtmkError::Io {
                path: out_path.display().to_string(),
                source,
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    /// 작은 zip 을 만들어 추출 round-trip — `spawn_blocking` path 검증.
    #[tokio::test]
    async fn extract_zip_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("test.zip");
        let target = tmp.path().join("extracted");

        // 1. 작은 zip 작성 — `LSMD_CONT_LDREG_test.shp` 1개.
        {
            let f = std::fs::File::create(&zip_path).expect("create zip");
            let mut zw = zip::ZipWriter::new(f);
            zw.start_file("LSMD_CONT_LDREG_test.shp", SimpleFileOptions::default())
                .expect("start file");
            zw.write_all(b"FAKE_SHP_BYTES").expect("write shp");
            zw.finish().expect("finish zip");
        }

        // 2. 추출.
        let zp = zip_path.clone();
        let tg = target.clone();
        tokio::task::spawn_blocking(move || extract_zip_sync(&zp, &tg))
            .await
            .expect("join")
            .expect("extract");

        // 3. .shp 발견.
        let shp = find_shp_in_dir(&target).await.expect("find").expect("some");
        assert!(shp.ends_with("LSMD_CONT_LDREG_test.shp"));
        let bytes = std::fs::read(&shp).expect("read shp");
        assert_eq!(bytes, b"FAKE_SHP_BYTES");
    }

    /// 추출 dir 가 없을 때 `find_shp_in_dir` = `Ok(None)`.
    #[tokio::test]
    async fn find_shp_returns_none_when_dir_absent() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("nope");
        let res = find_shp_in_dir(&dir).await.expect("ok");
        assert!(res.is_none());
    }

    /// zip-slip — `../etc/passwd` 같은 path 는 skip 해야 함 (path traversal 방어).
    #[tokio::test]
    async fn extract_zip_rejects_path_traversal() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let zip_path = tmp.path().join("evil.zip");
        let target = tmp.path().join("extracted");

        // path traversal entry 가 포함된 zip — `enclosed_name` 이 None 반환할 것.
        {
            let f = std::fs::File::create(&zip_path).expect("create");
            let mut zw = zip::ZipWriter::new(f);
            // 일반 entry 1개 + traversal entry 1개.
            zw.start_file("good.shp", SimpleFileOptions::default())
                .expect("start good");
            zw.write_all(b"OK").expect("w good");
            zw.start_file("../escape.txt", SimpleFileOptions::default())
                .expect("start evil");
            zw.write_all(b"BAD").expect("w evil");
            zw.finish().expect("finish");
        }

        let zp = zip_path.clone();
        let tg = target.clone();
        tokio::task::spawn_blocking(move || extract_zip_sync(&zp, &tg))
            .await
            .expect("join")
            .expect("extract should succeed but skip the evil entry");

        // good.shp 가 추출됐는지.
        assert!(target.join("good.shp").exists());
        // escape.txt 가 target 의 *상위* (tempdir 루트) 에 *없어야* 함.
        assert!(!tmp.path().join("escape.txt").exists());
    }
}
