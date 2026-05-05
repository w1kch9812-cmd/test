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
                // `InvalidSignature` 는 fallback 과 같으므로 명시 안 함 (`clippy::match_same_arms`).
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
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

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

/// 토큰 검증기 — production (`Real(JwtVerifier)`) 또는 dev mock (`Dev`).
///
/// `Dev` 모드는 토큰이 `DEV.<sub>` 형식 시 sub 만 추출해 fake [`Claims`] 반환해요.
/// `JwtVerifier` 의 `RS256`+`JWKS` 검증을 우회하므로 `CI` 빠른 e2e 전용. 실제
/// Zitadel 통합 검증은 별도 sub-project (staging integration test) 에서 처리.
pub enum Verifier {
    /// 진짜 Zitadel `JWT` 검증.
    Real(JwtVerifier),
    /// Dev mock — `DEV.<sub>` 형식 토큰 수용.
    Dev,
}

impl Verifier {
    /// 토큰 검증. `Real` 은 [`JwtVerifier::verify`] 위임, `Dev` 는 `DEV.<sub>` 파싱.
    ///
    /// # Errors
    ///
    /// `Real`: [`JwtVerifier::verify`] 동일.
    /// `Dev`: 형식 불일치 시 [`AuthError::MalformedToken`], `sub` 빈 시 [`AuthError::MissingSubject`].
    pub async fn verify(&self, token: &str) -> Result<Claims, AuthError> {
        match self {
            Self::Real(v) => v.verify(token).await,
            Self::Dev => Self::verify_dev(token),
        }
    }

    fn verify_dev(token: &str) -> Result<Claims, AuthError> {
        let sub = token
            .strip_prefix("DEV.")
            .ok_or(AuthError::MalformedToken)?
            .trim();
        if sub.is_empty() {
            return Err(AuthError::MissingSubject);
        }
        Ok(Claims {
            sub: sub.to_owned(),
            email: Some(format!("{sub}@dev.local")),
            name: Some(sub.to_owned()),
            preferred_username: None,
            jti: format!("dev-{sub}"),
            exp: i64::MAX, // dev 모드는 만료 안 함
            nbf: None,
            iss: "dev-mode".to_owned(),
            aud: crate::claims::Audience::Single("dev-mode".to_owned()),
        })
    }
}

#[cfg(test)]
mod verifier_enum_tests {
    #![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

    use super::*;

    #[tokio::test]
    async fn dev_verify_extracts_sub_and_generates_claims() {
        let v = Verifier::Dev;
        let claims = v.verify("DEV.test-user-1").await.expect("ok");
        assert_eq!(claims.sub, "test-user-1");
        assert_eq!(claims.email.as_deref(), Some("test-user-1@dev.local"));
        assert_eq!(claims.name.as_deref(), Some("test-user-1"));
    }

    #[tokio::test]
    async fn dev_rejects_non_dev_prefix() {
        let v = Verifier::Dev;
        let Err(err) = v.verify("eyJ.something").await else {
            panic!("expected error");
        };
        assert_eq!(err, AuthError::MalformedToken);
    }

    #[tokio::test]
    async fn dev_rejects_empty_sub() {
        let v = Verifier::Dev;
        let Err(err) = v.verify("DEV.").await else {
            panic!("expected error");
        };
        assert_eq!(err, AuthError::MissingSubject);
    }

    #[tokio::test]
    async fn real_dispatches_to_inner_verifier() {
        // Verifier::Real arm 디스패치 자체를 커버 — 토큰 형식 불량으로 즉시 MalformedToken
        let cache = Arc::new(JwksCache::new(
            "http://127.0.0.1:1/jwks".into(),
            reqwest::Client::new(),
        ));
        let inner = JwtVerifier::new("http://issuer".into(), "aud".into(), cache);
        let v = Verifier::Real(inner);
        let Err(err) = v.verify("not-a-jwt").await else {
            panic!("expected error");
        };
        assert_eq!(err, AuthError::MalformedToken);
    }
}
