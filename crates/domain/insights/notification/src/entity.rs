//! `Notification` Aggregate (append-mostly + 멱등 `mark_read`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, NotificationMarker, UserMarker};

use crate::errors::NotificationError;

/// `kind` 최대 길이 (spec § 5.2 `varchar(50)`).
const MAX_KIND_LEN: usize = 50;

/// 사용자 알림 1건. append-mostly (이벤트 발생 시 INSERT).
///
/// `mark_read`는 멱등 — 이미 읽은 알림 재호출 시 `read_at` 보존.
/// `payload`는 `serde_json::Value`라 `Eq`를 derive 할 수 없어요 (`PartialEq`만).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Notification {
    /// 식별자 (`ntf_<26 ULID>`).
    pub id: Id<NotificationMarker>,
    /// 수신자.
    pub user_id: Id<UserMarker>,
    /// 알림 종류 (≤50자, 비어있지 않음).
    /// 예: `bookmark_listing_changed`, `auction_deadline_approaching`.
    pub kind: String,
    /// 이벤트 컨텍스트 (`JSONB`).
    pub payload: serde_json::Value,
    /// 읽음 시각. `None` = 미읽음.
    pub read_at: Option<DateTime<Utc>>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl Notification {
    /// 검증 후 생성. `read_at = None`, `created_at = now`.
    ///
    /// # Errors
    ///
    /// - `kind` 빈 (trim 후) → [`NotificationError::EmptyKind`].
    /// - `kind` 50자 초과 → [`NotificationError::KindTooLong`].
    pub fn try_new(
        id: Id<NotificationMarker>,
        user_id: Id<UserMarker>,
        kind: &str,
        payload: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Result<Self, NotificationError> {
        let kind = kind.trim().to_owned();
        if kind.is_empty() {
            return Err(NotificationError::EmptyKind);
        }
        if kind.chars().count() > MAX_KIND_LEN {
            return Err(NotificationError::KindTooLong {
                actual: kind.chars().count(),
            });
        }
        Ok(Self {
            id,
            user_id,
            kind,
            payload,
            read_at: None,
            created_at: now,
        })
    }

    /// 읽음 처리 — 멱등. 이미 읽은 경우 `read_at`를 보존해요.
    ///
    /// append-mostly 도메인이라 `version` bump 없어요.
    pub fn mark_read(&mut self, at: DateTime<Utc>) {
        if self.read_at.is_none() {
            self.read_at = Some(at);
        }
    }

    /// 읽음 여부.
    #[must_use]
    pub const fn is_read(&self) -> bool {
        self.read_at.is_some()
    }

    /// 안 읽음 여부.
    #[must_use]
    pub const fn is_unread(&self) -> bool {
        self.read_at.is_none()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    fn sample_payload() -> serde_json::Value {
        serde_json::json!({"listing_id": "lst_01HXY3NK0Z9F6S1B2C3D4E5F6G", "delta": "price"})
    }

    #[test]
    fn happy_path_with_sample_kind() {
        let now = Utc::now();
        let n = Notification::try_new(
            Id::new(),
            Id::new(),
            "bookmark_listing_changed",
            sample_payload(),
            now,
        )
        .expect("valid");
        assert_eq!(n.kind, "bookmark_listing_changed");
        assert!(n.read_at.is_none());
        assert_eq!(n.created_at, now);
    }

    #[test]
    fn rejects_empty_kind() {
        let err = Notification::try_new(Id::new(), Id::new(), "", sample_payload(), Utc::now())
            .unwrap_err();
        assert!(matches!(err, NotificationError::EmptyKind));
    }

    #[test]
    fn rejects_whitespace_only_kind() {
        let err = Notification::try_new(Id::new(), Id::new(), "    ", sample_payload(), Utc::now())
            .unwrap_err();
        assert!(matches!(err, NotificationError::EmptyKind));
    }

    #[test]
    fn rejects_kind_over_50_chars() {
        let long = "X".repeat(51);
        let err = Notification::try_new(Id::new(), Id::new(), &long, sample_payload(), Utc::now())
            .unwrap_err();
        assert!(matches!(err, NotificationError::KindTooLong { actual: 51 }));
    }

    #[test]
    fn accepts_kind_exactly_50_chars() {
        let exactly = "X".repeat(50);
        let n = Notification::try_new(Id::new(), Id::new(), &exactly, sample_payload(), Utc::now())
            .expect("50 ok");
        assert_eq!(n.kind.chars().count(), 50);
    }

    #[test]
    fn mark_read_happy_path() {
        let mut n = Notification::try_new(
            Id::new(),
            Id::new(),
            "auction_deadline_approaching",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        assert!(n.is_unread());
        let read_time = Utc::now();
        n.mark_read(read_time);
        assert_eq!(n.read_at, Some(read_time));
        assert!(n.is_read());
    }

    #[test]
    fn mark_read_idempotent_preserves_first_timestamp() {
        let mut n = Notification::try_new(
            Id::new(),
            Id::new(),
            "bookmark_listing_changed",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        let first = Utc::now();
        n.mark_read(first);
        let second = first + chrono::Duration::seconds(60);
        n.mark_read(second);
        assert_eq!(n.read_at, Some(first));
    }

    #[test]
    fn is_read_and_is_unread_are_inverse() {
        let mut n = Notification::try_new(
            Id::new(),
            Id::new(),
            "bookmark_listing_changed",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        assert!(!n.is_read());
        assert!(n.is_unread());
        n.mark_read(Utc::now());
        assert!(n.is_read());
        assert!(!n.is_unread());
    }

    #[test]
    fn initial_read_at_none_and_created_at_matches_now() {
        let now = Utc::now();
        let n = Notification::try_new(
            Id::new(),
            Id::new(),
            "bookmark_listing_changed",
            sample_payload(),
            now,
        )
        .expect("valid");
        assert!(n.read_at.is_none());
        assert_eq!(n.created_at, now);
    }

    #[test]
    fn trim_normalizes_kind() {
        let n = Notification::try_new(
            Id::new(),
            Id::new(),
            "  bookmark_listing_changed  ",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(n.kind, "bookmark_listing_changed");
    }

    #[test]
    fn serde_roundtrip_unread() {
        let n = Notification::try_new(
            Id::new(),
            Id::new(),
            "bookmark_listing_changed",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&n).expect("serialize");
        let back: Notification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(n, back);
    }

    #[test]
    fn serde_roundtrip_read() {
        let mut n = Notification::try_new(
            Id::new(),
            Id::new(),
            "auction_deadline_approaching",
            sample_payload(),
            Utc::now(),
        )
        .expect("valid");
        n.mark_read(Utc::now());
        let json = serde_json::to_string(&n).expect("serialize");
        let back: Notification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(n, back);
        assert!(back.is_read());
    }
}
