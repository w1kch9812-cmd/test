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
}
