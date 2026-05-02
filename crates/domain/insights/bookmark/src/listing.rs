//! `BookmarkListing` Aggregate (composite PK).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker, UserMarker};

use crate::errors::BookmarkError;

/// 매물 북마크. 사용자가 매물을 *저장*한 기록.
///
/// Composite PK: `(user_id, listing_id)`. FK to `User` and `Listing`
/// (둘 다 `ON DELETE CASCADE`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkListing {
    /// 사용자 ID.
    pub user_id: Id<UserMarker>,
    /// 매물 ID.
    pub listing_id: Id<ListingMarker>,
    /// 사용자 메모 (≤500자, 선택).
    pub note: Option<String>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl BookmarkListing {
    /// 검증 후 생성. `note`가 `Some`이면 ≤500자 검증.
    ///
    /// # Errors
    ///
    /// `note` 500자 초과 → [`BookmarkError::NoteTooLong`].
    pub fn try_new(
        user_id: Id<UserMarker>,
        listing_id: Id<ListingMarker>,
        note: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, BookmarkError> {
        if let Some(ref n) = note {
            if n.chars().count() > 500 {
                return Err(BookmarkError::NoteTooLong {
                    actual: n.chars().count(),
                });
            }
        }
        Ok(Self {
            user_id,
            listing_id,
            note,
            created_at: now,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;

    #[test]
    fn happy_path_with_note() {
        let bm = BookmarkListing::try_new(
            Id::new(),
            Id::new(),
            Some("관심 매물".to_owned()),
            Utc::now(),
        )
        .expect("valid");
        assert!(bm.note.is_some());
    }

    #[test]
    fn happy_path_without_note() {
        let bm = BookmarkListing::try_new(Id::new(), Id::new(), None, Utc::now()).expect("valid");
        assert!(bm.note.is_none());
    }

    #[test]
    fn rejects_note_over_500_chars() {
        let long = "X".repeat(501);
        let err =
            BookmarkListing::try_new(Id::new(), Id::new(), Some(long), Utc::now()).unwrap_err();
        assert!(matches!(err, BookmarkError::NoteTooLong { actual: 501 }));
    }

    #[test]
    fn accepts_exactly_500_chars() {
        let exactly = "X".repeat(500);
        let bm = BookmarkListing::try_new(Id::new(), Id::new(), Some(exactly), Utc::now())
            .expect("500 ok");
        assert_eq!(bm.note.unwrap().len(), 500);
    }

    #[test]
    fn serde_roundtrip() {
        let bm = BookmarkListing::try_new(Id::new(), Id::new(), Some("hi".to_owned()), Utc::now())
            .expect("valid");
        let json = serde_json::to_string(&bm).expect("serialize");
        let back: BookmarkListing = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bm, back);
    }
}
