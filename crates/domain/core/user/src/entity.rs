//! `User` Aggregate struct + `try_new`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::email::Email;
use shared_kernel::id::{Id, UserMarker};

use crate::errors::UserError;

/// 사용자 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserKind {
    /// 개인.
    Individual,
    /// 법인.
    Corporation,
}

/// `User` Aggregate (Walking Skeleton 최소 필드).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// `usr_<26-char ULID>` 형식 `ID`.
    pub id: Id<UserMarker>,
    /// `Zitadel` `JWT` `sub` claim. 비어있지 않고 ≤255자.
    pub zitadel_sub: String,
    /// 이메일.
    pub email: Email,
    /// 표시 이름. 비어있지 않고 ≤100자.
    pub display_name: String,
    /// 사용자 종류 (개인/법인).
    pub user_kind: UserKind,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
    /// 마지막 갱신 시각.
    pub updated_at: DateTime<Utc>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl User {
    /// 검증 후 새 `User` 생성. `created_at == updated_at == now`, `version = 1`.
    ///
    /// # Errors
    ///
    /// `display_name` 빈 문자열 → [`UserError::EmptyDisplayName`].
    /// 100자 초과 → [`UserError::DisplayNameTooLong`].
    /// `zitadel_sub` 빈 → [`UserError::EmptyZitadelSub`].
    /// 255자 초과 → [`UserError::ZitadelSubTooLong`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<UserMarker>,
        zitadel_sub: String,
        email: Email,
        display_name: String,
        user_kind: UserKind,
        now: DateTime<Utc>,
    ) -> Result<Self, UserError> {
        let display_name = display_name.trim().to_owned();
        if display_name.is_empty() {
            return Err(UserError::EmptyDisplayName);
        }
        let display_len = display_name.chars().count();
        if display_len > 100 {
            return Err(UserError::DisplayNameTooLong {
                actual: display_len,
            });
        }

        let zitadel_sub = zitadel_sub.trim().to_owned();
        if zitadel_sub.is_empty() {
            return Err(UserError::EmptyZitadelSub);
        }
        let sub_len = zitadel_sub.chars().count();
        if sub_len > 255 {
            return Err(UserError::ZitadelSubTooLong { actual: sub_len });
        }

        Ok(Self {
            id,
            zitadel_sub,
            email,
            display_name,
            user_kind,
            created_at: now,
            updated_at: now,
            version: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn sample_email() -> Email {
        Email::try_new("alice@example.com").expect("valid")
    }

    #[test]
    fn try_new_happy_path() {
        let id = Id::<UserMarker>::new();
        let now = Utc::now();
        let user = User::try_new(
            id.clone(),
            "zitadel-sub-1".to_owned(),
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            now,
        )
        .expect("valid");
        assert_eq!(user.id, id);
        assert_eq!(user.display_name, "Alice");
        assert_eq!(user.version, 1);
        assert_eq!(user.created_at, now);
        assert_eq!(user.updated_at, now);
    }

    #[test]
    fn rejects_empty_display_name() {
        let err = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            String::new(),
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyDisplayName));
    }

    #[test]
    fn rejects_whitespace_only_display_name() {
        let err = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            "   ".to_owned(),
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyDisplayName));
    }

    #[test]
    fn rejects_too_long_display_name() {
        let long = "X".repeat(101);
        let err = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            long,
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::DisplayNameTooLong { actual: 101 }));
    }

    #[test]
    fn accepts_exactly_100_char_display_name() {
        let exactly = "X".repeat(100);
        let user = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            exactly.clone(),
            UserKind::Individual,
            Utc::now(),
        )
        .expect("100 chars OK");
        assert_eq!(user.display_name, exactly);
    }

    #[test]
    fn rejects_empty_zitadel_sub() {
        let err = User::try_new(
            Id::new(),
            String::new(),
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyZitadelSub));
    }

    #[test]
    fn rejects_whitespace_only_zitadel_sub() {
        let err = User::try_new(
            Id::new(),
            "   ".to_owned(),
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::EmptyZitadelSub));
    }

    #[test]
    fn rejects_too_long_zitadel_sub() {
        let long = "X".repeat(256);
        let err = User::try_new(
            Id::new(),
            long,
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, UserError::ZitadelSubTooLong { actual: 256 }));
    }

    #[test]
    fn accepts_exactly_255_char_zitadel_sub() {
        let exactly = "X".repeat(255);
        let user = User::try_new(
            Id::new(),
            exactly.clone(),
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            Utc::now(),
        )
        .expect("255 chars OK");
        assert_eq!(user.zitadel_sub, exactly);
    }

    #[test]
    fn user_kind_serializes_snake_case() {
        let json = serde_json::to_string(&UserKind::Individual).expect("ok");
        assert_eq!(json, r#""individual""#);
        let json = serde_json::to_string(&UserKind::Corporation).expect("ok");
        assert_eq!(json, r#""corporation""#);
    }

    #[test]
    fn user_serde_roundtrip() {
        let now = Utc::now();
        let user = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            "Alice".to_owned(),
            UserKind::Individual,
            now,
        )
        .expect("valid");
        let json = serde_json::to_string(&user).expect("serialize");
        let back: User = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(user, back);
    }

    #[test]
    fn corporation_user_kind_works() {
        let user = User::try_new(
            Id::new(),
            "sub".to_owned(),
            sample_email(),
            "Acme Co.".to_owned(),
            UserKind::Corporation,
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(user.user_kind, UserKind::Corporation);
    }
}
