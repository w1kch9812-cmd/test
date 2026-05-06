//! `Notification` Aggregate (append-mostly + 멱등 `mark_read`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, NotificationMarker, UserMarker};

use crate::kind::NotificationKind;

/// 사용자 알림 1건. append-mostly (이벤트 발생 시 INSERT).
///
/// SP6-v: `kind: NotificationKind` 도메인 enum (이전 `String`, 1-50자 검증).
/// enum variant 가 bounded 라 length 검증 불필요 — `try_new` 가 infallible.
///
/// `mark_read`는 멱등 — 이미 읽은 알림 재호출 시 `read_at` 보존.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notification {
    /// 식별자 (`ntf_<26 ULID>`).
    pub id: Id<NotificationMarker>,
    /// 수신자.
    pub user_id: Id<UserMarker>,
    /// 알림 종류 (도메인 enum).
    pub kind: NotificationKind,
    /// 이벤트 컨텍스트 (`JSONB`).
    pub payload: serde_json::Value,
    /// 읽음 시각. `None` = 미읽음.
    pub read_at: Option<DateTime<Utc>>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl Notification {
    /// 새 알림 생성. `read_at = None`, `created_at = now`.
    ///
    /// `NotificationKind` enum 이 bounded variant 라 검증 불필요 — infallible.
    /// SP6-v 이전 String 기반 `try_new` 의 `EmptyKind` / `KindTooLong` 에러는
    /// 도달 불가능해져 deprecated.
    #[must_use]
    pub const fn new(
        id: Id<NotificationMarker>,
        user_id: Id<UserMarker>,
        kind: NotificationKind,
        payload: serde_json::Value,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            kind,
            payload,
            read_at: None,
            created_at: now,
        }
    }

    /// 읽음 처리 — 멱등. 이미 읽은 경우 `read_at`를 보존해요.
    ///
    /// append-mostly 도메인이라 `version` bump 없어요.
    pub const fn mark_read(&mut self, at: DateTime<Utc>) {
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
    fn happy_path_listing_bookmarked() {
        let now = Utc::now();
        let n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingBookmarked,
            sample_payload(),
            now,
        );
        assert_eq!(n.kind, NotificationKind::ListingBookmarked);
        assert!(n.read_at.is_none());
        assert_eq!(n.created_at, now);
    }

    #[test]
    fn happy_path_listing_approved() {
        let n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingApproved,
            sample_payload(),
            Utc::now(),
        );
        assert_eq!(n.kind, NotificationKind::ListingApproved);
    }

    #[test]
    fn mark_read_happy_path() {
        let mut n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingApproved,
            sample_payload(),
            Utc::now(),
        );
        assert!(n.is_unread());
        let read_time = Utc::now();
        n.mark_read(read_time);
        assert_eq!(n.read_at, Some(read_time));
        assert!(n.is_read());
    }

    #[test]
    fn mark_read_idempotent_preserves_first_timestamp() {
        let mut n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingBookmarked,
            sample_payload(),
            Utc::now(),
        );
        let first = Utc::now();
        n.mark_read(first);
        let second = first + chrono::Duration::seconds(60);
        n.mark_read(second);
        assert_eq!(n.read_at, Some(first));
    }

    #[test]
    fn is_read_and_is_unread_are_inverse() {
        let mut n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingBookmarked,
            sample_payload(),
            Utc::now(),
        );
        assert!(!n.is_read());
        assert!(n.is_unread());
        n.mark_read(Utc::now());
        assert!(n.is_read());
        assert!(!n.is_unread());
    }

    #[test]
    fn initial_read_at_none_and_created_at_matches_now() {
        let now = Utc::now();
        let n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingRejected,
            sample_payload(),
            now,
        );
        assert!(n.read_at.is_none());
        assert_eq!(n.created_at, now);
    }

    #[test]
    fn serde_roundtrip_unread() {
        let n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingBookmarked,
            sample_payload(),
            Utc::now(),
        );
        let json = serde_json::to_string(&n).expect("serialize");
        let back: Notification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(n, back);
    }

    #[test]
    fn serde_roundtrip_read() {
        let mut n = Notification::new(
            Id::new(),
            Id::new(),
            NotificationKind::ListingApproved,
            sample_payload(),
            Utc::now(),
        );
        n.mark_read(Utc::now());
        let json = serde_json::to_string(&n).expect("serialize");
        let back: Notification = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(n, back);
        assert!(back.is_read());
    }
}
