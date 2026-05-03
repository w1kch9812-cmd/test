//! `JWT` 검증기 — `RS256` + `JWKS` + `iss`/`aud`/`exp`/`nbf`.

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
    /// 검증기 생성. `JWKS` 페치는 첫 verify 호출 시 lazy 수행.
    #[must_use]
    pub const fn new(issuer: String, audience: String, jwks: Arc<JwksCache>) -> Self {
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
    /// - `nbf` 미도래 → [`AuthError::NotYetValid`]
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
        validation.validate_aud = false; // 직접 검증 (`Audience::Single|Multiple`)
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

    fn fixture() -> JwtVerifier {
        let cache = Arc::new(JwksCache::new(
            "http://127.0.0.1:1/jwks".into(),
            reqwest::Client::new(),
        ));
        JwtVerifier::new("http://issuer".into(), "aud".into(), cache)
    }

    fn expect_err(r: Result<Claims, AuthError>) -> AuthError {
        match r {
            Ok(_) => panic!("expected error, got Ok"),
            Err(e) => e,
        }
    }

    #[tokio::test]
    async fn malformed_token_returns_malformed() {
        let v = fixture();
        let err = expect_err(v.verify("not-a-jwt").await);
        assert_eq!(err, AuthError::MalformedToken);
    }

    #[tokio::test]
    async fn empty_token_returns_malformed() {
        let v = fixture();
        let err = expect_err(v.verify("").await);
        assert_eq!(err, AuthError::MalformedToken);
    }

    #[tokio::test]
    async fn random_string_returns_malformed() {
        let v = fixture();
        let err = expect_err(v.verify("aaaa.bbbb.cccc").await);
        // base64 가 아닌 random string 도 MalformedToken
        assert_eq!(err, AuthError::MalformedToken);
    }
}
