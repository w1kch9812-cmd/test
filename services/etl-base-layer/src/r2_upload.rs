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

use aws_config::Region;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{BehaviorVersion, Builder as S3ConfigBuilder};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use thiserror::Error;
use tracing::{info, instrument};

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

/// R2 업로드 에러.
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
    /// JSON 직렬화 실패.
    #[error("json serialize failed: {0}")]
    JsonSerialize(#[from] serde_json::Error),
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
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(config.endpoint_url())
            .credentials_provider(creds)
            .force_path_style(true)
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

        info!("uploading file → R2");

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(body)
            .content_type(content_type)
            .cache_control("public, max-age=31536000")
            .send()
            .await
            .map_err(|e| UploadError::PutObject {
                key: key.to_owned(),
                detail: format!("{}", aws_sdk_s3::error::DisplayErrorContext(&e)),
            })?;

        info!("file uploaded");
        Ok(())
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

        info!(bytes = bytes_len, "uploading json → R2");

        self.client
            .put_object()
            .bucket(&self.config.bucket)
            .key(key)
            .body(ByteStream::from(json))
            .content_type("application/json")
            .cache_control(cache_control)
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
