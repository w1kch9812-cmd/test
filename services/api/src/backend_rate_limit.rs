#[cfg(test)]
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::time::{Duration, Instant};
use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use auth::middleware::AuthenticatedUser;
use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use deadpool_redis::redis;

use crate::http::problem::problem;

const RATE_LUA: &str = r#"
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
redis.call("ZREMRANGEBYSCORE", key, 0, now - window_ms)
local count = redis.call("ZCARD", key)
if count >= limit then
  return {0, 0}
end
local seq = redis.call("INCR", key .. ":seq")
redis.call("EXPIRE", key .. ":seq", math.ceil(window_ms / 1000) + 60)
redis.call("ZADD", key, now, now .. ":" .. seq)
redis.call("PEXPIRE", key, window_ms)
return {1, limit - count - 1}
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendRateKeyStrategy {
    ClientIp,
    SessionSub,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendRatePolicy {
    pub method: &'static str,
    pub path_pattern: &'static str,
    pub key_prefix: &'static str,
    pub key_strategy: BackendRateKeyStrategy,
    pub limit: u32,
    pub window_seconds: u64,
    pub problem_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendRateDecision {
    pub allowed: bool,
    pub remaining: u32,
}

#[derive(Debug, thiserror::Error)]
#[error("backend rate limiter unavailable: {0}")]
pub struct BackendRateLimitError(String);

#[async_trait]
pub trait BackendRateLimiter: Send + Sync {
    async fn check(
        &self,
        key: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<BackendRateDecision, BackendRateLimitError>;
}

#[derive(Clone)]
pub struct BackendRateLimitState {
    limiter: Arc<dyn BackendRateLimiter>,
    policies: Arc<[BackendRatePolicy]>,
}

impl BackendRateLimitState {
    #[must_use]
    pub fn new(
        limiter: Arc<dyn BackendRateLimiter>,
        policies: &'static [BackendRatePolicy],
    ) -> Self {
        Self {
            limiter,
            policies: Arc::from(policies),
        }
    }

    #[cfg(test)]
    fn new_for_tests(
        limiter: Arc<dyn BackendRateLimiter>,
        policies: Vec<BackendRatePolicy>,
    ) -> Self {
        Self {
            limiter,
            policies: Arc::from(policies.into_boxed_slice()),
        }
    }
}

pub struct RedisBackendRateLimiter {
    pool: Arc<deadpool_redis::Pool>,
}

impl RedisBackendRateLimiter {
    #[must_use]
    pub const fn new(pool: Arc<deadpool_redis::Pool>) -> Self {
        Self { pool }
    }
}

pub struct AllowAllBackendRateLimiter;

#[async_trait]
impl BackendRateLimiter for AllowAllBackendRateLimiter {
    async fn check(
        &self,
        _key: &str,
        limit: u32,
        _window_seconds: u64,
    ) -> Result<BackendRateDecision, BackendRateLimitError> {
        Ok(BackendRateDecision {
            allowed: true,
            remaining: limit,
        })
    }
}

#[async_trait]
impl BackendRateLimiter for RedisBackendRateLimiter {
    async fn check(
        &self,
        key: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<BackendRateDecision, BackendRateLimitError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|error| BackendRateLimitError(error.to_string()))?;
        let now_ms = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| BackendRateLimitError(error.to_string()))?
                .as_millis(),
        )
        .map_err(|error| BackendRateLimitError(error.to_string()))?;
        let result: (i64, i64) = redis::cmd("EVAL")
            .arg(RATE_LUA)
            .arg(1)
            .arg(format!("rate:{key}"))
            .arg(now_ms)
            .arg(window_seconds.saturating_mul(1000))
            .arg(limit)
            .query_async(&mut conn)
            .await
            .map_err(|error| BackendRateLimitError(error.to_string()))?;
        Ok(BackendRateDecision {
            allowed: result.0 == 1,
            remaining: u32::try_from(result.1.max(0)).unwrap_or(u32::MAX),
        })
    }
}

pub async fn enforce_backend_rate_limit(
    State(state): State<BackendRateLimitState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let Some(policy) = matching_policy(&state.policies, req.method(), req.uri().path()) else {
        return next.run(req).await;
    };
    let key = resolve_rate_key(policy, &req);
    match state
        .limiter
        .check(&key, policy.limit, policy.window_seconds)
        .await
    {
        Ok(decision) if decision.allowed => next.run(req).await,
        Ok(_) => problem(
            policy.problem_type,
            "요청이 너무 많아요",
            StatusCode::TOO_MANY_REQUESTS,
            Some("잠시 후 다시 시도해 주세요.".to_owned()),
        )
        .into_response(),
        Err(error) => {
            tracing::error!(error = %error, "backend rate limiter failed");
            problem(
                "backend/rate-limit-unavailable",
                "요청 제한을 확인할 수 없어요",
                StatusCode::SERVICE_UNAVAILABLE,
                Some("잠시 후 다시 시도해 주세요.".to_owned()),
            )
            .into_response()
        }
    }
}

fn matching_policy<'a>(
    policies: &'a [BackendRatePolicy],
    method: &Method,
    path: &str,
) -> Option<&'a BackendRatePolicy> {
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

fn resolve_rate_key(policy: &BackendRatePolicy, req: &Request<Body>) -> String {
    let subject = match policy.key_strategy {
        BackendRateKeyStrategy::ClientIp => client_ip(req.headers()),
        BackendRateKeyStrategy::SessionSub => req
            .extensions()
            .get::<AuthenticatedUser>()
            .map_or_else(|| client_ip(req.headers()), |auth| auth.claims.sub.clone()),
    };
    format!("{}:{subject}", policy.key_prefix)
}

fn client_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

#[cfg(test)]
#[derive(Default)]
struct InMemoryBackendRateLimiter {
    entries: Mutex<HashMap<String, Vec<Instant>>>,
    last_key: Mutex<Option<String>>,
}

#[cfg(test)]
impl InMemoryBackendRateLimiter {
    fn last_key(&self) -> Option<String> {
        self.last_key.lock().ok()?.clone()
    }
}

#[cfg(test)]
#[async_trait]
#[allow(clippy::significant_drop_tightening)]
impl BackendRateLimiter for InMemoryBackendRateLimiter {
    async fn check(
        &self,
        key: &str,
        limit: u32,
        window_seconds: u64,
    ) -> Result<BackendRateDecision, BackendRateLimitError> {
        *self
            .last_key
            .lock()
            .map_err(|error| BackendRateLimitError(error.to_string()))? = Some(key.to_owned());
        let limit_as_usize = usize::try_from(limit).unwrap_or(usize::MAX);
        let remaining = {
            let mut entries = self
                .entries
                .lock()
                .map_err(|error| BackendRateLimitError(error.to_string()))?;
            let now = Instant::now();
            let window = Duration::from_secs(window_seconds);
            let hits = entries.entry(key.to_owned()).or_default();
            hits.retain(|hit| now.duration_since(*hit) < window);
            if hits.len() >= limit_as_usize {
                return Ok(BackendRateDecision {
                    allowed: false,
                    remaining: 0,
                });
            }
            hits.push(now);
            limit.saturating_sub(u32::try_from(hits.len()).unwrap_or(u32::MAX))
        };
        Ok(BackendRateDecision {
            allowed: true,
            remaining,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use std::sync::Arc;

    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use axum::middleware;
    use axum::response::Response;
    use axum::routing::get;
    use axum::Router;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use tower::ServiceExt;
    use user_domain::entity::{User, UserKind};

    use super::*;
    use auth::claims::{Audience, Claims};
    use auth::middleware::AuthenticatedUser;

    async fn insert_auth(mut req: Request<Body>, next: middleware::Next) -> Response {
        req.extensions_mut().insert(test_auth("sub-rate-1"));
        next.run(req).await
    }

    fn test_auth(sub: &str) -> AuthenticatedUser {
        let user = User::try_new(
            Id::new(),
            sub,
            Email::try_new("rate@example.com").unwrap(),
            "rate-user",
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap();
        let claims = Claims {
            sub: sub.to_owned(),
            email: Some("rate@example.com".to_owned()),
            name: Some("rate-user".to_owned()),
            preferred_username: None,
            jti: "jti-rate".to_owned(),
            exp: 4_102_444_800,
            nbf: None,
            iss: "issuer".to_owned(),
            aud: Audience::Single("aud".to_owned()),
        };
        AuthenticatedUser { user, claims }
    }

    #[tokio::test]
    async fn middleware_rejects_registered_route_after_policy_limit() {
        let limiter = Arc::new(InMemoryBackendRateLimiter::default());
        let state = BackendRateLimitState::new_for_tests(
            limiter,
            vec![BackendRatePolicy {
                method: "GET",
                path_pattern: "/listings/:id",
                key_prefix: "test:read",
                key_strategy: BackendRateKeyStrategy::ClientIp,
                limit: 1,
                window_seconds: 60,
                problem_type: "backend/too-many-requests",
            }],
        );
        let app = Router::new()
            .route("/listings/:id", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state,
                enforce_backend_rate_limit,
            ));

        let first = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/listings/listing_1")
                    .header("x-forwarded-for", "203.0.113.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);

        let second = app
            .oneshot(
                Request::builder()
                    .uri("/listings/listing_1")
                    .header("x-forwarded-for", "203.0.113.10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
        let body =
            String::from_utf8(to_bytes(second.into_body(), 1024).await.unwrap().to_vec()).unwrap();
        assert!(body.contains("https://gongzzang.com/errors/backend/too-many-requests"));
    }

    #[tokio::test]
    async fn session_sub_key_strategy_uses_authenticated_user_extension() {
        let limiter = Arc::new(InMemoryBackendRateLimiter::default());
        let state = BackendRateLimitState::new_for_tests(
            limiter.clone(),
            vec![BackendRatePolicy {
                method: "GET",
                path_pattern: "/users/me",
                key_prefix: "test:session",
                key_strategy: BackendRateKeyStrategy::SessionSub,
                limit: 10,
                window_seconds: 60,
                problem_type: "backend/too-many-requests",
            }],
        );
        let app = Router::new()
            .route("/users/me", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                state,
                enforce_backend_rate_limit,
            ))
            .layer(middleware::from_fn(insert_auth));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/users/me")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            limiter.last_key().as_deref(),
            Some("test:session:sub-rate-1")
        );
    }
}
