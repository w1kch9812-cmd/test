//! HTTP 요청 → `MutationContext::new_user_action` helper.
//!
//! 모든 mutation handler 의 1줄 helper. `correlation_id` 는 axum extension 의
//! `X-Request-Id` (있으면) 또는 자동 ULID. SP7 관측성에서 X-Request-Id 미들웨어
//! 추가 예정 — 현재는 `cor_<ULID>` 자동.

use auth::middleware::AuthenticatedUser;
use shared_kernel::mutation::MutationContext;
use ulid::Ulid;

/// HTTP 요청 → `MutationContext::new_user_action(actor_id, correlation_id, action)`.
///
/// `actor_id` = `auth.user.id` (인증 통과한 사용자 ID).
/// `correlation_id` = `cor_<ULID>` 자동 생성. 후속 (SP7) 에서 `X-Request-Id`
/// 헤더 propagate.
#[must_use]
#[allow(dead_code)] // T4 의 POST/PATCH 핸들러 가 wire-up.
pub fn http_user_action(auth: &AuthenticatedUser, action: &str) -> MutationContext {
    let cor_id = format!("cor_{}", Ulid::new());
    MutationContext::new_user_action(auth.user.id.clone(), cor_id, action)
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
}
