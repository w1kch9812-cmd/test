//! `BookmarkExternal` Aggregate (polymorphic to `R2` entities).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{BookmarkExternalMarker, Id, UserMarker};

use crate::errors::BookmarkError;
use crate::external_kind::BookmarkExternalKind;

/// 외부 `R2` entity 북마크.
/// `Parcel`/`CourtAuction`/`Manufacturer`/`IndustrialComplex` 대상.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookmarkExternal {
    /// 식별자 (`bme_<26 ULID>`).
    pub id: Id<BookmarkExternalMarker>,
    /// 사용자 ID.
    pub user_id: Id<UserMarker>,
    /// 대상 종류.
    pub target_kind: BookmarkExternalKind,
    /// 대상 식별자 (`PNU` 또는 `R2` 식별자, ≤50자).
    pub target_id: String,
    /// 사용자 메모 (≤500자, 선택).
    pub note: Option<String>,
    /// 생성 시각.
    pub created_at: DateTime<Utc>,
}

impl BookmarkExternal {
    /// 검증 후 생성.
    ///
    /// # Errors
    ///
    /// - `target_id` 빈 → [`BookmarkError::EmptyTargetId`].
    /// - `target_id` 50자 초과 → [`BookmarkError::TargetIdTooLong`].
    /// - `note` 500자 초과 → [`BookmarkError::NoteTooLong`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Id<BookmarkExternalMarker>,
        user_id: Id<UserMarker>,
        target_kind: BookmarkExternalKind,
        target_id: &str,
        note: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, BookmarkError> {
        let target_id = target_id.trim().to_owned();
        if target_id.is_empty() {
            return Err(BookmarkError::EmptyTargetId);
        }
        if target_id.chars().count() > 50 {
            return Err(BookmarkError::TargetIdTooLong {
                actual: target_id.chars().count(),
            });
        }
        if let Some(ref n) = note {
            if n.chars().count() > 500 {
                return Err(BookmarkError::NoteTooLong {
                    actual: n.chars().count(),
                });
            }
        }
        Ok(Self {
            id,
            user_id,
            target_kind,
            target_id,
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
    fn happy_path_parcel_target() {
        let bm = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::Parcel,
            "1111010100100010000",
            None,
            Utc::now(),
        )
        .expect("valid");
        assert_eq!(bm.target_kind, BookmarkExternalKind::Parcel);
    }

    #[test]
    fn happy_path_with_note() {
        let bm = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::CourtAuction,
            "2024타경12345",
            Some("관심 경매".to_owned()),
            Utc::now(),
        )
        .expect("valid");
        assert!(bm.note.is_some());
    }

    #[test]
    fn rejects_empty_target_id() {
        let err = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::Parcel,
            "",
            None,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, BookmarkError::EmptyTargetId));
    }

    #[test]
    fn rejects_whitespace_only_target_id() {
        let err = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::Parcel,
            "   ",
            None,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, BookmarkError::EmptyTargetId));
    }

    #[test]
    fn rejects_target_id_over_50() {
        let long = "X".repeat(51);
        let err = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::Manufacturer,
            &long,
            None,
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, BookmarkError::TargetIdTooLong { actual: 51 }));
    }

    #[test]
    fn rejects_note_over_500() {
        let long_note = "X".repeat(501);
        let err = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::Parcel,
            "valid_target",
            Some(long_note),
            Utc::now(),
        )
        .unwrap_err();
        assert!(matches!(err, BookmarkError::NoteTooLong { actual: 501 }));
    }

    #[test]
    fn serde_roundtrip() {
        let bm = BookmarkExternal::try_new(
            Id::new(),
            Id::new(),
            BookmarkExternalKind::IndustrialComplex,
            "IC_001",
            Some("입주 검토".to_owned()),
            Utc::now(),
        )
        .expect("valid");
        let json = serde_json::to_string(&bm).expect("serialize");
        let back: BookmarkExternal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bm, back);
    }
}
