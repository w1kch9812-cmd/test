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
        self.apply_terminal(ListingReportStatus::Confirmed, handler_id, Some(note), at)
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
        self.apply_terminal(ListingReportStatus::Dismissed, handler_id, Some(note), at)
    }

    /// `mark_confirmed` / `mark_dismissed` 공통 — handler note 검증 + state 전이.
    ///
    /// `note` 를 `Option<String>` 으로 받아 `.map(|n| n.trim().to_owned())` 패턴으로 소비.
    fn apply_terminal(
        &mut self,
        next: ListingReportStatus,
        handler_id: Id<UserMarker>,
        note: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<(), ListingReportError> {
        if self.status.is_terminal() {
            return Err(ListingReportError::InvalidTransition { from: self.status });
        }
        let trimmed = note.map_or_else(String::new, |n| n.trim().to_owned());
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
#[path = "entity_tests.rs"]
mod tests;
