use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use circuit_breaker::execute as breaker_execute;
use futures_util::stream::{self, StreamExt};
use tracing::{info, instrument, warn};
use walkdir::WalkDir;

use super::error::{breaker_to_upload, UploadError};
use super::uploader::{DirectoryUploadResult, R2Uploader};

impl R2Uploader {
    /// 파일을 R2 객체로 업로드. `content_type` 은 R2 가 그대로 보존 → 클라이언트 fetch 시 사용.
    ///
    /// T2 — `circuit-breaker` wrap. timeout/retry/open 정책은 [`Policy::r2_default`].
    ///
    /// # Errors
    ///
    /// 파일 read 실패 / `PutObject` 실패 / circuit open / max-retries / timeout.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn put_object_file(
        &self,
        key: &str,
        path: &Path,
        content_type: &str,
    ) -> Result<(), UploadError> {
        info!(
            r2_op = "PutObject",
            r2_bucket = %self.config.bucket,
            r2_key = %key,
            "uploading file → R2"
        );

        breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.put_object_file",
            || async {
                let body =
                    ByteStream::from_path(path)
                        .await
                        .map_err(|e| UploadError::ReadFile {
                            path: path.display().to_string(),
                            source: std::io::Error::other(e),
                        })?;
                self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(key)
                .body(body)
                .content_type(content_type)
                .cache_control("public, max-age=31536000")
                // L5 Security — R2 server-side encryption (AES256). R2 default 가 이미
                // encrypted at rest 지만 `x-amz-server-side-encryption: AES256` header 명시 →
                // audit 정합 (audit log 가 SSE 사용 박제 가능).
                .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
                .send()
                .await
                .map_err(|e| UploadError::PutObject {
                    key: key.to_owned(),
                    detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                })?;
                Ok::<(), UploadError>(())
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.put_object_file", e))?;

        info!("file uploaded");
        Ok(())
    }

    /// ADR 0021 — flat tile 디렉터리 batch upload.
    ///
    /// `<local_root>/<z>/<x>/<y>.pbf` 들을 walk → R2 의
    /// `<key_prefix>/<z>/<x>/<y>.pbf` 들로 *concurrent* `PutObject` (default 100).
    /// tippecanoe 의 .pbf 는 기본 gzip → `Content-Encoding: gzip` + immutable
    /// `Cache-Control` metadata 자동 부여 (Cloudflare CDN edge 가 그대로 헤더 전달).
    ///
    /// 본 메서드가 X9 의 production 측 진정한 batch — DAU 1000+ 가 R2 직결 fetch.
    ///
    /// # Errors
    ///
    /// 단일 파일 read / `PutObject` 실패 시 [`UploadError`] 첫 1개 반환. 나머지는
    /// concurrent 진행 상태에서 cancel.
    #[instrument(
        skip(self, local_root),
        fields(bucket = %self.config.bucket, prefix = %key_prefix, root = %local_root.display()),
    )]
    #[allow(clippy::too_many_lines)] // T2: breaker wrapping 추가로 100줄 초과 — 분해 시 stream pipeline 흐름 흩어짐.
    pub async fn put_directory(
        &self,
        local_root: &Path,
        key_prefix: &str,
        concurrency: usize,
    ) -> Result<DirectoryUploadResult, UploadError> {
        // P0 (Codex Round 3): `buffer_unordered(0)` 은 stream 영구 pending — 빌드는 통과하지만
        // 모든 PUT 이 stuck. fail-fast.
        if concurrency == 0 {
            return Err(UploadError::InvalidConcurrency);
        }
        let key_prefix = key_prefix.trim_end_matches('/').to_owned();
        // P0 (Codex Round 3): WalkDir 의 I/O 에러를 silent drop 하지 않음. 권한 / broken
        // symlink / readdir fail 모두 첫 에러에서 즉시 abort — partial upload 차단.
        let mut entries: Vec<(std::path::PathBuf, String)> = Vec::new();
        for entry_result in WalkDir::new(local_root) {
            let entry = entry_result.map_err(|e| UploadError::WalkDir {
                root: local_root.display().to_string(),
                detail: e.to_string(),
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|x| x.to_str()) != Some("pbf") {
                continue;
            }
            let abs = entry.path().to_path_buf();
            // strip_prefix 실패 = local_root 와 entry 의 drive/UNC mismatch — silent 절대
            // 경로 key 생성 trick 제거. 명시적 에러로 fail-fast.
            let rel = abs
                .strip_prefix(local_root)
                .map_err(|_| UploadError::StripPrefix {
                    path: abs.display().to_string(),
                    root: local_root.display().to_string(),
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let key = format!("{key_prefix}/{rel}");
            entries.push((abs, key));
        }

        info!(count = entries.len(), concurrency, "starting batch upload");

        let success = Arc::new(AtomicU64::new(0));
        let bytes = Arc::new(AtomicU64::new(0));
        let success_clone = Arc::clone(&success);
        let bytes_clone = Arc::clone(&bytes);

        let result = stream::iter(entries.into_iter())
            .map(move |(path, key)| {
                let client = self.client.clone();
                let bucket = self.config.bucket.clone();
                let success = Arc::clone(&success_clone);
                let bytes = Arc::clone(&bytes_clone);
                let breaker = Arc::clone(&self.breaker);
                let policy = self.policy;
                async move {
                    // T2 — 각 PUT 마다 breaker 통과. systemic 장애 시 stream 잔여 PUT 도 즉시 fail.
                    breaker_execute(&breaker, &policy, "r2.put_directory_item", || {
                        let client = client.clone();
                        let bucket = bucket.clone();
                        let path = path.clone();
                        let key = key.clone();
                        let success = Arc::clone(&success);
                        let bytes = Arc::clone(&bytes);
                        async move {
                            let body =
                                ByteStream::from_path(&path)
                                    .await
                                    .map_err(|e| UploadError::ReadFile {
                                        path: path.display().to_string(),
                                        source: std::io::Error::other(e),
                                    })?;
                            let len = tokio::fs::metadata(&path)
                                .await
                                .map(|m| m.len())
                                .unwrap_or(0);
                            client
                                .put_object()
                                .bucket(&bucket)
                                .key(&key)
                                .body(body)
                                .content_type("application/x-protobuf")
                                .content_encoding("gzip") // tippecanoe 출력은 default gzip
                                .cache_control("public, max-age=31536000, immutable")
                                .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
                                .send()
                                .await
                                .map_err(|e| UploadError::PutObject {
                                    key: key.clone(),
                                    detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                                })?;
                            success.fetch_add(1, Ordering::Relaxed);
                            bytes.fetch_add(len, Ordering::Relaxed);
                            Ok::<(), UploadError>(())
                        }
                    })
                    .await
                    .map_err(|e| breaker_to_upload("r2.put_directory_item", e))
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;

        let mut first_err: Option<UploadError> = None;
        for r in result {
            if let Err(e) = r {
                if first_err.is_none() {
                    first_err = Some(e);
                } else {
                    warn!("multiple upload errors — only first reported");
                }
            }
        }
        if let Some(e) = first_err {
            return Err(e);
        }

        let count = success.load(Ordering::Relaxed);
        let total_bytes = bytes.load(Ordering::Relaxed);
        info!(
            uploaded = count,
            bytes = total_bytes,
            "batch upload complete"
        );
        Ok(DirectoryUploadResult {
            uploaded: count,
            total_bytes,
        })
    }
}
