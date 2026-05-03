//! `JWKS` 캐시 — `kid` → `DecodingKey`, `1h TTL`, lazy refetch.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::AuthError;

const TTL: Duration = Duration::from_secs(3600);

/// `JWKS` (`JSON Web Key Set`) 응답 wrapper.
#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

/// `JWKS` 내부 단일 키 (`RSA` 만 지원).
#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    #[serde(default)]
    alg: Option<String>,
    #[allow(dead_code)]
    #[serde(default, rename = "use")]
    use_: Option<String>,
}

/// 캐시 entry — 키 + 페치 시각.
struct Entry {
    key: Arc<DecodingKey>,
    fetched_at: Instant,
}

/// `JWKS` 캐시 (`kid` → `DecodingKey` + `1h TTL`).
pub struct JwksCache {
    jwks_url: String,
    http: reqwest::Client,
    entries: RwLock<HashMap<String, Entry>>,
}

impl JwksCache {
    /// 캐시 생성. 첫 페치는 [`Self::get_or_fetch`] 호출 시 lazy 수행.
    #[must_use]
    pub fn new(jwks_url: String, http: reqwest::Client) -> Self {
        Self {
            jwks_url,
            http,
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// `kid` 로 키 조회 — 캐시 만료 또는 미존재 시 `JWKS` 재페치.
    ///
    /// # Errors
    ///
    /// 페치 실패 → [`AuthError::JwksFetchFailed`].
    /// 페치 후에도 `kid` 없으면 [`AuthError::UnknownKey`].
    pub async fn get_or_fetch(&self, kid: &str) -> Result<Arc<DecodingKey>, AuthError> {
        // hot path: 캐시 hit + TTL 살아있음
        {
            let entries = self.entries.read().await;
            if let Some(entry) = entries.get(kid) {
                if entry.fetched_at.elapsed() < TTL {
                    return Ok(entry.key.clone());
                }
            }
        }
        // miss or expired — 페치
        self.refetch().await?;
        let entries = self.entries.read().await;
        entries
            .get(kid)
            .map(|e| e.key.clone())
            .ok_or(AuthError::UnknownKey)
    }

    async fn refetch(&self) -> Result<(), AuthError> {
        let resp = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
        let jwks: Jwks = resp
            .json()
            .await
            .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
        let mut entries = self.entries.write().await;
        let now = Instant::now();
        entries.clear();
        for k in jwks.keys {
            if k.kty != "RSA" {
                continue;
            }
            let key = DecodingKey::from_rsa_components(&k.n, &k.e)
                .map_err(|e| AuthError::JwksFetchFailed(e.to_string()))?;
            entries.insert(
                k.kid,
                Entry {
                    key: Arc::new(key),
                    fetched_at: now,
                },
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn ttl_one_hour() {
        assert_eq!(TTL.as_secs(), 3600);
    }

    #[tokio::test]
    async fn fetch_failure_when_jwks_unreachable() {
        let cache = JwksCache::new(
            "http://127.0.0.1:1/jwks".into(), // 의도적으로 닫힌 포트
            reqwest::Client::new(),
        );
        let err = cache.get_or_fetch("any-kid").await.unwrap_err();
        assert!(matches!(err, AuthError::JwksFetchFailed(_)));
    }

    #[tokio::test]
    async fn fetch_failure_with_invalid_url_scheme() {
        let cache = JwksCache::new("not-a-url".into(), reqwest::Client::new());
        let err = cache.get_or_fetch("any-kid").await.unwrap_err();
        assert!(matches!(err, AuthError::JwksFetchFailed(_)));
    }
}
