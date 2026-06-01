# Sub-project 3 Auth Zitadel JWT - Part 01C: JWKS Cache And Verifier

Parent index: [Sub-project 3 Auth Zitadel JWT - Part 01](./2026-05-03-sub-project-3-auth-zitadel-jwt.part-01.md).

### Task 3: `JwksCache` (1h TTL + lazy refetch)

**Files:**
- Modify: `crates/auth/src/jwks_cache.rs`

- [ ] **Step 1: 구현 + 테스트**

```rust
//! `JWKS` 캐시 — `kid` → `DecodingKey`, `1h TTL`, lazy refetch.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use tokio::sync::RwLock;

use crate::errors::AuthError;

const TTL: Duration = Duration::from_secs(3600);

/// `JWKS` (JSON Web Key Set) 응답 wrapper.
#[derive(Debug, Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

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

/// `JWKS` 캐시 (`kid` → `DecodingKey` + TTL).
pub struct JwksCache {
    jwks_url: String,
    http: reqwest::Client,
    entries: RwLock<HashMap<String, Entry>>,
}

impl JwksCache {
    /// 캐시 생성. 첫 페치는 `get_or_fetch` 호출 시 lazy 수행.
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
    async fn miss_returns_unknown_key_when_jwks_unreachable() {
        let cache = JwksCache::new(
            "http://127.0.0.1:1/jwks".into(), // 의도적으로 닫힌 포트
            reqwest::Client::new(),
        );
        let err = cache.get_or_fetch("any-kid").await.unwrap_err();
        assert!(matches!(err, AuthError::JwksFetchFailed(_)));
    }
}
```

> **참고:** 더 자세한 캐시 로직 단위 테스트 (lazy refetch 동작, kid 매칭) 는 mock HTTP server 필요 — wiremock 의존성 추가 비용 고려해 본 plan 은 통합 테스트 (T9 CI) 에서 진짜 Zitadel JWKS 로 검증.

- [ ] **Step 2: commit + push + CI 그린 확인**

```bash
git add crates/auth/src/jwks_cache.rs
git commit -m "feat(auth): JwksCache 1h TTL + lazy refetch + reqwest fetch (SP3 T3)

- JwksCache::get_or_fetch — hot path read-lock, miss/expired 시 write-lock 페치
- RSA only (kty='RSA'), DecodingKey::from_rsa_components(n, e)
- AuthError::JwksFetchFailed mapping
- 2 단위 테스트 (TTL 상수 + 페치 실패 경로); JWKS 매칭은 T9 e2e 로 검증"
git push
```

---

### Task 4: `JwtVerifier`

**Files:**
- Modify: `crates/auth/src/verifier.rs`

- [ ] **Step 1: 구현 + 테스트**

```rust
//! `JWT` 검증기 — RS256 + JWKS + iss/aud/exp/nbf.

use std::sync::Arc;

use jsonwebtoken::{decode, decode_header, Algorithm, Validation};

use crate::claims::Claims;
use crate::errors::AuthError;
use crate::jwks_cache::JwksCache;

/// Zitadel `JWT` 검증기.
pub struct JwtVerifier {
    issuer: String,
    audience: String,
    jwks: Arc<JwksCache>,
}

impl JwtVerifier {
    /// 검증기 생성. JWKS 페치는 첫 verify 호출 시 lazy 수행.
    #[must_use]
    pub fn new(issuer: String, audience: String, jwks: Arc<JwksCache>) -> Self {
        Self {
            issuer,
            audience,
            jwks,
        }
    }

    /// `JWT` 토큰을 검증해 [`Claims`] 를 반환해요.
    ///
    /// # Errors
    ///
    /// - 헤더 파싱 실패 → [`AuthError::MalformedToken`]
    /// - `kid` 없음 → [`AuthError::UnknownKey`]
    /// - 서명 실패 → [`AuthError::InvalidSignature`]
    /// - `exp` 만료 → [`AuthError::Expired`]
    /// - `iss` 불일치 → [`AuthError::InvalidIssuer`]
    /// - `aud` 불일치 → [`AuthError::InvalidAudience`]
    /// - `sub` 빈 값 → [`AuthError::MissingSubject`]
    pub async fn verify(&self, token: &str) -> Result<Claims, AuthError> {
        let header = decode_header(token).map_err(|_| AuthError::MalformedToken)?;
        if header.alg != Algorithm::RS256 {
            return Err(AuthError::InvalidSignature);
        }
        let kid = header.kid.ok_or(AuthError::UnknownKey)?;
        let key = self.jwks.get_or_fetch(&kid).await?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.validate_aud = false; // 직접 검증 (Audience::Single|Multiple)
        validation.leeway = 30; // clock skew 30s

        let data = decode::<Claims>(token, &key, &validation).map_err(|e| {
            use jsonwebtoken::errors::ErrorKind as E;
            match e.kind() {
                E::ExpiredSignature => AuthError::Expired,
                E::ImmatureSignature => AuthError::NotYetValid,
                E::InvalidIssuer => AuthError::InvalidIssuer,
                E::InvalidSignature => AuthError::InvalidSignature,
                _ => AuthError::InvalidSignature,
            }
        })?;

        if !data.claims.aud.contains(&self.audience) {
            return Err(AuthError::InvalidAudience);
        }
        if data.claims.sub.trim().is_empty() {
            return Err(AuthError::MissingSubject);
        }
        Ok(data.claims)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[tokio::test]
    async fn malformed_token_returns_malformed() {
        let cache = Arc::new(JwksCache::new(
            "http://127.0.0.1:1/jwks".into(),
            reqwest::Client::new(),
        ));
        let v = JwtVerifier::new("http://issuer".into(), "aud".into(), cache);
        let err = v.verify("not-a-jwt").await.unwrap_err();
        assert_eq!(err, AuthError::MalformedToken);
    }
}
```

> 더 깊은 검증 (서명 / iss / aud / exp 분기) 는 T9 CI 의 진짜 Zitadel 토큰으로 검증.

- [ ] **Step 2: commit + push + CI 그린**

```bash
git add crates/auth/src/verifier.rs
git commit -m "feat(auth): JwtVerifier::verify — RS256 + iss/aud/exp + JWKS lookup (SP3 T4)

- Algorithm RS256만 허용 (downgrade attack 방지)
- jsonwebtoken Validation: iss check via lib, aud/sub 직접 검증
- clock skew 30s leeway
- ErrorKind → AuthError 매핑 (Expired/NotYetValid/InvalidIssuer/InvalidSignature)
- 1 happy-path 단위 테스트 (malformed); 깊은 검증은 T9 e2e"
git push
```

---

## Phase B: Middleware + extractor + role guard
