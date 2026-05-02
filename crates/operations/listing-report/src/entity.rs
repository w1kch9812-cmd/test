//! `ListingReport` Aggregate (4-status workflow, no OCC, anonymous reporter 허용).
//!
//! 사용자가 등록한 매물에 대해 신고를 접수하면 어드민이 `Open` → `Investigating` →
//! `Confirmed` / `Dismissed` 워크플로우로 처리해요.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker, ListingReportMarker, UserMarker};

use crate::errors::ListingReportError;
use crate::reason::ListingReportReason;
use crate::status::ListingReportStatus;

/// `detail` 최대 길이 (2000자).
const MAX_DETAIL_LEN: usize = 2000;
/// `handler_note` 최대 길이 (2000자).
const MAX_HANDLER_NOTE_LEN: usize = 2000;

/// 매물 신고 1건. 4-status workflow + 익명 신고 허용 + handler 메모 필수(마감 시).
///
/// `status` 가 `Confirmed` / `Dismissed` 면 terminal — 이후 모든 `mark_*` 호출이
/// `InvalidTransition` 으로 거부돼요.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingReport {
    /// 식별자 (`lrp_<26 ULID>`).
    pub id: Id<ListingReportMarker>,
    /// 신고 대상 매물 (FK → `listing.id`).
    pub listing_id: Id<ListingMarker>,
    /// 신고자 (FK → `user.id`). `None` = 익명.
    pub reporter_id: Option<Id<UserMarker>>,
    /// 신고 사유 (6값).
    pub reason: ListingReportReason,
    /// 신고 상세 설명 (≤2000자, 선택).
    pub detail: Option<String>,
    /// 처리 상태 (4값). 기본 `Open`.
    pub status: ListingReportStatus,
    /// 처리 어드민 (FK → `user.id`). 미배정이면 `None`.
    pub handler_id: Option<Id<UserMarker>>,
    /// 처리 메모 (≤2000자). `Confirmed` / `Dismissed` 시 필수.
    pub handler_note: Option<String>,
    /// 신고 접수 시각.
    pub created_at: DateTime<Utc>,
    /// 처리 완료 시각 (terminal 진입 시 기록). 기본 `None`.
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ListingReport {
    /// 신고 신규 접수. `status = Open`, `handler_*` / `resolved_at = None`.
    /// ID 자동 생성 (`lrp_…`).
    ///
    /// `reporter_id = None` 이면 익명 신고로 기록돼요.
    ///
    /// # Errors
    ///
    /// `detail` 이 `Some` 이고 2000자 초과 시 [`ListingReportError::DetailTooLong`].
    pub fn try_new(
        listing_id: Id<ListingMarker>,
        reporter_id: Option<Id<UserMarker>>,
        reason: ListingReportReason,
        detail: Option<String>,
        created_at: DateTime<Utc>,
    ) -> Result<Self, ListingReportError> {
        let normalized_detail = match detail {
            Some(d) => {
                let len = d.chars().count();
                if len > MAX_DETAIL_LEN {
                    return Err(ListingReportError::DetailTooLong { actual: len });
                }
                Some(d)
            }
            None => None,
        };
        Ok(Self {
            id: Id::new(),
            listing_id,
            reporter_id,
            reason,
            detail: normalized_detail,
            status: ListingReportStatus::Open,
            handler_id: None,
            handler_note: None,
            created_at,
            resolved_at: None,
        })
    }

    /// 미처리 신고 (status == `Open`) 인지 검사.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        matches!(self.status, ListingReportStatus::Open)
    }

    /// 처리 완료 (status == `Confirmed` / `Dismissed`) 인지 검사.
    #[must_use]
    pub const fn is_resolved(&self) -> bool {
        self.status.is_terminal()
    }

    /// `Open` → `Investigating`. handler 배정 + 조사 시작.
    ///
    /// `at` 은 워크플로우 일관성을 위해 받아두지만 `Investigating` 은 terminal 이 아니므로
    /// `resolved_at` 에는 기록되지 않아요.
    ///
    /// # Errors
    ///
    /// `Open` 이 아닌 모든 상태에서 [`ListingReportError::InvalidTransition`].
    pub fn mark_investigating(
        &mut self,
        handler_id: Id<UserMarker>,
        _at: DateTime<Utc>,
    ) -> Result<(), ListingReportError> {
        // `_at` 은 spec 워크플로우 일관성을 위해 받지만 `Investigating` 은 terminal 이
        // 아니므로 `resolved_at` 에 기록되지 않아요.
        if !matches!(self.status, ListingReportStatus::Open) {
            return Err(ListingReportError::InvalidTransition { from: self.status });
        }
        self.status = ListingReportStatus::Investigating;
        self.handler_id = Some(handler_id);
        Ok(())
    }

    /// `Open` / `Investigating` → `Confirmed` (terminal). 신고 확정 + handler 메모 필수.
    ///
    /// `resolved_at = Some(at)`, `handler_id = Some(handler_id)`,
    /// `handler_note = Some(trim 된 note)`.
    ///
    /// # Errors
    ///
    /// - 이미 terminal (`Confirmed` / `Dismissed`) 시 [`ListingReportError::InvalidTransition`].
    /// - `note` 가 trim 후 빈 문자열이면 [`ListingReportError::EmptyHandlerNote`].
    /// - `note` 가 2000자 초과 시 [`ListingReportError::HandlerNoteTooLong`].
    pub fn mark_confirmed(
        &mut self,
        handler_id: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), ListingReportError> {
        self.apply_terminal(ListingReportStatus::Confirmed, handler_id, note, at)
    }

    /// `Open` / `Investigating` → `Dismissed` (terminal). 신고 기각 + handler 메모 필수.
    ///
    /// `resolved_at = Some(at)`, `handler_id = Some(handler_id)`,
    /// `handler_note = Some(trim 된 note)`.
    ///
    /// # Errors
    ///
    /// - 이미 terminal (`Confirmed` / `Dismissed`) 시 [`ListingReportError::InvalidTransition`].
    /// - `note` 가 trim 후 빈 문자열이면 [`ListingReportError::EmptyHandlerNote`].
    /// - `note` 가 2000자 초과 시 [`ListingReportError::HandlerNoteTooLong`].
    pub fn mark_dismissed(
        &mut self,
        handler_id: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), ListingReportError> {
        self.apply_terminal(ListingReportStatus::Dismissed, handler_id, note, at)
    }

    /// `mark_confirmed` / `mark_dismissed` 공통 로직 — note 검증 + state 전이.
    fn apply_terminal(
        &mut self,
        next: ListingReportStatus,
        handler_id: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), ListingReportError> {
        if self.status.is_terminal() {
            return Err(ListingReportError::InvalidTransition { from: self.status });
        }
        let trimmed = note.trim().to_owned();
        if trimmed.is_empty() {
            return Err(ListingReportError::EmptyHandlerNote);
        }
        let len = trimmed.chars().count();
        if len > MAX_HANDLER_NOTE_LEN {
            return Err(ListingReportError::HandlerNoteTooLong { actual: len });
        }
        self.status = next;
        self.handler_id = Some(handler_id);
        self.handler_note = Some(trimmed);
        self.resolved_at = Some(at);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use chrono::Duration;

    fn make_open(at: DateTime<Utc>) -> ListingReport {
        ListingReport::try_new(
            Id::new(),
            Some(Id::<UserMarker>::new()),
            ListingReportReason::FakeListing,
            Some("의심돼요".to_owned()),
            at,
        )
        .expect("valid open report")
    }

    // ── try_new ───────────────────────────────────────────────────

    #[test]
    fn try_new_status_is_open() {
        let now = Utc::now();
        let r = make_open(now);
        assert_eq!(r.status, ListingReportStatus::Open);
        assert!(r.is_pending());
        assert!(!r.is_resolved());
    }

    #[test]
    fn try_new_id_has_lrp_prefix() {
        let now = Utc::now();
        let r = make_open(now);
        assert!(r.id.as_str().starts_with("lrp_"));
        assert_eq!(r.id.as_str().len(), 30);
    }

    #[test]
    fn try_new_handler_fields_are_none() {
        let now = Utc::now();
        let r = make_open(now);
        assert!(r.handler_id.is_none());
        assert!(r.handler_note.is_none());
        assert!(r.resolved_at.is_none());
    }

    #[test]
    fn try_new_preserves_listing_and_reason() {
        let now = Utc::now();
        let listing_id = Id::<ListingMarker>::new();
        let r = ListingReport::try_new(
            listing_id.clone(),
            None,
            ListingReportReason::Spam,
            None,
            now,
        )
        .expect("ok");
        assert_eq!(r.listing_id, listing_id);
        assert_eq!(r.reason, ListingReportReason::Spam);
        assert_eq!(r.created_at, now);
    }

    #[test]
    fn try_new_with_anonymous_reporter() {
        let now = Utc::now();
        let r = ListingReport::try_new(Id::new(), None, ListingReportReason::Other, None, now)
            .expect("anon ok");
        assert!(r.reporter_id.is_none());
        assert!(r.detail.is_none());
    }

    #[test]
    fn try_new_with_named_reporter() {
        let now = Utc::now();
        let reporter = Id::<UserMarker>::new();
        let r = ListingReport::try_new(
            Id::new(),
            Some(reporter.clone()),
            ListingReportReason::WrongPrice,
            Some("실거래보다 30% 비싸요".to_owned()),
            now,
        )
        .expect("named ok");
        assert_eq!(r.reporter_id, Some(reporter));
        assert_eq!(r.detail.as_deref(), Some("실거래보다 30% 비싸요"));
    }

    #[test]
    fn try_new_with_2000_char_detail_accepted() {
        let now = Utc::now();
        let exactly = "X".repeat(2000);
        let r = ListingReport::try_new(
            Id::new(),
            None,
            ListingReportReason::Other,
            Some(exactly.clone()),
            now,
        )
        .expect("2000 ok");
        assert_eq!(r.detail.as_deref(), Some(exactly.as_str()));
    }

    #[test]
    fn try_new_with_2001_char_detail_errors() {
        let now = Utc::now();
        let too_long = "X".repeat(2001);
        let err = ListingReport::try_new(
            Id::new(),
            None,
            ListingReportReason::Other,
            Some(too_long),
            now,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::DetailTooLong { actual: 2001 }
        ));
    }

    // ── mark_investigating ────────────────────────────────────────

    #[test]
    fn mark_investigating_happy_path() {
        let now = Utc::now();
        let mut r = make_open(now);
        let handler = Id::<UserMarker>::new();
        r.mark_investigating(handler.clone(), now + Duration::hours(1))
            .expect("investigate ok");
        assert_eq!(r.status, ListingReportStatus::Investigating);
        assert_eq!(r.handler_id, Some(handler));
        // resolved_at 은 terminal 이 아니라 기록 안 됨.
        assert!(r.resolved_at.is_none());
        assert!(r.handler_note.is_none());
    }

    #[test]
    fn mark_investigating_already_investigating_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_investigating(Id::new(), now + Duration::hours(1))
            .expect("first ok");
        let err = r
            .mark_investigating(Id::new(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Investigating
            }
        ));
    }

    #[test]
    fn mark_investigating_already_confirmed_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_confirmed(Id::new(), "확정".to_owned(), now + Duration::hours(1))
            .expect("confirmed ok");
        let err = r
            .mark_investigating(Id::new(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Confirmed
            }
        ));
    }

    #[test]
    fn mark_investigating_already_dismissed_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_dismissed(Id::new(), "기각".to_owned(), now + Duration::hours(1))
            .expect("dismissed ok");
        let err = r
            .mark_investigating(Id::new(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Dismissed
            }
        ));
    }

    // ── mark_confirmed ────────────────────────────────────────────

    #[test]
    fn mark_confirmed_from_open_happy_path() {
        let now = Utc::now();
        let mut r = make_open(now);
        let handler = Id::<UserMarker>::new();
        let later = now + Duration::hours(2);
        r.mark_confirmed(handler.clone(), "신고 확정해요".to_owned(), later)
            .expect("confirmed ok");
        assert_eq!(r.status, ListingReportStatus::Confirmed);
        assert_eq!(r.handler_id, Some(handler));
        assert_eq!(r.handler_note.as_deref(), Some("신고 확정해요"));
        assert_eq!(r.resolved_at, Some(later));
        assert!(r.is_resolved());
        assert!(!r.is_pending());
    }

    #[test]
    fn mark_confirmed_from_investigating_happy_path() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_investigating(Id::new(), now + Duration::hours(1))
            .expect("inv ok");
        let later = now + Duration::hours(3);
        r.mark_confirmed(Id::new(), "조사 후 확정".to_owned(), later)
            .expect("confirmed ok");
        assert_eq!(r.status, ListingReportStatus::Confirmed);
        assert_eq!(r.handler_note.as_deref(), Some("조사 후 확정"));
        assert_eq!(r.resolved_at, Some(later));
    }

    #[test]
    fn mark_confirmed_already_confirmed_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_confirmed(Id::new(), "first".to_owned(), now + Duration::hours(1))
            .expect("first ok");
        let err = r
            .mark_confirmed(Id::new(), "second".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Confirmed
            }
        ));
    }

    #[test]
    fn mark_confirmed_already_dismissed_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_dismissed(Id::new(), "dismiss".to_owned(), now + Duration::hours(1))
            .expect("dismissed ok");
        let err = r
            .mark_confirmed(Id::new(), "confirm".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Dismissed
            }
        ));
    }

    #[test]
    fn mark_confirmed_with_empty_note_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        let err = r
            .mark_confirmed(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(err, ListingReportError::EmptyHandlerNote));
        assert!(r.is_pending());
        assert!(r.handler_id.is_none());
    }

    #[test]
    fn mark_confirmed_with_whitespace_note_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        let err = r
            .mark_confirmed(Id::new(), "   \t\n".to_owned(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(err, ListingReportError::EmptyHandlerNote));
        assert!(r.is_pending());
    }

    #[test]
    fn mark_confirmed_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        let too_long = "X".repeat(2001);
        let err = r
            .mark_confirmed(Id::new(), too_long, now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::HandlerNoteTooLong { actual: 2001 }
        ));
        assert!(r.is_pending());
    }

    #[test]
    fn mark_confirmed_with_2000_char_note_accepted() {
        let now = Utc::now();
        let mut r = make_open(now);
        let exactly = "X".repeat(2000);
        r.mark_confirmed(Id::new(), exactly.clone(), now + Duration::hours(1))
            .expect("2000 ok");
        assert_eq!(r.handler_note.as_deref(), Some(exactly.as_str()));
    }

    // ── mark_dismissed ────────────────────────────────────────────

    #[test]
    fn mark_dismissed_from_open_happy_path() {
        let now = Utc::now();
        let mut r = make_open(now);
        let handler = Id::<UserMarker>::new();
        let later = now + Duration::hours(2);
        r.mark_dismissed(handler.clone(), "문제 없음".to_owned(), later)
            .expect("dismiss ok");
        assert_eq!(r.status, ListingReportStatus::Dismissed);
        assert_eq!(r.handler_id, Some(handler));
        assert_eq!(r.handler_note.as_deref(), Some("문제 없음"));
        assert_eq!(r.resolved_at, Some(later));
        assert!(r.is_resolved());
    }

    #[test]
    fn mark_dismissed_from_investigating_happy_path() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_investigating(Id::new(), now + Duration::hours(1))
            .expect("inv ok");
        let later = now + Duration::hours(3);
        r.mark_dismissed(Id::new(), "조사 후 기각".to_owned(), later)
            .expect("dismiss ok");
        assert_eq!(r.status, ListingReportStatus::Dismissed);
        assert_eq!(r.handler_note.as_deref(), Some("조사 후 기각"));
    }

    #[test]
    fn mark_dismissed_already_terminal_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_confirmed(Id::new(), "confirmed".to_owned(), now + Duration::hours(1))
            .expect("confirmed ok");
        let err = r
            .mark_dismissed(Id::new(), "dismiss".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::InvalidTransition {
                from: ListingReportStatus::Confirmed
            }
        ));
    }

    #[test]
    fn mark_dismissed_with_empty_note_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        let err = r
            .mark_dismissed(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(err, ListingReportError::EmptyHandlerNote));
        assert!(r.is_pending());
    }

    #[test]
    fn mark_dismissed_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut r = make_open(now);
        let too_long = "X".repeat(2001);
        let err = r
            .mark_dismissed(Id::new(), too_long, now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            ListingReportError::HandlerNoteTooLong { actual: 2001 }
        ));
    }

    // ── is_pending / is_resolved ──────────────────────────────────

    #[test]
    fn is_pending_true_for_open() {
        let now = Utc::now();
        let r = make_open(now);
        assert!(r.is_pending());
    }

    #[test]
    fn is_pending_false_for_investigating() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_investigating(Id::new(), now + Duration::hours(1))
            .expect("inv ok");
        assert!(!r.is_pending());
        assert!(!r.is_resolved()); // not terminal yet
    }

    #[test]
    fn is_resolved_true_after_confirmed() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_confirmed(Id::new(), "ok".to_owned(), now + Duration::hours(1))
            .expect("confirmed ok");
        assert!(r.is_resolved());
        assert!(!r.is_pending());
    }

    #[test]
    fn is_resolved_true_after_dismissed() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_dismissed(Id::new(), "ok".to_owned(), now + Duration::hours(1))
            .expect("dismiss ok");
        assert!(r.is_resolved());
    }

    // ── serde ─────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_open() {
        let now = Utc::now();
        let r = make_open(now);
        let json = serde_json::to_string(&r).expect("serialize");
        let back: ListingReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }

    #[test]
    fn serde_roundtrip_confirmed() {
        let now = Utc::now();
        let mut r = make_open(now);
        r.mark_confirmed(Id::new(), "확정".to_owned(), now + Duration::hours(1))
            .expect("confirmed ok");
        let json = serde_json::to_string(&r).expect("serialize");
        let back: ListingReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }
}
