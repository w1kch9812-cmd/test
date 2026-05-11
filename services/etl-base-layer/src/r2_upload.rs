//! Cloudflare R2 업로드 — `aws-sdk-s3` (S3-호환) wrapper.
//!
//! R2 는 S3-호환 API 를 노출하므로 `aws-sdk-s3` 가 그대로 동작. 단, 엔드포인트는
//! `https://<account_id>.r2.cloudflarestorage.com` 형식 → [`R2Config::endpoint_url`].
//!
//! 책임:
//! - 파일 업로드 (`put_object_file`) — Bronze SHP archive / Gold `PMTiles`
//! - JSON 업로드 (`put_object_json`) — manifest / index 파일
//!
//! ## Circuit Breaker (T2 / Round 2)
//!
//! 모든 R2 호출 (`put_object_file` / `put_object_json` / `put_directory` / `list_objects`
//! / `try_get_object_bytes` / `download_to_file`) 은 [`circuit_breaker::execute`] 를 통과 —
//! [`Policy::r2_default`] 정책 (timeout 8s, max 1 retry, open after 5 fail in 10s, 60s cooldown).
//! `Breaker` 는 [`R2Uploader`] 안에 박제되어 모든 호출이 *동일* 상태 공유 — 시스템적
//! 장애 시 batch upload 가 즉시 중단되어 stream 의 나머지 PUT 도 빠르게 실패.

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
use circuit_breaker::{execute as breaker_execute, Breaker, BreakerError, Policy};
use futures_util::stream::{self, StreamExt};
use sp9_base_layer_config::{R2PublicBase, Version};
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

    /// Gold layer flat tile prefix: `<gold_prefix>/<version>/<layer>`.
    /// **SSOT** — 모든 gold key 생성이 이 helper 를 통해야 함.
    /// `version` 은 검증된 [`Version`] 만 받음 — 잘못된 라벨 생성 시점 차단.
    #[must_use]
    pub fn gold_layer_prefix(&self, version: &Version, layer_name: &str) -> String {
        format!("{}/{}/{}", self.gold_prefix, version, layer_name)
    }

    /// Gold layer `TileJSON` key: `<gold_prefix>/<version>/<layer>.json`.
    #[must_use]
    pub fn tilejson_key(&self, version: &Version, layer_name: &str) -> String {
        format!("{}/{}/{}.json", self.gold_prefix, version, layer_name)
    }

    /// Gold manifest key: `<gold_prefix>/manifest.json`.
    #[must_use]
    pub fn manifest_key(&self) -> String {
        format!("{}/manifest.json", self.gold_prefix)
    }

    /// Gold manifest backup key: `<gold_prefix>/manifest.<version>.json`.
    /// `version` 은 [`Version`] — 백업 키도 동일 검증 통과.
    #[must_use]
    pub fn manifest_backup_key(&self, version: &Version) -> String {
        format!("{}/manifest.{}.json", self.gold_prefix, version)
    }

    /// Gold staging spec key: `<gold_prefix>/staging/<version>/<layer>.spec.json`.
    #[must_use]
    pub fn staging_spec_key(&self, version: &Version, layer_name: &str) -> String {
        format!("{}/staging/{}/{}.spec.json", self.gold_prefix, version, layer_name)
    }

    /// Tiles URL template for `TileJSON` / manifest:
    /// `<public_base>/<gold_prefix>/<version>/<layer>/{z}/{x}/{y}.pbf`.
    /// `public_base` / `version` 모두 newtype — invalid scheme/host/format 시점 차단.
    #[must_use]
    pub fn tiles_url_template(
        &self,
        public_base: &R2PublicBase,
        version: &Version,
        layer_name: &str,
    ) -> String {
        let raw = public_base.as_str();
        let base = if raw.ends_with('/') {
            raw.to_owned()
        } else {
            format!("{raw}/")
        };
        #[allow(clippy::literal_string_with_formatting_args)]
        {
            format!("{base}{}/{}/{}/{{z}}/{{x}}/{{y}}.pbf", self.gold_prefix, version, layer_name)
        }
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
    /// `put_directory` 의 `concurrency` 인자가 0 — `buffer_unordered(0)` 은 stream 정지.
    /// 호출자 정책 위반이라 컴파일 단계 차단보다 runtime fail-fast 선택.
    #[error("put_directory concurrency must be ≥ 1, got 0")]
    InvalidConcurrency,
    /// `WalkDir` 가 디렉터리 traversal 중 I/O 에러 (권한 / broken symlink / readdir fail).
    /// 이전 path 가 `filter_map(Result::ok)` 로 silent drop 하던 trick 제거.
    #[error("walk dir {root} failed: {detail}")]
    WalkDir {
        /// traversal 시작 root.
        root: String,
        /// `walkdir::Error` 의 사람-가독 메시지 (path + os error).
        detail: String,
    },
    /// `WalkDir` 가 발견한 파일을 `local_root` 의 *상대* 경로로 변환 못 함 (drive 차이 등).
    /// 이전 path 가 `unwrap_or(&abs)` 로 절대경로 키를 silent 생성하던 trick 제거.
    #[error("strip_prefix failed for {path} (root: {root})")]
    StripPrefix {
        /// 문제의 절대 경로.
        path: String,
        /// traversal root.
        root: String,
    },
    /// Circuit breaker 차단 / max-retries exceeded / timeout.
    #[error("breaker [{op}]: {detail}")]
    Breaker {
        /// 호출 op 이름 (e.g. `r2.put_object_file`).
        op: &'static str,
        /// `BreakerError::{Open|Timeout|MaxRetriesExceeded|Inner}` 의 사람-가독.
        detail: String,
    },
}

/// `BreakerError<UploadError>` → `UploadError` 변환 helper.
fn breaker_to_upload(op: &'static str, e: BreakerError<UploadError>) -> UploadError {
    match e {
        // inner error 가 R2 SDK 호출 자체의 실패면 그 카테고리 그대로 노출 (Put/Get/List).
        BreakerError::Inner(inner) => inner,
        // 그 외 (Open / Timeout / MaxRetriesExceeded) 는 breaker variant 로 박제.
        other => UploadError::Breaker {
            op,
            detail: other.to_string(),
        },
    }
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
/// `Breaker` 는 모든 호출이 공유 — systemic 장애 시 빠른 차단.
#[derive(Debug, Clone)]
pub struct R2Uploader {
    client: S3Client,
    config: R2Config,
    /// T2 — circuit breaker 상태 공유 (모든 R2 호출이 동일 인스턴스).
    breaker: Arc<Breaker>,
    /// `Policy::r2_default()` 박제 — Copy 타입이라 매번 호출에 그대로 통과.
    policy: Policy,
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
        Self {
            client,
            config,
            breaker: Arc::new(Breaker::new()),
            policy: Policy::r2_default(),
        }
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
        // 테스트는 timeout 짧게 + retry 0 — wiremock 응답이 즉시이라 breaker 의미 적음.
        let test_policy = Policy {
            timeout_ms: 5_000,
            max_retries: 0,
            retry_base_ms: 1,
            open_threshold: 99, // 테스트가 의도적 fail 상황 만들어도 open 안 되도록.
            open_window_ms: 1_000,
            open_cooldown_ms: 1_000,
        };
        Self {
            client,
            config,
            breaker: Arc::new(Breaker::new()),
            policy: test_policy,
        }
    }

    /// 설정 (bucket / prefix) 접근자.
    #[must_use]
    pub const fn config(&self) -> &R2Config {
        &self.config
    }

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

        breaker_execute(&self.breaker, &self.policy, "r2.put_object_file", || async {
            let body = ByteStream::from_path(path)
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
        })
        .await
        .map_err(|e| breaker_to_upload("r2.put_object_file", e))?;

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

    /// `DeleteObject` — Round 5 P1 (ADR 0028 + runbook § 6).
    ///
    /// `manifest_backup_cleanup` 이 호출. breaker wrap 통과. idempotent —
    /// 같은 key 가 이미 없어도 `DeleteObject` 는 200 OK (S3 spec).
    ///
    /// # Errors
    ///
    /// `DeleteObject` API 실패 / circuit open / timeout.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn delete_object(&self, key: &str) -> Result<(), UploadError> {
        breaker_execute(&self.breaker, &self.policy, "r2.delete_object", || async {
            self.client
                .delete_object()
                .bucket(&self.config.bucket)
                .key(key)
                .send()
                .await
                .map_err(|e| UploadError::PutObject {
                    key: key.to_owned(),
                    detail: format!(
                        "delete_object: {}",
                        aws_sdk_s3::error::DisplayErrorContext(&e)
                    ),
                })?;
            Ok::<(), UploadError>(())
        })
        .await
        .map_err(|e| breaker_to_upload("r2.delete_object", e))?;
        Ok(())
    }

    /// `ListObjectsV2` paginated — `prefix` 하위 모든 객체 메타 반환.
    ///
    /// R2 의 `ListObjectsV2` 는 default 1000 객체/page → continuation token 으로 loop.
    /// 273 시군구 SHP zip 가정 시 1 page 면 충분하지만 안전하게 pagination 구현.
    /// T2 — 각 page 마다 breaker 통과 (long pagination 의 systemic fail 차단).
    ///
    /// # Errors
    ///
    /// `ListObjectsV2` API 실패 / circuit open / max-retries / timeout.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, prefix = %prefix))]
    pub async fn list_objects(&self, prefix: &str) -> Result<Vec<RemoteObject>, UploadError> {
        let mut all = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let token = continuation.clone();
            let resp = breaker_execute(
                &self.breaker,
                &self.policy,
                "r2.list_objects",
                || async {
                    let mut req = self
                        .client
                        .list_objects_v2()
                        .bucket(&self.config.bucket)
                        .prefix(prefix);
                    if let Some(t) = token.as_deref() {
                        req = req.continuation_token(t);
                    }
                    req.send().await.map_err(|e| UploadError::ListObjects {
                        prefix: prefix.to_owned(),
                        detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                    })
                },
            )
            .await
            .map_err(|e| breaker_to_upload("r2.list_objects", e))?;
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
    /// T2 — `GetObject` 호출만 breaker wrap (body stream 은 connection 후 단일 흐름).
    ///
    /// # Errors
    ///
    /// `GetObject` API 실패 / body stream 실패 / 디스크 I/O 실패 / circuit open / timeout.
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

        let resp = breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.get_object_initiate",
            || async {
                self.client
                    .get_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .send()
                    .await
                    .map_err(|e| UploadError::GetObject {
                        key: key.to_owned(),
                        detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                    })
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.get_object_initiate", e))?;

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

    /// `GetObject` of a *possibly-missing* object — NoSuchKey → `Ok(None)`.
    ///
    /// ## 왜 `get_object_bytes` 와 분리했나
    ///
    /// `get_object_bytes` 는 NoSuchKey 도 `UploadError::GetObject` 로 전파 → breaker 의
    /// `record_failure` 가 호출됨. 이건 manifest *first publish* 같은 *expected miss*
    /// path 에 치명적: (a) NoSuchKey 가 `MaxRetriesExceeded` 로 wrap → typed match 가
    /// `UploadError::GetObject` 를 못 잡음 → first publish 가 영구 실패. (b) 반복되는
    /// expected miss 가 circuit open 트리거 → 후속 정상 GET 도 차단.
    ///
    /// 본 메서드는 NoSuchKey 를 closure *안에서* `Ok(None)` 으로 흡수 — breaker 입장에서는
    /// 성공이라 failure window 누적 0. 다른 모든 에러 (네트워크 / 5xx / 권한) 는 그대로
    /// 전파해서 정상 breaker 로직 유지.
    ///
    /// ## 사용처
    ///
    /// - promote 단계의 `gold/manifest.json` fetch (first publish 시 None — 정상 path).
    /// - promote 의 staging spec fetch (None → typed `MissingLineage` 에러로 매핑).
    ///
    /// # Errors
    ///
    /// `GetObject` API / body stream 실패 (NoSuchKey 제외) / circuit open / timeout.
    #[instrument(skip(self), fields(bucket = %self.config.bucket, key = %key))]
    pub async fn try_get_object_bytes(&self, key: &str) -> Result<Option<Vec<u8>>, UploadError> {
        let resp = breaker_execute(
            &self.breaker,
            &self.policy,
            "r2.try_get_object_bytes",
            || async {
                match self
                    .client
                    .get_object()
                    .bucket(&self.config.bucket)
                    .key(key)
                    .send()
                    .await
                {
                    Ok(r) => Ok(Some(r)),
                    Err(e) => {
                        // NoSuchKey 는 expected miss — breaker 입장에서는 성공으로 처리.
                        if let Some(svc_err) = e.as_service_error() {
                            if svc_err.is_no_such_key() {
                                return Ok(None);
                            }
                        }
                        Err(UploadError::GetObject {
                            key: key.to_owned(),
                            detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                        })
                    }
                }
            },
        )
        .await
        .map_err(|e| breaker_to_upload("r2.try_get_object_bytes", e))?;

        let Some(resp) = resp else {
            return Ok(None);
        };

        let mut body = resp.body;
        let mut buf = Vec::new();
        while let Some(chunk) = body.next().await {
            let chunk = chunk.map_err(|e| UploadError::BodyStream {
                key: key.to_owned(),
                detail: format!("{e}"),
            })?;
            buf.extend_from_slice(&chunk);
        }
        Ok(Some(buf))
    }

    /// JSON pretty-encoded 객체 업로드. `content_type=application/json`.
    ///
    /// manifest / index 류에 사용 — 작은 (~10KB) 페이로드 가정.
    /// T2 — circuit breaker wrap.
    ///
    /// `T: Sync` 는 `#[instrument]` 가 만드는 future 가 multi-thread runtime
    /// 에서도 안전하게 await 되도록 강제 (clippy `future_not_send`).
    ///
    /// # Errors
    ///
    /// JSON 직렬화 실패 / `PutObject` 실패 / circuit open / max-retries / timeout.
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

        // breaker 의 retry 가 새 future 를 매번 만들기 때문에 body 도 매 호출마다 fresh.
        // `Arc<Vec<u8>>` 으로 share — clone 비용 감소.
        let json_arc = Arc::new(json);
        breaker_execute(&self.breaker, &self.policy, "r2.put_object_json", || async {
            let body = ByteStream::from(json_arc.as_ref().clone());
            self.client
                .put_object()
                .bucket(&self.config.bucket)
                .key(key)
                .body(body)
                .content_type("application/json")
                .cache_control(cache_control)
                .server_side_encryption(aws_sdk_s3::types::ServerSideEncryption::Aes256)
                .send()
                .await
                .map_err(|e| UploadError::PutObject {
                    key: key.to_owned(),
                    detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
                })?;
            Ok::<(), UploadError>(())
        })
        .await
        .map_err(|e| breaker_to_upload("r2.put_object_json", e))?;

        info!("json uploaded");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

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

    // P2: R2Config key layout SSOT property tests.
    // 이 테스트들이 곧 "key layout 이 변경되면 컴파일러 차단" 보장.
    // URL 변경 = ADR + 이 테스트 갱신 = backward-compatibility gate.

    fn v(s: &str) -> Version {
        Version::new(s).expect("test version must be valid")
    }

    fn pub_base(s: &str) -> R2PublicBase {
        R2PublicBase::new(s).expect("test public base must be valid")
    }

    #[test]
    fn gold_layer_prefix_layout() {
        let cfg = test_config("bucket");
        assert_eq!(
            cfg.gold_layer_prefix(&v("v3"), "parcels"),
            "gold/v3/parcels"
        );
    }

    #[test]
    fn tilejson_key_layout() {
        let cfg = test_config("bucket");
        assert_eq!(
            cfg.tilejson_key(&v("v3"), "parcels"),
            "gold/v3/parcels.json"
        );
    }

    #[test]
    fn manifest_key_layout() {
        let cfg = test_config("bucket");
        assert_eq!(cfg.manifest_key(), "gold/manifest.json");
    }

    #[test]
    fn manifest_backup_key_layout() {
        let cfg = test_config("bucket");
        assert_eq!(
            cfg.manifest_backup_key(&v("v2")),
            "gold/manifest.v2.json"
        );
    }

    #[test]
    fn staging_spec_key_layout() {
        let cfg = test_config("bucket");
        assert_eq!(
            cfg.staging_spec_key(&v("v3"), "admin"),
            "gold/staging/v3/admin.spec.json"
        );
    }

    #[test]
    fn tiles_url_template_with_trailing_slash() {
        let cfg = test_config("bucket");
        let url = cfg.tiles_url_template(
            &pub_base("https://r2.example.com/"),
            &v("v3"),
            "parcels",
        );
        assert_eq!(
            url,
            "https://r2.example.com/gold/v3/parcels/{z}/{x}/{y}.pbf"
        );
    }

    #[test]
    fn tiles_url_template_without_trailing_slash() {
        let cfg = test_config("bucket");
        let url = cfg.tiles_url_template(
            &pub_base("https://r2.example.com"),
            &v("v3"),
            "admin",
        );
        assert_eq!(
            url,
            "https://r2.example.com/gold/v3/admin/{z}/{x}/{y}.pbf"
        );
    }

    #[test]
    fn key_helpers_round_trip_coverage() {
        // 모든 helper 가 gold_prefix 를 일관되게 prefix 로 사용하는지 확인.
        // gold_prefix 변경 시 모든 key 가 한꺼번에 변경됨을 보장.
        let cfg = R2Config {
            account_id: "fake".into(),
            access_key: "fake".into(),
            secret_key: "fake".into(),
            bucket: "bucket".into(),
            bronze_prefix: "bronze".into(),
            gold_prefix: "custom-gold".into(),
        };
        let ver = v("v1");
        let prefix = cfg.gold_layer_prefix(&ver, "parcels");
        assert!(prefix.starts_with("custom-gold/"), "gold_prefix must be respected");
        assert!(cfg.manifest_key().starts_with("custom-gold/"), "manifest must use gold_prefix");
        assert!(
            cfg.staging_spec_key(&ver, "parcels").starts_with("custom-gold/"),
            "staging key must use gold_prefix"
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

    /// P0 (Codex Round 3): `concurrency: 0` 은 fail-fast — `buffer_unordered(0)`
    /// 가 stream 정지 시키는 silent failure 차단.
    #[tokio::test]
    async fn put_directory_rejects_zero_concurrency() {
        let server = MockServer::start().await;
        let cfg = test_config("test-bucket");
        let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
        let tmp = tempfile::tempdir().expect("tempdir");
        let err = uploader
            .put_directory(tmp.path(), "gold/v1/parcels", 0)
            .await
            .expect_err("concurrency=0 must reject");
        assert!(matches!(err, UploadError::InvalidConcurrency));
    }

    /// 회귀 테스트 — Codex stop-time review 발견 (Round 2 hotfix):
    /// breaker wrap 이 first publish promote 를 깨뜨림. `try_get_object_bytes` 가
    /// NoSuchKey 를 `Ok(None)` 으로 흡수해야 (1) typed `Option` 분기 + (2) breaker
    /// failure window 누적 0.
    #[tokio::test]
    async fn try_get_object_bytes_returns_none_on_no_such_key() {
        let server = MockServer::start().await;
        // S3 NoSuchKey 응답 — 정확한 wire format (status 404 + AWS XML body).
        Mock::given(method("GET"))
            .and(path("/test-bucket/gold/manifest.json"))
            .respond_with(ResponseTemplate::new(404).set_body_string(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <Error><Code>NoSuchKey</Code><Message>The specified key does not exist.</Message>\
                 <Key>gold/manifest.json</Key><RequestId>test</RequestId></Error>",
            ))
            .expect(1)
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        let uploader = R2Uploader::with_endpoint_override(cfg, server.uri());

        let result = uploader
            .try_get_object_bytes("gold/manifest.json")
            .await
            .expect("NoSuchKey must be Ok(None), not Err");
        assert!(result.is_none(), "expected None for NoSuchKey, got Some");
    }

    /// 회귀 테스트 — NoSuchKey 가 breaker failure 로 카운트되지 않아야 함
    /// (반복되는 expected miss 가 circuit open 트리거하면 first-publish 가 영구 차단됨).
    #[tokio::test]
    async fn try_get_object_bytes_no_such_key_does_not_open_breaker() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404).set_body_string(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <Error><Code>NoSuchKey</Code><Message>not found</Message></Error>",
            ))
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        let mut uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
        // 매우 낮은 threshold — 만약 NoSuchKey 가 실패로 카운트되면 1번에 open.
        uploader.policy = circuit_breaker::Policy {
            timeout_ms: 1_000,
            max_retries: 0,
            retry_base_ms: 1,
            open_threshold: 1,
            open_window_ms: 60_000,
            open_cooldown_ms: 60_000,
        };

        // 5번 연속 NoSuchKey — open 안 되어야 함.
        for _ in 0..5 {
            let r = uploader.try_get_object_bytes("missing.json").await;
            assert!(
                matches!(r, Ok(None)),
                "expected Ok(None), got: {r:?}"
            );
        }
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
        // T2 — breaker wrap: inner error 가 MaxRetriesExceeded 로 전파 → `Breaker` variant.
        // op 식별자 + 원본 stderr (`InternalError`) 가 detail 에 보존됨을 검증.
        match err {
            UploadError::Breaker { op, detail } => {
                assert_eq!(op, "r2.put_object_file");
                assert!(
                    detail.contains("InternalError") || detail.contains("put_object"),
                    "breaker detail must preserve inner PutObject context: {detail}"
                );
            }
            other => panic!("expected Breaker variant, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn breaker_opens_after_repeated_500_failures() {
        // T2 회귀 — circuit-breaker 가 R2 systemic 장애 시 fast-fail 를 보장.
        // open_threshold=5 (`Policy::r2_default`) 라 max_retries 1 + 의도적 5xx 가
        // 누적되어 open 으로 전이.
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(500).set_body_string(
                "<Error><Code>InternalError</Code><Message>oops</Message></Error>",
            ))
            .mount(&server)
            .await;

        let cfg = test_config("test-bucket");
        // production-like policy 와 비슷하지만 timeout 짧게 + cooldown 짧게.
        let mut uploader = R2Uploader::with_endpoint_override(cfg, server.uri());
        uploader.policy = circuit_breaker::Policy {
            timeout_ms: 1_000,
            max_retries: 0,
            retry_base_ms: 1,
            open_threshold: 3,
            open_window_ms: 60_000,
            open_cooldown_ms: 60_000,
        };

        let mut tmp = tempfile::NamedTempFile::new().expect("tempfile");
        tmp.write_all(b"x").expect("write");

        // 3회 실패 누적 → 4회째 호출은 즉시 Open 으로 거부.
        for _ in 0..3 {
            let _ = uploader
                .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
                .await;
        }
        let err = uploader
            .put_object_file("bronze/x.bin", tmp.path(), "application/octet-stream")
            .await
            .expect_err("breaker should be open");
        match err {
            UploadError::Breaker { detail, .. } => {
                assert!(
                    detail.contains("circuit open"),
                    "expected open-state detail, got: {detail}"
                );
            }
            other => panic!("expected Breaker(Open), got: {other:?}"),
        }
    }
}
