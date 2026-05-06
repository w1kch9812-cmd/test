//! 역할 가드 — `require_role` helper.

use user_domain::entity::UserRole;

use crate::errors::AuthError;
use crate::middleware::AuthenticatedUser;

/// `auth.user.roles` 가 `role` 을 포함하는지 확인.
///
/// # Errors
///
/// 미포함 → [`AuthError::InsufficientRole`].
pub fn require_role(auth: &AuthenticatedUser, role: UserRole) -> Result<(), AuthError> {
    if auth.user.roles.contains(&role) {
        Ok(())
    } else {
        Err(AuthError::InsufficientRole)
    }
}

/// `auth.user.roles` 가 `roles` 중 *하나라도* 포함하는지 확인 (OR 매칭, SP6-v).
///
/// admin 또는 operator 등 *복수 권한 OR* 가 필요할 때 사용. 모두 통과 (AND)
/// 가 필요하면 `require_role` 을 N번 호출.
///
/// # Errors
///
/// 어느 것도 미포함 → [`AuthError::InsufficientRole`].
pub fn require_one_of_roles(auth: &AuthenticatedUser, roles: &[UserRole]) -> Result<(), AuthError> {
    if roles.iter().any(|r| auth.user.roles.contains(r)) {
        Ok(())
    } else {
        Err(AuthError::InsufficientRole)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Utc;
    use shared_kernel::email::Email;
    use shared_kernel::id::Id;
    use user_domain::entity::{User, UserKind, UserRole};

    use crate::claims::{Audience, Claims};

    fn fixture(roles: Vec<UserRole>) -> AuthenticatedUser {
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
            roles,
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
    fn allows_when_role_present() {
        let auth = fixture(vec![UserRole::Buyer]);
        assert!(require_role(&auth, UserRole::Buyer).is_ok());
    }

    #[test]
    fn denies_when_role_missing() {
        let auth = fixture(vec![UserRole::Buyer]);
        let err = require_role(&auth, UserRole::Admin).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }

    #[test]
    fn denies_when_no_roles() {
        let auth = fixture(vec![]);
        let err = require_role(&auth, UserRole::Buyer).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }

    #[test]
    fn allows_one_of_multiple_roles() {
        let auth = fixture(vec![UserRole::Buyer, UserRole::Seller, UserRole::Broker]);
        assert!(require_role(&auth, UserRole::Seller).is_ok());
    }

    // ── require_one_of_roles (SP6-v) ────────────────────────────────────

    #[test]
    fn require_one_of_roles_allows_when_any_matches() {
        let auth = fixture(vec![UserRole::Operator]);
        assert!(require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).is_ok());
    }

    #[test]
    fn require_one_of_roles_allows_first_match() {
        let auth = fixture(vec![UserRole::Admin]);
        assert!(require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).is_ok());
    }

    #[test]
    fn require_one_of_roles_denies_when_none_match() {
        let auth = fixture(vec![UserRole::Buyer]);
        let err = require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }

    #[test]
    fn require_one_of_roles_denies_with_no_user_roles() {
        let auth = fixture(vec![]);
        let err = require_one_of_roles(&auth, &[UserRole::Admin, UserRole::Operator]).unwrap_err();
        assert_eq!(err, AuthError::InsufficientRole);
    }
}
