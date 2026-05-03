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
