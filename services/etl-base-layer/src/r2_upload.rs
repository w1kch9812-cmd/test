//! Cloudflare R2 업로드 — `aws-sdk-s3` (S3-호환) wrapper.
//!
//! R2 는 S3-호환 API 를 노출하므로 `aws-sdk-s3` 가 그대로 동작. 단, 엔드포인트는
//! `https://<account_id>.r2.cloudflarestorage.com` 형식 → [`R2Config::endpoint_url`].
//!
//! 책임:
//! - 파일 업로드 (`put_object_file`) — Bronze SHP archive / Gold `PMTiles`
//! - JSON 업로드 (`put_object_json`) — manifest / index 파일
//!
//! T3b.1 = R2 업로드 path 만. ogr2ogr / tippecanoe / verify 는 T3b.2.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use futures_util::stream::{self, StreamExt};
use thiserror::Error;
use tracing::{info, instrument, warn};
use walkdir::WalkDir;

/// R2 자격 증명 + 버킷 설정.
///
/// 환경변수에서 로드. `account_id` 는 R2 endpoint URL 구성에 사용.
#[derive(Debug, Clone)]
pub struct R2Config {
    /// Cloudflare account id — endpoint 구성 (`<id>.r2.cloudflarestorage.com`).
    pub account_id: String,
    /// R2 access key (S3-호환 access key id).
    pub access_key: String,
    /// R2 secret key (S3-호환 secret).
    pub secret_key: String,
    /// 대상 버킷 이름 (예: `gongzzang-static`).
    pub bucket: String,
    /// Bronze archive key prefix (예: `bronze`). 끝 `/` 제외.
    pub bronze_prefix: String,
    /// Gold artifact key prefix (예: `gold`). 끝 `/` 제외.
    /// T3b.1 에서는 미사용 — T3b.2 의 ogr2ogr/tippecanoe 출력 PUT key 에 사용.
    #[allow(dead_code)]
    pub gold_prefix: String,
}

impl R2Config {
    /// `https://<account_id>.r2.cloudflarestorage.com` URL 빌드.
    #[must_use]
    pub fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }
}

/// R2 업로드 / 다운로드 에러.
#[derive(Debug, Error)]
pub enum UploadError {
    /// 로컬 파일 읽기 실패.
    #[error("read file {path} failed: {source}")]
    ReadFile {
        /// 대상 파일 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// 로컬 파일 쓰기 실패 (download path).
    #[error("write file {path} failed: {source}")]
    WriteFile {
        /// 대상 파일 경로.
        path: String,
        /// 원인.
        #[source]
        source: std::io::Error,
    },
    /// S3 `PutObject` API 실패 (네트워크 / 권한 / 4xx / 5xx).
    ///
    /// `aws_sdk_s3::error::SdkError` 는 generic 에 `Box<dyn StdError>` 가 아니어서
    /// `#[source]` 로 직접 wrap 하기 까다로움 → `DisplayErrorContext` 로 string 화.
    /// 디버깅 시에는 `RUST_LOG=aws_smithy_http=debug` 로 raw 응답 확인.
    #[error("put_object {key} failed: {detail}")]
    PutObject {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify (`DisplayErrorContext`).
        detail: String,
    },
    /// S3 `GetObject` API 실패.
    #[error("get_object {key} failed: {detail}")]
    GetObject {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify.
        detail: String,
    },
    /// S3 `ListObjectsV2` API 실패.
    #[error("list_objects {prefix} failed: {detail}")]
    ListObjects {
        /// 대상 prefix.
        prefix: String,
        /// 원인 stringify.
        detail: String,
    },
    /// `GetObject` body stream 읽기 실패.
    #[error("body stream {key} failed: {detail}")]
    BodyStream {
        /// 대상 객체 key.
        key: String,
        /// 원인 stringify.
        detail: String,
    },
    /// JSON 직렬화 실패.
    #[error("json serialize failed: {0}")]
    JsonSerialize(#[from] serde_json::Error),
}

/// 디렉터리 batch upload 결과.
#[derive(Debug, Clone, Copy)]
pub struct DirectoryUploadResult {
    /// 성공 PUT 수.
    pub uploaded: u64,
    /// 총 bytes (`PutObject` body 합).
    pub total_bytes: u64,
}

/// `list_objects` 가 반환하는 단일 객체 메타.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteObject {
    /// 전체 key (prefix 포함).
    pub key: String,
    /// 객체 size (bytes). idempotent skip 비교에 사용.
    pub size: u64,
    /// HTTP `If-Match` 등에 쓸 `ETag` (있으면). R2 가 항상 보장하지는 않음 → `Option`.
    pub etag: Option<String>,
}

/// R2 업로더 — `aws-sdk-s3` Client 래퍼.
///
/// 한 번 생성하면 여러 객체 업로드에 재사용. Client 는 connection pool 을 내부 보유.
#[derive(Debug, Clone)]
pub struct R2Uploader {
    client: S3Client,
    config: R2Config,
}

impl R2Uploader {
    /// 명시 자격으로 새 uploader 생성.
    ///
    /// `behavior_version_latest` + `force_path_style(true)` 사용 — R2 가 virtual-host
    /// style 도 지원하지만 path-style 이 endpoint URL 패턴과 더 호환적.
    ///
    /// L8 build resilience — `RetryConfig` 가 standard mode (max 3 attempts, exponential
    /// backoff). 1M+ 객체 batch 의 transient 4xx/5xx (R2 rate limit / network blip) 자동 재시도.
    #[must_use]
    pub fn new(config: R2Config) -> Self {
        let creds = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "etl-base-layer-r2",
        );
        // R2 는 region 무시하지만 SigV4 가 필수로 요구 — `auto` 사용.
        // `WhenRequired` checksum 설정: R2 가 aws-sdk-s3 1.86 의 default
        // `STREAMING-UNSIGNED-PAYLOAD-TRAILER` (`aws-chunked` 인코딩) 와 호환 안 함 →
        // `SignatureDoesNotMatch` 에러. R2 측 docs 권장 설정.
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(config.endpoint_url())
            .credentials_provider(creds)
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            // L8: standard retry mode = 최대 3 시도, exponential backoff.
            // adaptive 도 가능하지만 R2 가 rate limit signal 안 함 → standard 권장.
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(5))
            .build();
        let client = S3Client::from_conf(s3_config);
        Self { client, config }
    }

    /// 외부 endpoint override — 테스트용 (wiremock 등 mock S3).
    ///
    /// `endpoint_override` 는 `R2Config::endpoint_url()` 보다 우선.
    /// `#[cfg(test)]` — production 빌드에 포함 안 됨.
    #[cfg(test)]
    #[must_use]
    pub fn with_endpoint_override(config: R2Config, endpoint_override: String) -> Self {
        let creds = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "etl-base-layer-r2-test",
        );
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(endpoint_override)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();
        let client = S3Client::from_conf(s3_config);
        Self { client, config }
    }

    /// 설정 (bucket / prefix) 접근자.
    #[must_use]
    pub const fn config(&self) -> &R2Config {
        &self.config
    }

    /// 파일을 R2 객체로 업로드. `content_type` 은 R2 가 그대로 보존 → 클라이언트 fetch 시 사용.
    ///
    /// # Errors
    ///
    /// 파일 read 실패 / `PutObject` 실패.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn put_object_file(
        &self,
        key: &str,
        path: &Path,
        content_type: &str,
    ) -> Result<(), UploadError> {
        let body = ByteStream::from_path(path)
            .await
            .map_err(|e| UploadError::ReadFile {
                path: path.display().to_string(),
                source: std::io::Error::other(e),
            })?;

        info!(
            r2_op = "PutObject",
            r2_bucket = %self.config.bucket,
            r2_key = %key,
            "uploading file → R2"
        );

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

        info!("file uploaded");
        Ok(())
    }

    /// ADR 0021 — flat tile 디렉터리 batch upload.
    ///
    /// `<local_root>/<z>/<x>/<y>.pbf` 들을 walk → R2 의
    /// `<key_prefix>/<z>/<x>/<y>.pbf` 들로 *concurrent* PutObject (default 100).
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
    pub async fn put_directory(
        &self,
        local_root: &Path,
        key_prefix: &str,
        concurrency: usize,
    ) -> Result<DirectoryUploadResult, UploadError> {
        let key_prefix = key_prefix.trim_end_matches('/').to_owned();
        let entries: Vec<(std::path::PathBuf, String)> = WalkDir::new(local_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("pbf"))
            .map(|e| {
                let abs = e.path().to_path_buf();
                let rel = abs
                    .strip_prefix(local_root)
                    .unwrap_or(&abs)
                    .to_string_lossy()
                    .replace('\\', "/");
                let key = format!("{key_prefix}/{rel}");
                (abs, key)
            })
            .collect();

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

    /// `ListObjectsV2` paginated — `prefix` 하위 모든 객체 메타 반환.
    ///
    /// R2 의 `ListObjectsV2` 는 default 1000 객체/page → continuation token 으로 loop.
    /// 273 시군구 SHP zip 가정 시 1 page 면 충분하지만 안전하게 pagination 구현.
    ///
    /// # Errors
    ///
    /// `ListObjectsV2` API 실패.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, prefix = %prefix))]
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<RemoteObject>, UploadError> {
        let mut all = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let mut req = self
                .client
                .list_objects_v2()
                .bucket(&self.config.bucket)
                .prefix(prefix);
            if let Some(token) = continuation.as_deref() {
                req = req.continuation_token(token);
            }
            let resp = req.send().await.map_err(|e| UploadError::ListObjects {
                prefix: prefix.to_owned(),
                detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
            })?;
            for obj in resp.contents() {
                if let Some(key) = obj.key() {
                    all.push(RemoteObject {
                        key: key.to_owned(),
                        size: u64::try_from(obj.size().unwrap_or(0)).unwrap_or(0),
                        etag: obj.e_tag().map(str::to_owned),
                    });
                }
            }
            if resp.is_truncated().unwrap_or(false) {
                continuation = resp.next_continuation_token().map(str::to_owned);
                if continuation.is_none() {
                    break;
                }
            } else {
                break;
            }
        }
        info!(count = all.len(), "list_objects complete");
        Ok(all)
    }

    /// `GetObject` → 로컬 파일 stream 저장. 메모리 적재 X (대용량 SHP zip 가정).
    ///
    /// 부모 디렉터리는 자동 생성. 출력 파일은 *덮어쓰기* (`fs::File::create`).
    /// idempotent skip 은 호출자가 사전 size 비교로 처리 (본 메서드는 항상 다운).
    ///
    /// # Errors
    ///
    /// `GetObject` API 실패 / body stream 실패 / 디스크 I/O 실패.
    #[instrument(
        skip(self, dest),
        fields(bucket = %self.config.bucket, key = %key, dest = %dest.display()),
    )]
    pub async fn download_to_file(&self, key: &str, dest: &Path) -> Result<u64, UploadError> {
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: parent.display().to_string(),
                    source,
                })?;
        }

        let resp = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| UploadError::GetObject {
                key: key.to_owned(),
                detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
            })?;

        let mut body = resp.body;
        let mut file =
            tokio::fs::File::create(dest)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: dest.display().to_string(),
                    source,
                })?;
        let mut total: u64 = 0;
        use tokio::io::AsyncWriteExt;
        // ByteStream impl Stream → futures_util::StreamExt::next.
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| UploadError::BodyStream {
                key: key.to_owned(),
                detail: format!("{e}"),
            })?;
            file.write_all(&chunk)
                .await
                .map_err(|source| UploadError::WriteFile {
                    path: dest.display().to_string(),
                    source,
                })?;
            total += chunk.len() as u64;
        }
        file.flush()
            .await
            .map_err(|source| UploadError::WriteFile {
                path: dest.display().to_string(),
                source,
            })?;
        info!(bytes = total, "download complete");
        Ok(total)
    }

    /// `GetObject` → 메모리 `Vec<u8>` 으로 collect (작은 객체 - manifest / spec - 가정).
    /// 큰 객체는 [`Self::download_to_file`] 사용.
    ///
    /// # Errors
    ///
    /// `GetObject` API / body stream 실패.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn get_object_bytes(&self, key: &str) -> Result<Vec<u8>, UploadError> {
        let resp = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| UploadError::GetObject {
                key: key.to_owned(),
                detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
            })?;
        let mut body = resp.body;
        let mut buf = Vec::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| UploadError::BodyStream {
                key: key.to_owned(),
                detail: format!("{e}"),
            })?;
            buf.extend_from_slice(&chunk);
        }
        Ok(buf)
    }

    /// JSON pretty-encoded 객체 업로드. `content_type=application/json`.
    ///
    /// manifest / index 류에 사용 — 작은 (~10KB) 페이로드 가정.
    ///
    /// `T: Sync` 는 `#[instrument]` 가 만드는 future 가 multi-thread runtime
    /// 에서도 안전하게 await 되도록 강제 (clippy `future_not_send`).
    ///
    /// # Errors
    ///
    /// JSON 직렬화 실패 / `PutObject` 실패.
    #[instrument(skip(self, value), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn put_object_json<T: serde::Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        cache_control: &str,
    ) -> Result<(), UploadError> {
        let json = serde_json::to_vec_pretty(value)?;
        let bytes_len = json.len();

        info!(
            r2_op = "PutObject",
            r2_bucket = %self.config.bucket,
            r2_key = %key,
            bytes = bytes_len,
            "uploading json → R2"
        );

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(ByteStream::from(json))
            .content_type("application/json")
            .cache_control(cache_control)
            .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
            .send()
            .await
            .map_err(|e| UploadError::PutObject {
                key: key.to_owned(),
                detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
            })?;

        info!("json uploaded");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use serde_json::json;
    use std::io::Write;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_config(bucket: &str) -> R2Config {
        R2Config {
            account_id: "fake-account".into(),
            access_key: "fake-access".into(),
            secret_key: "fake-secret".into(),
            bucket: bucket.into(),
            bronze_prefix: "bronze".into(),
            gold_prefix: "gold".into(),
        }
    }

    #[test]
    fn endpoint_url_uses_account_id() {
        let cfg = test_config("any");
        assert_eq!(
            cfg.endpoint_url(),
            "https://fake-account.r2.cloudflarestorage.com"
        );
    }

    #[tokio::test]
    async fn put_object_file_sends_body_and_headers() {
        let server = MockServer::start().await;

        // path-style: /bucket/key  (force_path_style = true)
        Mock::given(method("PUT"))
            .and(path("/test-bucket/bronze/2026-05/parcel.shp.zip"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"deadbeef\""))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(b"PK\x03\x04 fake zip body")
            .expect("write tmp");

        uploader
            .put_object_file(
                "bronze/2026-05/parcel.shp.zip",
                tmp.path(),
                "application/zip",
            )
            .await
            .expect("upload");

        // wiremock 의 `expect(1)` 가 drop 시 검증 → 통과하면 PUT 1회 받음.
    }

    #[tokio::test]
    async fn put_object_json_serializes_pretty() {
        let server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/test-bucket/gold/manifest.json"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

        let payload = json!({"current_version": "v1", "artifacts": []});
        uploader
            .put_object_json("gold/manifest.json", &payload, "no-cache, max-age=0")
            .await
            .expect("upload");
    }

    #[tokio::test]
    async fn put_object_file_propagates_5xx() {
        let server = MockServer::start().await;

        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(500).set_body_string(
                "<Error><Code>InternalError</Code><Message>oops</Message></Error>",
            ))
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(b"x").expect("write");

        let err = uploader
            .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
            .await
            .expect_err("should fail");
        assert!(matches!(err, UploadError::PutObject { .. }));
    }
}
