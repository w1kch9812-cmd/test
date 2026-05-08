//! `R2RawCapture` — `RawCapture` 의 R2 (S3-호환) 구현체. ADR 0026.
//!
//! 외부 API raw 응답을 `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json` 키로
//! append-only 저장. Postgres jsonb 보존 (`PgRawCapture`) 폐기 (cost + UPSERT 시계열
//! 손실).
//!
//! # 키 구조 (ADR 0026)
//!
//! ```text
//! gongzzang/bronze/data_go_kr_building/2026/05/08/1168010100107370000_1715156234567.json
//! └bucket   └bronze prefix              └yyyy└mm└dd└pnu                └epoch_ms      .json
//! ```
//!
//! - `epoch_ms` = `fetched_at` 의 epoch milliseconds. 같은 (pnu, source) 가 시간이 흐르며
//!   다른 응답을 보내도 *모든 시점* 보존 — 진짜 append-only.
//! - 일자 prefix → R2 lifecycle policy / 분석 (e.g. `aws s3 ls bronze/.../2026/05/08/`).

#![allow(clippy::module_name_repetitions)]

use std::env;
use std::time::Duration;

use async_trait::async_trait;
use aws_credential_types::Credentials;
use aws_sdk_s3::config::{
    BehaviorVersion, Builder as S3ConfigBuilder, Region, RequestChecksumCalculation,
    ResponseChecksumValidation,
};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client as S3Client;
use chrono::{DateTime, Datelike, Utc};
use raw_capture_client::{RawCapture, RawCaptureError};
use thiserror::Error;
use tracing::instrument;

/// R2 Bronze archive 환경 설정.
#[derive(Debug, Clone)]
pub struct R2RawCaptureConfig {
    /// Cloudflare account id (endpoint 구성).
    pub account_id: String,
    /// R2 access key id (S3-호환).
    pub access_key: String,
    /// R2 secret key.
    pub secret_key: String,
    /// 대상 버킷.
    pub bucket: String,
    /// Bronze prefix (예: `"bronze"`). 끝 `/` 제외.
    pub bronze_prefix: String,
}

/// 설정 로드 에러.
#[derive(Debug, Error)]
pub enum R2ConfigError {
    /// 환경변수 누락.
    #[error("env {0} not set")]
    MissingEnv(&'static str),
    /// 환경변수 빈 문자열.
    #[error("env {0} empty")]
    EmptyEnv(&'static str),
}

impl R2RawCaptureConfig {
    /// 환경변수 로드 — `R2_ACCOUNT_ID` / `R2_ACCESS_KEY` / `R2_SECRET_KEY` / `R2_BUCKET`.
    /// `BRONZE_PREFIX` 는 옵션 (default `"bronze"`).
    ///
    /// # Errors
    /// 필수 변수 누락 / 빈 값.
    pub fn from_env() -> Result<Self, R2ConfigError> {
        Ok(Self {
            account_id: require_env("R2_ACCOUNT_ID")?,
            access_key: require_env("R2_ACCESS_KEY")?,
            secret_key: require_env("R2_SECRET_KEY")?,
            bucket: require_env("R2_BUCKET")?,
            bronze_prefix: env::var("BRONZE_PREFIX").unwrap_or_else(|_| "bronze".to_owned()),
        })
    }

    /// `https://<account_id>.r2.cloudflarestorage.com`.
    #[must_use]
    pub fn endpoint_url(&self) -> String {
        format!("https://{}.r2.cloudflarestorage.com", self.account_id)
    }
}

fn require_env(name: &'static str) -> Result<String, R2ConfigError> {
    match env::var(name) {
        Ok(v) if v.trim().is_empty() => Err(R2ConfigError::EmptyEnv(name)),
        Ok(v) => Ok(v),
        Err(_) => Err(R2ConfigError::MissingEnv(name)),
    }
}

/// `RawCapture` 의 R2 구현체. ADR 0026.
#[derive(Debug, Clone)]
pub struct R2RawCapture {
    client: S3Client,
    bucket: String,
    bronze_prefix: String,
}

impl R2RawCapture {
    /// 새 [`R2RawCapture`].
    #[must_use]
    pub fn new(config: R2RawCaptureConfig) -> Self {
        let creds = Credentials::new(
            &config.access_key,
            &config.secret_key,
            None,
            None,
            "api-r2-raw-capture",
        );
        // R2 는 region 무시하지만 SigV4 가 필수로 요구 — `auto` 사용.
        // R2 가 `STREAMING-UNSIGNED-PAYLOAD-TRAILER` 와 호환 안 함 → checksum WhenRequired.
        let s3_config = S3ConfigBuilder::default()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new("auto"))
            .endpoint_url(config.endpoint_url())
            .credentials_provider(creds)
            .force_path_style(true)
            .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
            .response_checksum_validation(ResponseChecksumValidation::WhenRequired)
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(3))
            .timeout_config(
                aws_config::timeout::TimeoutConfig::builder()
                    .operation_attempt_timeout(Duration::from_secs(15))
                    .build(),
            )
            .build();
        Self {
            client: S3Client::from_conf(s3_config),
            bucket: config.bucket,
            bronze_prefix: config.bronze_prefix,
        }
    }

    /// `bronze/{source}/{yyyy}/{mm}/{dd}/{pnu}_{epoch_ms}.json` 빌드.
    fn build_key(&self, pnu: &str, source: &str, fetched_at: DateTime<Utc>) -> String {
        format!(
            "{prefix}/{source}/{year:04}/{month:02}/{day:02}/{pnu}_{ts}.json",
            prefix = self.bronze_prefix,
            source = source,
            year = fetched_at.year(),
            month = fetched_at.month(),
            day = fetched_at.day(),
            pnu = pnu,
            ts = fetched_at.timestamp_millis(),
        )
    }
}

#[async_trait]
impl RawCapture for R2RawCapture {
    #[instrument(skip(self, raw), fields(pnu = %pnu, source = %source))]
    async fn capture(
        &self,
        pnu: &str,
        source: &str,
        raw: &serde_json::Value,
        fetched_at: DateTime<Utc>,
    ) -> Result<(), RawCaptureError> {
        let key = self.build_key(pnu, source, fetched_at);
        let body = serde_json::to_vec(raw)
            .map_err(|e| RawCaptureError::Sink(format!("json serialize: {e}")))?;
        let bytes_len = body.len();
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(body))
            .content_type("application/json")
            .send()
            .await
            .map_err(|e| RawCaptureError::Sink(format!("r2 put_object {key}: {e}")))?;
        tracing::info!(
            event = "raw_capture.r2.put",
            key = %key,
            bytes = bytes_len,
            "Bronze R2 PUT 성공"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::TimeZone;

    fn cfg() -> R2RawCaptureConfig {
        R2RawCaptureConfig {
            account_id: "acc".to_owned(),
            access_key: "ak".to_owned(),
            secret_key: "sk".to_owned(),
            bucket: "gongzzang".to_owned(),
            bronze_prefix: "bronze".to_owned(),
        }
    }

    #[test]
    fn endpoint_url_format() {
        assert_eq!(
            cfg().endpoint_url(),
            "https://acc.r2.cloudflarestorage.com"
        );
    }

    #[test]
    fn build_key_yyyy_mm_dd_zero_padded() {
        let capture = R2RawCapture::new(cfg());
        // 2026-05-08 03:04:05.067 UTC = epoch_ms 1778554645067
        let ts = Utc.with_ymd_and_hms(2026, 5, 8, 3, 4, 5).unwrap();
        let key = capture.build_key("1168010100107370000", "data_go_kr_building", ts);
        assert!(key.starts_with("bronze/data_go_kr_building/2026/05/08/"));
        assert!(key.contains("1168010100107370000_"));
        assert!(std::path::Path::new(&key)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json")));
    }

    #[test]
    fn build_key_january_pads_month() {
        let capture = R2RawCapture::new(cfg());
        let ts = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let key = capture.build_key("1111010100100010000", "vworld", ts);
        assert!(key.contains("/2026/01/01/"));
    }

    #[test]
    fn epoch_ms_distinguishes_two_calls_same_pnu() {
        let capture = R2RawCapture::new(cfg());
        let t1 = Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 0).unwrap();
        let t2 = Utc
            .with_ymd_and_hms(2026, 5, 8, 0, 0, 1)
            .unwrap();
        let k1 = capture.build_key("1111010100100010000", "vworld", t1);
        let k2 = capture.build_key("1111010100100010000", "vworld", t2);
        assert_ne!(k1, k2, "epoch_ms 가 다르면 키가 달라야 함 (append-only 보장)");
    }
}
