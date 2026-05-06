//! R2 HTTP 클라이언트 — `reqwest` GET + circuit breaker.
//!
//! 1차 = public-read bucket 가정. private bucket / pre-signed URL 은 FU 67
//! (aws-sdk-s3 SigV4 통합). 본 1차는 단순 `reqwest::get(public_url)` — R2 가
//! S3-호환이지만 public objects 라 SigV4 불필요.

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::env;
use std::time::Duration;

use bytes::Bytes;
use circuit_breaker::{execute, Breaker, BreakerError, Policy};
use tracing::instrument;

use crate::error::ConfigError;

/// R2 client 환경 설정.
///
/// 1차 = public-read bucket. R2 dashboard 또는 Cloudflare CDN 가 노출하는
/// HTTPS URL base 만 필요.
#[derive(Debug, Clone)]
pub struct R2Config {
    /// 공개 URL base (예: `https://pub-<hash>.r2.dev` 또는 CDN).
    /// 끝에 `/` 없이 — 객체 key 가 `static/parcels.pmtiles` 형태로 붙음.
    pub public_url_base: String,
}

impl R2Config {
    /// 환경변수에서 설정 로드.
    ///
    /// 필수 — `R2_PUBLIC_URL_BASE`.
    ///
    /// # Errors
    ///
    /// 미설정 / 빈 문자열 → [`ConfigError`].
    pub fn from_env() -> Result<Self, ConfigError> {
        let public_url_base = require_env("R2_PUBLIC_URL_BASE")?;
        Ok(Self { public_url_base })
    }
}

fn require_env(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(v) if v.trim().is_empty() => Err(ConfigError::EmptyEnv(name)),
        Ok(v) => Ok(v),
        Err(_) => Err(ConfigError::MissingEnv(name)),
    }
}

/// R2 HTTP 클라이언트.
///
/// `reqwest::Client` + `Breaker` + `Policy::r2_default`. 객체 key 를 받아
/// `{public_url_base}/{key}` 로 GET.
#[derive(Debug)]
pub struct R2Client {
    http: reqwest::Client,
    config: R2Config,
    breaker: Breaker,
    policy: Policy,
}

impl R2Client {
    /// R2 표준 정책 (`Policy::r2_default`) 으로 새 클라이언트.
    #[must_use]
    pub fn new(config: R2Config) -> Self {
        Self::with_policy(config, Policy::r2_default())
    }

    /// 정책 명시 — 테스트 / 특수 케이스용.
    #[must_use]
    pub fn with_policy(config: R2Config, policy: Policy) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_millis(policy.timeout_ms + 1_000))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            config,
            breaker: Breaker::new(),
            policy,
        }
    }

    /// 객체 key 에 해당하는 binary 응답 (PMTiles tile / JSON 인덱스 등).
    ///
    /// `{public_url_base}/{key}` GET. circuit breaker 통과 — timeout / retry
    /// / open 자동.
    ///
    /// # Errors
    ///
    /// 네트워크 오류 / 4xx / 5xx → [`BreakerError`].
    #[instrument(skip(self), fields(key = %key))]
    pub async fn get_object_bytes(&self, key: &str) -> Result<Bytes, BreakerError<reqwest::Error>> {
        let base = &self.config.public_url_base;
        let url = format!("{base}/{key}");

        execute(&self.breaker, &self.policy, "r2.get_object", || async {
            let resp = self.http.get(&url).send().await?;
            resp.error_for_status()?.bytes().await
        })
        .await
    }

    /// 객체 key 에 해당하는 JSON 응답 → `serde_json::Value`.
    ///
    /// `pnu_to_buildings.json` 같은 인덱스 파일 fetch 용.
    ///
    /// # Errors
    ///
    /// 네트워크 / 파싱 오류 → [`BreakerError`].
    #[instrument(skip(self), fields(key = %key))]
    pub async fn get_object_json(
        &self,
        key: &str,
    ) -> Result<serde_json::Value, BreakerError<reqwest::Error>> {
        let base = &self.config.public_url_base;
        let url = format!("{base}/{key}");

        execute(
            &self.breaker,
            &self.policy,
            "r2.get_object_json",
            || async {
                let resp = self.http.get(&url).send().await?;
                resp.error_for_status()?.json::<serde_json::Value>().await
            },
        )
        .await
    }

    /// 내부용 — config 접근.
    #[must_use]
    pub fn public_url_base(&self) -> &str {
        &self.config.public_url_base
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn config_from_env_missing_returns_error() {
        let result = require_env("__GONGZZANG_NEVER_SET_ENV__");
        assert!(matches!(result, Err(ConfigError::MissingEnv(_))));
    }

    #[test]
    fn client_exposes_public_url_base() {
        let cfg = R2Config {
            public_url_base: "https://pub-abc.r2.dev".to_owned(),
        };
        let client = R2Client::new(cfg);
        assert_eq!(client.public_url_base(), "https://pub-abc.r2.dev");
    }

    #[test]
    fn with_policy_overrides_default() {
        let cfg = R2Config {
            public_url_base: "https://pub-x.r2.dev".to_owned(),
        };
        let custom = Policy::vworld_default();
        let client = R2Client::with_policy(cfg, custom);
        assert_eq!(client.policy.timeout_ms, 10_000);
    }
}
