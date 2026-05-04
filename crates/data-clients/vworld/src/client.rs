//! V-World HTTP 클라이언트 — `reqwest` + `circuit-breaker`.
//!
//! `VWorldClient` 가 V-World API 단일 진입점:
//! - WFS GetFeature: `fetch_feature_by_pnu(layer, pnu)`
//! - 모든 호출은 `circuit_breaker::execute` 를 통과
//!
//! API URL 형식 (`docs/data-sources/v-world.md`):
//! ```text
//! https://api.vworld.kr/req/data
//!   ?service=data
//!   &request=GetFeature
//!   &data=<layer>
//!   &key=<api_key>
//!   &domain=<domain>
//!   &attrFilter=pnu:=:<pnu>
//!   &format=json
//!   &size=10
//!   &geometry=true
//!   &crs=EPSG:4326
//! ```

#![allow(clippy::module_name_repetitions, clippy::doc_markdown)]

use std::env;
use std::time::Duration;

use circuit_breaker::{execute, Breaker, BreakerError, Policy};
use serde_json::Value;
use tracing::instrument;

use crate::error::ConfigError;

/// V-World API 환경 설정.
#[derive(Debug, Clone)]
pub struct VWorldConfig {
    /// V-World API 키 (개발자 센터 발급).
    pub api_key: String,
    /// 등록된 도메인 (Referer 검증). 개발 시 `localhost`.
    pub domain: String,
    /// API base URL — 테스트 시 mock server 로 override 가능.
    pub base_url: String,
}

impl VWorldConfig {
    /// 환경변수에서 설정 로드.
    ///
    /// 필수 — `VWORLD_API_KEY`, `VWORLD_DOMAIN`.
    /// 선택 — `VWORLD_BASE_URL` (default `https://api.vworld.kr`).
    ///
    /// # Errors
    ///
    /// 필수 변수 미설정 또는 빈 문자열이면 [`ConfigError`].
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key = require_env("VWORLD_API_KEY")?;
        let domain = require_env("VWORLD_DOMAIN")?;
        let base_url =
            env::var("VWORLD_BASE_URL").unwrap_or_else(|_| "https://api.vworld.kr".to_owned());
        Ok(Self {
            api_key,
            domain,
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

/// V-World HTTP 클라이언트.
///
/// `reqwest::Client` + `Breaker` + `Policy`. 모든 메서드는 자동으로 timeout +
/// retry + circuit breaking 적용.
#[derive(Debug)]
pub struct VWorldClient {
    http: reqwest::Client,
    config: VWorldConfig,
    breaker: Breaker,
    policy: Policy,
}

impl VWorldClient {
    /// V-World 표준 정책 (`Policy::vworld_default`) 으로 새 클라이언트.
    #[must_use]
    pub fn new(config: VWorldConfig) -> Self {
        Self::with_policy(config, Policy::vworld_default())
    }

    /// 정책 명시 — 테스트 / 특수 케이스용.
    #[must_use]
    pub fn with_policy(config: VWorldConfig, policy: Policy) -> Self {
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

    /// V-World WFS GetFeature 호출 — raw JSON 반환.
    ///
    /// `attrFilter=pnu:=:<pnu>` 로 단일 필지 조회.
    ///
    /// # Errors
    ///
    /// circuit breaker 가 매핑한 [`BreakerError`].
    #[instrument(skip(self), fields(layer = %layer, pnu = %pnu))]
    pub async fn fetch_feature_by_pnu(
        &self,
        layer: &str,
        pnu: &str,
    ) -> Result<Value, BreakerError<reqwest::Error>> {
        let base = &self.config.base_url;
        let key = &self.config.api_key;
        let domain = &self.config.domain;
        let url = format!(
            "{base}/req/data?service=data&request=GetFeature&data={layer}&key={key}&domain={domain}&attrFilter=pnu:=:{pnu}&format=json&size=10&geometry=true&crs=EPSG:4326"
        );

        execute(
            &self.breaker,
            &self.policy,
            "vworld.fetch_feature_by_pnu",
            || async {
                let resp = self.http.get(&url).send().await?;
                resp.error_for_status()?.json::<Value>().await
            },
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn config_from_env_missing_returns_error() {
        // 명시적으로 unset 후 에러 검증 — 다른 테스트와 격리 위해 unique 이름 사용 X.
        // 테스트에서 std::env 변경은 thread-unsafe 라 본 검증은 require_env 직접 호출로.
        let result = require_env("__GONGZZANG_NEVER_SET_ENV__");
        assert!(matches!(result, Err(ConfigError::MissingEnv(_))));
    }

    #[test]
    fn config_error_display() {
        let e = ConfigError::MissingEnv("VWORLD_API_KEY");
        assert_eq!(e.to_string(), "required env var 'VWORLD_API_KEY' not set");
        let e = ConfigError::EmptyEnv("VWORLD_DOMAIN");
        assert_eq!(e.to_string(), "env var 'VWORLD_DOMAIN' is empty");
    }
}
