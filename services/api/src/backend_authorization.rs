use std::sync::Arc;

use auth::errors::AuthError;
use auth::middleware::AuthenticatedUser;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::Method;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use user_domain::entity::UserRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendRolePolicy {
    pub method: &'static str,
    pub path_pattern: &'static str,
    pub required_roles: &'static [UserRole],
}

#[derive(Clone)]
pub struct BackendAuthorizationState {
    policies: Arc<[BackendRolePolicy]>,
}

impl BackendAuthorizationState {
    #[must_use]
    pub fn new(policies: &'static [BackendRolePolicy]) -> Self {
        Self {
            policies: Arc::from(policies),
        }
    }

    #[cfg(test)]
    fn new_for_tests(policies: Vec<BackendRolePolicy>) -> Self {
        Self {
            policies: Arc::from(policies.into_boxed_slice()),
        }
    }
}

pub async fn enforce_backend_roles(
    State(state): State<BackendAuthorizationState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(policy) = matching_policy(&state.policies, req.method(), req.uri().path()) else {
        return next.run(req).await;
    };
    if policy.required_roles.is_empty() {
        return next.run(req).await;
    }
    let Some(auth) = req.extensions().get::<AuthenticatedUser>() else {
        return AuthError::MissingToken.into_response();
    };
    if policy
        .required_roles
        .iter()
        .any(|role| auth.user.roles.contains(role))
    {
        next.run(req).await
    } else {
        AuthError::InsufficientRole.into_response()
    }
}

fn matching_policy<'a>(
    policies: &'a [BackendRolePolicy],
    method: &Method,
    path: &str,
) -> Option<&'a BackendRolePolicy> {
    policies.iter().find(|policy| {
        policy.method == method.as_str() && matches_template_path(policy.path_pattern, path)
    })
}

fn matches_template_path(template: &str, path: &str) -> bool {
    let template_segments: Vec<_> = template.trim_matches('/').split('/').collect();
    let path_segments: Vec<_> = path.trim_matches('/').split('/').collect();
    if template_segments.len() != path_segments.len() {
        return false;
    }

    template_segments
        .iter()
        .zip(path_segments.iter())
        .all(|(template_segment, path_segment)| {
            template_segment.starts_with(':')
                || (template_segment.starts_with('{') && template_segment.ends_with('}'))
                || template_segment == path_segment
        })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use auth::claims::{Audience, Claims};
    use auth::middleware::AuthenticatedUser;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::routing::post;
    use axum::Router;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use tower::ServiceExt;
    use user_domain::entity::{User, UserKind, UserRole};

    use super::*;

    async fn insert_auth(
        mut req: Request<Body>,
        next: middleware::Next,
    ) -> axum::response::Response {
        req.extensions_mut()
            .insert(test_auth(vec![UserRole::Buyer]));
        next.run(req).await
    }

    fn test_auth(roles: Vec<UserRole>) -> AuthenticatedUser {
        let user = User::try_new_full(
            Id::new(),
            "sub-role-1",
            Email::try_new("role@example.com").unwrap(),
            None,
            "role-user",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            roles,
            None,
            None,
            Utc::now(),
        )
        .unwrap();
        let claims = Claims {
            sub: "sub-role-1".to_owned(),
            email: Some("role@example.com".to_owned()),
            name: Some("role-user".to_owned()),
            preferred_username: None,
            jti: "jti-role".to_owned(),
            exp: 4_102_444_800,
            nbf: None,
            iss: "issuer".to_owned(),
            aud: Audience::Single("aud".to_owned()),
        };
        AuthenticatedUser { user, claims }
    }

    #[tokio::test]
    async fn middleware_rejects_matching_route_when_required_role_missing() {
        let state = BackendAuthorizationState::new_for_tests(vec![BackendRolePolicy {
            method: "POST",
            path_pattern: "/listings",
            required_roles: &[UserRole::Broker],
        }]);
        let app = Router::new()
            .route("/listings", post(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, enforce_backend_roles))
            .layer(middleware::from_fn(insert_auth));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/listings")
                    .method("POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
