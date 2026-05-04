//! data.go.kr HTTP 클라이언트 — `reqwest` + `circuit-breaker`.
//!
//! `DataGoKrClient` 가 data.go.kr API 단일 진입점. 각 endpoint 는 sub-module
//! (`building_register`, FU: `land_register`, `real_transaction`) 에서 별도 호출.
//!
//! API 인증 방식 — `serviceKey` query param. 본 클라이언트는 `serviceKey` 를
//! 멤버에 보유, 호출 시점에 URL 에 합성.

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::env;
use std::time::Duration;

use circuit_breaker::{Breaker, Policy};

use crate::error::ConfigError;

/// data.go.kr API 환경 설정.
#[derive(Debug, Clone)]
pub struct DataGoKrConfig {
    /// data.go.kr 서비스 키 (Open Data Portal 발급, URL-encoded 가능).
    pub service_key: String,
    /// API base URL — 테스트 시 mock server 로 override.
    pub base_url: String,
}

impl DataGoKrConfig {
    /// 환경변수에서 설정 로드.
    ///
    /// 필수 — `ODP_SERVICE_KEY`.
    /// 선택 — `ODP_BASE_URL` (default `https://apis.data.go.kr`).
    ///
    /// # Errors
    ///
    /// 필수 변수 미설정 또는 빈 문자열이면 [`ConfigError`].
    pub fn from_env() -> Result<Self, ConfigError> {
        let service_key = require_env("ODP_SERVICE_KEY")?;
        let base_url =
            env::var("ODP_BASE_URL").unwrap_or_else(|_| "https://apis.data.go.kr".to_owned());
        Ok(Self {
            service_key,
            base_url,
        })
    }
}

fn require_env(name: &'static str) -> Result<String, ConfigError> {
    match env::var(name) {
        Ok(v) if v.trim().is_empty() => Err(ConfigError::EmptyEnv(name)),
        Ok(v) => Ok(v),
        Err(_) => Err(ConfigError::MissingEnv(name)),
    }
}

/// data.go.kr HTTP 클라이언트.
///
/// `reqwest::Client` + `Breaker` + `Policy::data_go_kr_default`. 모든 endpoint
/// 호출은 `circuit_breaker::execute` 통과 — 자동 timeout / retry / circuit breaking.
#[derive(Debug)]
pub struct DataGoKrClient {
    pub(crate) http: reqwest::Client,
    pub(crate) config: DataGoKrConfig,
    pub(crate) breaker: Breaker,
    pub(crate) policy: Policy,
}

impl DataGoKrClient {
    /// data.go.kr 표준 정책 (`Policy::data_go_kr_default`) 으로 새 클라이언트.
    #[must_use]
    pub fn new(config: DataGoKrConfig) -> Self {
        Self::with_policy(config, Policy::data_go_kr_default())
    }

    /// 정책 명시 — 테스트 / 특수 케이스용.
    #[must_use]
    pub fn with_policy(config: DataGoKrConfig, policy: Policy) -> Self {
        let http = reqwest::Client::builder()
            // 클라이언트 자체 timeout 은 policy.timeout_ms 보다 약간 길게
            // (circuit breaker 가 우선) — 안전망.
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

    /// 내부용 — base_url 접근.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.config.base_url
    }

    /// 내부용 — service_key 접근.
    #[must_use]
    pub fn service_key(&self) -> &str {
        &self.config.service_key
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
    fn client_exposes_base_url_and_service_key() {
        let cfg = DataGoKrConfig {
            service_key: "test-key".to_owned(),
            base_url: "http://localhost:9999".to_owned(),
        };
        let client = DataGoKrClient::new(cfg);
        assert_eq!(client.base_url(), "http://localhost:9999");
        assert_eq!(client.service_key(), "test-key");
    }

    #[test]
    fn with_policy_overrides_default() {
        let cfg = DataGoKrConfig {
            service_key: "k".to_owned(),
            base_url: "http://x".to_owned(),
        };
        let custom = Policy::vworld_default(); // 다른 정책 사용 — override 검증.
        let client = DataGoKrClient::with_policy(cfg, custom);
        assert_eq!(client.policy.timeout_ms, 10_000);
    }
}
