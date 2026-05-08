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
//!
//! # 신뢰성 보호 (Codex stop-time review fix)
//!
//! 1. **Circuit Breaker (FU 26 강제)** — 모든 R2 PUT 은 `circuit_breaker::execute` 통과
//!    (`Policy::r2_default`). systemic 장애 시 빠른 차단 + retry/timeout 자동.
//! 2. **로컬 디스크 fallback** — R2 PUT 최종 실패 시 `BRONZE_FALLBACK_DIR` 에 동일 키
//!    구조로 저장. 운영팀이 사후 `aws s3 sync` 로 R2 에 옮김. raw 영구 손실 0.
//! 3. **fallback 도 실패하면** 그때야 `RawCaptureError::Sink` 반환 — caller (best-effort
//!    pattern) 가 warn 후 정상 진행. 즉 *raw 손실은 R2 와 디스크 모두 죽어야 발생*.

#![allow(clippy::module_name_repetitions)]

use std::env;
use std::path::PathBuf;
use std::sync::Arc;
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
use circuit_breaker::{execute, Breaker, BreakerError, Policy};
use raw_capture_client::{RawCapture, RawCaptureError};
use thiserror::Error;
use tracing::{instrument, warn};

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
    /// R2 PUT 최종 실패 시 fallback 저장 디렉터리.
    /// `None` 이면 fallback 0 — R2 죽으면 raw 손실 (dev/test 시).
    /// production 은 *반드시* 설정 (예: `/var/lib/gongzzang/bronze-fallback`).
    pub fallback_dir: Option<PathBuf>,
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
    /// 옵션: `BRONZE_PREFIX` (default `"bronze"`), `BRONZE_FALLBACK_DIR` (default `None`).
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
            fallback_dir: env::var("BRONZE_FALLBACK_DIR")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from),
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

/// `RawCapture` 의 R2 구현체. ADR 0026 + Codex stop-time review fix.
#[derive(Debug, Clone)]
pub struct R2RawCapture {
    client: S3Client,
    bucket: String,
    bronze_prefix: String,
    fallback_dir: Option<PathBuf>,
    /// FU 26: R2 호출 모두 본 breaker 공유 (systemic 장애 시 빠른 차단).
    breaker: Arc<Breaker>,
    policy: Policy,
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
            // SDK retry 도 가능하지만 우리는 circuit_breaker::execute 의 retry 정책으로
            // 통일 (Policy::r2_default) — SDK 내부 retry 는 1로 둠.
            .retry_config(aws_config::retry::RetryConfig::standard().with_max_attempts(1))
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
            fallback_dir: config.fallback_dir,
            breaker: Arc::new(Breaker::new()),
            policy: Policy::r2_default(),
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

    /// R2 PUT — circuit breaker 통과. body 는 한 번만 owned, retry 시 재생성 필요해 호출자가 owned 전달.
    async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<(), BreakerError<RawCaptureError>> {
        let body_arc = Arc::new(body);
        execute(
            &self.breaker,
            &self.policy,
            "r2.raw_capture.put_object",
            || {
                let body_arc = Arc::clone(&body_arc);
                async move {
                    self.client
                        .put_object()
                        .bucket(&self.bucket)
                        .key(key)
                        .body(ByteStream::from((*body_arc).clone()))
                        .content_type("application/json")
                        .send()
                        .await
                        .map_err(|e| RawCaptureError::Sink(format!("r2 put_object {key}: {e}")))?;
                    Ok(())
                }
            },
        )
        .await
    }

    /// fallback — 로컬 디스크에 동일 키 구조로 저장. R2 PUT 실패 후 raw 영구 손실 차단.
    /// 운영팀이 사후 `aws s3 sync {fallback_dir}/ s3://{bucket}/` 로 옮김.
    fn write_fallback(&self, key: &str, body: &[u8]) -> Result<PathBuf, std::io::Error> {
        let Some(base) = self.fallback_dir.as_ref() else {
            return Err(std::io::Error::other("fallback_dir not configured"));
        };
        let path = base.join(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, body)?;
        Ok(path)
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

        match self.put_object(&key, body.clone()).await {
            Ok(()) => {
                tracing::info!(
                    event = "raw_capture.r2.put",
                    key = %key,
                    bytes = bytes_len,
                    "Bronze R2 PUT 성공"
                );
                Ok(())
            }
            Err(r2_err) => {
                // R2 최종 실패 → 로컬 fallback 시도 (raw 손실 차단).
                match self.write_fallback(&key, &body) {
                    Ok(path) => {
                        warn!(
                            event = "raw_capture.r2.fallback_disk",
                            key = %key,
                            fallback_path = %path.display(),
                            bytes = bytes_len,
                            r2_error = %r2_err,
                            "R2 PUT 실패 → 로컬 디스크 fallback 저장 (운영팀 사후 sync 필요)"
                        );
                        Ok(())
                    }
                    Err(disk_err) => {
                        // R2 + 디스크 둘 다 죽음 — 진짜 raw 손실.
                        Err(RawCaptureError::Sink(format!(
                            "r2 put + disk fallback both failed (key={key}): r2={r2_err}, disk={disk_err}"
                        )))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    fn cfg() -> R2RawCaptureConfig {
        R2RawCaptureConfig {
            account_id: "acc".to_owned(),
            access_key: "ak".to_owned(),
            secret_key: "sk".to_owned(),
            bucket: "gongzzang".to_owned(),
            bronze_prefix: "bronze".to_owned(),
            fallback_dir: None,
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
        let t2 = Utc.with_ymd_and_hms(2026, 5, 8, 0, 0, 1).unwrap();
        let k1 = capture.build_key("1111010100100010000", "vworld", t1);
        let k2 = capture.build_key("1111010100100010000", "vworld", t2);
        assert_ne!(k1, k2, "epoch_ms 가 다른 시점은 다른 키 (append-only)");
    }

    #[test]
    fn fallback_writes_to_disk_with_full_key_path() {
        let tmp = TempDir::new().expect("tempdir");
        let mut c = cfg();
        c.fallback_dir = Some(tmp.path().to_owned());
        let capture = R2RawCapture::new(c);
        let key = "bronze/vworld/2026/05/08/1234567890123456789_1715156234567.json";
        let path = capture
            .write_fallback(key, b"{\"raw\":true}")
            .expect("fallback write");
        assert!(path.exists(), "fallback file 생성 안 됨");
        assert_eq!(path, tmp.path().join(key));
        let content = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(content, "{\"raw\":true}");
    }

    #[test]
    fn fallback_without_dir_returns_err() {
        let capture = R2RawCapture::new(cfg());
        let err = capture
            .write_fallback("bronze/x/2026/05/08/p_1.json", b"{}")
            .expect_err("must err");
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
    }
}
