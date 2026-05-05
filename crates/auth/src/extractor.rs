//! `AuthenticatedUser` extractor — middleware 가 주입한 `Extension` 을 핸들러용으로 노출.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Self>()
            .cloned()
            .ok_or(AuthError::MissingToken)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use axum::http::Request;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use user_domain::entity::{User, UserKind};

    use super::*;
    use crate::claims::{Audience, Claims};

    fn sample_user() -> User {
        User::try_new(
            Id::new(),
            "zitadel-sub-1",
            Email::try_new("u@example.com").expect("email"),
            "User One",
            UserKind::Individual,
            Utc::now(),
        )
        .expect("user")
    }

    fn sample_claims() -> Claims {
        Claims {
            sub: "zitadel-sub-1".to_owned(),
            email: Some("u@example.com".to_owned()),
            name: Some("User One".to_owned()),
            preferred_username: None,
            jti: "j1".to_owned(),
            exp: i64::MAX,
            nbf: None,
            iss: "issuer".to_owned(),
            aud: Audience::Single("aud".to_owned()),
        }
    }

    #[tokio::test]
    async fn extracts_authenticated_user_from_extensions() {
        let injected = AuthenticatedUser {
            user: sample_user(),
            claims: sample_claims(),
        };
        let mut req = Request::builder().body(()).expect("req");
        req.extensions_mut().insert(injected.clone());
        let (mut parts, ()) = req.into_parts();

        let extracted = AuthenticatedUser::from_request_parts(&mut parts, &())
            .await
            .expect("extracted");
        assert_eq!(extracted.claims.sub, injected.claims.sub);
    }

    #[tokio::test]
    async fn missing_extension_returns_missing_token_error() {
        let req = Request::builder().body(()).expect("req");
        let (mut parts, ()) = req.into_parts();

        let err = AuthenticatedUser::from_request_parts(&mut parts, &())
            .await
            .expect_err("must error");
        assert_eq!(err, AuthError::MissingToken);
    }
}
