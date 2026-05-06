//! HTTP 요청 → `MutationContext` 구축 — SP6-iv `http_user_action` 헬퍼 +
//! SP-Obs T3 `MutationContextBuilder` extractor.
//!
//! `MutationContextBuilder` 가 SSS 답: `auth + correlation + client_ip +
//! user_agent` 모두 자동 채움 — handler 가 잊을 수 없음 (§ 2 자동강제).

use async_trait::async_trait;
use auth::middleware::AuthenticatedUser;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use shared_kernel::mutation::MutationContext;
use ulid::Ulid;

use crate::http::problem::{problem, ProblemResponse};
use crate::http::request_id::RequestId;

/// HTTP `User-Agent` 헤더 도메인 합리적 상한 (SP-Obs T3). DB `audit_log.user_agent`
/// 는 `text` (무제한) 라 상한이 도메인 정책 — 추적 가치 vs storage 트레이드오프.
const UA_MAX_LEN: usize = 500;

/// HTTP 요청 → `MutationContext::new_user_action(actor_id, correlation_id, action)`.
///
/// `actor_id` = `auth.user.id` (인증 통과한 사용자 ID).
/// `correlation_id` = `cor_<ULID>` 자동 생성.
///
/// **SP-Obs T3**: 신규 `MutationContextBuilder` extractor 가 더 풍부한 컨텍스트
/// (X-Request-Id correlation + ip + ua) 자동 채움 — 신규 핸들러는 그쪽 권장.
/// 본 헬퍼는 backward compat 유지 (기존 핸들러가 사용 중).
#[must_use]
#[allow(dead_code)]
pub fn http_user_action(auth: &AuthenticatedUser, action: &str) -> MutationContext {
    let cor_id = format!("cor_{}", Ulid::new());
    MutationContext::new_user_action(auth.user.id.clone(), cor_id, action)
}

/// HTTP 요청 → `MutationContext` 구축 extractor (SP-Obs T3).
///
/// 핸들러 1 줄로 `auth + correlation + client_ip + user_agent` 자동 채움:
///
/// ```ignore
/// pub async fn create_listing(
///     State(state): State<ListingsState>,
///     builder: MutationContextBuilder,
///     Json(body): Json<CreateListingRequest>,
/// ) -> Result<...> {
///     let ctx = builder.build("create_listing");
///     repo.save(&listing, ctx).await?;
/// }
/// ```
///
/// 추출 우선순위:
/// - `actor_id` = `Extension<AuthenticatedUser>` (`auth_layer` 통과 시 항상 존재)
/// - `correlation_id` = `Extension<RequestId>` (`request_id_layer` 가 항상 set),
///   fallback `cor_<ULID>` (테스트 / extractor 단독 시나리오)
/// - `client_ip` = `X-Forwarded-For` 첫 IP (production proxy `ALB`/Cloudflare 환경)
/// - `user_agent` = `User-Agent` header, ≤500자 trim
#[derive(Debug, Clone)]
#[allow(dead_code)] // 기존 핸들러 미마이그 — adoption 은 점진적 (FU 이후 핸들러부터).
pub struct MutationContextBuilder {
    /// 인증된 사용자 — `actor_id` source.
    pub auth: AuthenticatedUser,
    /// X-Request-Id 또는 fallback ULID — DB `correlation_id` 컬럼 source.
    pub correlation_id: String,
    /// X-Forwarded-For 첫 IP (있으면) — DB `audit_log.ip_address` source.
    pub client_ip: Option<String>,
    /// User-Agent header ≤500자 — DB `audit_log.user_agent` source.
    pub user_agent: Option<String>,
}

impl MutationContextBuilder {
    /// `action` 라벨 부여해 `MutationContext` 빌드.
    #[must_use]
    #[allow(dead_code)]
    pub fn build(self, action: &str) -> MutationContext {
        MutationContext::new_user_action(self.auth.user.id, self.correlation_id, action)
            .with_client_info_optional(self.client_ip, self.user_agent)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for MutationContextBuilder
where
    S: Send + Sync,
{
    type Rejection = ProblemResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth = parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or_else(|| {
                problem(
                    "auth/required",
                    "인증이 필요해요",
                    StatusCode::UNAUTHORIZED,
                    None,
                )
            })?;

        let correlation_id = parts
            .extensions
            .get::<RequestId>()
            .map_or_else(|| format!("cor_{}", Ulid::new()), |r| r.as_str().to_owned());

        let client_ip = extract_client_ip(parts);
        let user_agent = extract_user_agent(parts);

        Ok(Self {
            auth,
            correlation_id,
            client_ip,
            user_agent,
        })
    }
}

/// `X-Forwarded-For` 첫 IP 추출. `1.2.3.4, 5.6.7.8` 형태에서 `1.2.3.4` 선택.
///
/// **production trust boundary 주의**: ALB / Cloudflare 가 *마지막 hop* 만
/// trusted 이라면 first IP 가 client (정상). 직접 노출 환경에서는 스푸핑 가능 —
/// SP8 `IaC` 단계에서 trusted proxy hop count 명시 (1차 = first IP 채택).
fn extract_client_ip(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// `User-Agent` header → ≤500자 trim. 한글 / multibyte 안전 (chars iter).
fn extract_user_agent(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.chars().take(UA_MAX_LEN).collect::<String>())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use auth::claims::{Audience, Claims};
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use user_domain::entity::{User, UserKind};

    fn fixture_auth() -> AuthenticatedUser {
        let email = Email::try_new("a@b.com").expect("email");
        let user = User::try_new_full(
            Id::new(),
            "sub-1",
            email,
            None,
            "alice",
            UserKind::Individual,
            None,
            None,
            None,
            None,
            vec![],
            None,
            None,
            Utc::now(),
        )
        .expect("user");
        let claims = Claims {
            sub: "sub-1".into(),
            email: Some("a@b.com".into()),
            name: Some("alice".into()),
            preferred_username: None,
            jti: "j1".into(),
            exp: 0,
            nbf: None,
            iss: "i".into(),
            aud: Audience::Single("a".into()),
        };
        AuthenticatedUser { user, claims }
    }

    #[test]
    fn http_user_action_sets_actor_to_user_id() {
        let auth = fixture_auth();
        let user_id = auth.user.id.clone();
        let ctx = http_user_action(&auth, "create_listing");
        assert_eq!(
            ctx.actor_id.as_ref().map(Id::as_str),
            Some(user_id.as_str())
        );
        assert_eq!(ctx.action, "create_listing");
    }

    #[test]
    fn http_user_action_generates_cor_prefix_correlation_id() {
        let auth = fixture_auth();
        let ctx = http_user_action(&auth, "submit_for_review");
        assert!(
            ctx.correlation_id.starts_with("cor_"),
            "expected cor_ prefix, got: {}",
            ctx.correlation_id
        );
        // ULID = 26 chars, plus 4 char prefix = 30.
        assert_eq!(ctx.correlation_id.len(), 30);
    }

    #[test]
    fn http_user_action_unique_correlation_per_call() {
        let auth = fixture_auth();
        let a = http_user_action(&auth, "create_listing");
        let b = http_user_action(&auth, "create_listing");
        assert_ne!(a.correlation_id, b.correlation_id);
    }

    // ── MutationContextBuilder extractor (SP-Obs T3) ──────────────────────

    use axum::http::{HeaderMap, HeaderValue, Method, Request};

    fn parts_with_headers(headers: HeaderMap) -> axum::http::request::Parts {
        let mut req = Request::new(());
        *req.headers_mut() = headers;
        *req.method_mut() = Method::POST;
        req.into_parts().0
    }

    #[test]
    fn extract_client_ip_picks_first_xff() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("1.2.3.4, 5.6.7.8"),
        );
        let parts = parts_with_headers(headers);
        assert_eq!(extract_client_ip(&parts), Some("1.2.3.4".to_owned()));
    }

    #[test]
    fn extract_client_ip_returns_none_when_missing() {
        let parts = parts_with_headers(HeaderMap::new());
        assert_eq!(extract_client_ip(&parts), None);
    }

    #[test]
    fn extract_client_ip_returns_none_when_empty() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static(""));
        let parts = parts_with_headers(headers);
        assert_eq!(extract_client_ip(&parts), None);
    }

    #[test]
    fn extract_user_agent_trims_to_max_len() {
        // 1000 char UA → 500 char trim. multibyte 안전 (한글 한자 = 1 char).
        let long = "X".repeat(UA_MAX_LEN + 100);
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_str(&long).unwrap());
        let parts = parts_with_headers(headers);
        let ua = extract_user_agent(&parts).expect("ua");
        assert_eq!(ua.chars().count(), UA_MAX_LEN);
    }

    #[test]
    fn extract_user_agent_returns_none_when_missing() {
        let parts = parts_with_headers(HeaderMap::new());
        assert_eq!(extract_user_agent(&parts), None);
    }

    #[test]
    fn extract_user_agent_returns_none_when_empty() {
        let mut headers = HeaderMap::new();
        headers.insert("user-agent", HeaderValue::from_static(""));
        let parts = parts_with_headers(headers);
        assert_eq!(extract_user_agent(&parts), None);
    }
}
