use std::sync::Arc;

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::Client as S3Client;
use circuit_breaker::{Breaker, Policy};

use super::config::R2Config;

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
    pub(super) client: S3Client,
    pub(super) config: R2Config,
    /// T2 — circuit breaker 상태 공유 (모든 R2 호출이 동일 인스턴스).
    pub(super) breaker: Arc<Breaker>,
    /// `Policy::r2_default()` 박제 — Copy 타입이라 매번 호출에 그대로 통과.
    pub(super) policy: Policy,
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
}
