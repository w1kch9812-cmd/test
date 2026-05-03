//! `ListingReviewQueue` Aggregate (decision-based workflow + version OCC + 12h SLA + `auto_check`).
//!
//! 사용자가 등록한 매물을 어드민이 검토해 *승인* / *거부* / *변경 요청*
//! (`request_changes`) 처리하는 워크플로우를 모델링해요.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::id::{Id, ListingMarker, LrqMarker, UserMarker};

use crate::decision::LrqDecision;
use crate::errors::LrqError;

/// `reviewer_note` 최대 길이 (2000자).
const MAX_REVIEWER_NOTE_LEN: usize = 2000;
/// SLA 기간 (12시간).
const SLA_HOURS: i64 = 12;
/// `auto_check_score` 최댓값 (0-100).
const MAX_AUTO_CHECK_SCORE: u8 = 100;

/// 매물 검토 큐 1건. Decision-based workflow + version OCC.
///
/// `decision` 이 `None` 이면 pending (검토 대기), `Some(_)` 이면 terminal (이후 변경 불가).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingReviewQueue {
    /// 식별자 (`lrq_<26 ULID>`).
    pub id: Id<LrqMarker>,
    /// 검토 대상 매물 (FK → `listing.id`, `ON DELETE CASCADE`).
    pub listing_id: Id<ListingMarker>,
    /// 제출 시각.
    pub submitted_at: DateTime<Utc>,
    /// 룰 기반 자동 점수 (0-100). `None` 이면 자동 검사 미실행.
    pub auto_check_score: Option<u8>,
    /// 자동 검사 플래그 (`JSONB`, 예: `["suspected_duplicate", "price_anomaly"]`).
    pub auto_check_flags: Option<serde_json::Value>,
    /// 검토 어드민 (FK → `user.id`). 결정 전에는 `None`.
    pub reviewer_id: Option<Id<UserMarker>>,
    /// 검토 메모 (≤2000자). `reject` / `request_changes` 는 필수.
    pub reviewer_note: Option<String>,
    /// 결정. `None` = pending, `Some(_)` = terminal.
    pub decision: Option<LrqDecision>,
    /// 결정 완료 시각 (`decide_*` 시 기록).
    pub decided_at: Option<DateTime<Utc>>,
    /// SLA 만료 시각 (`submitted_at + 12h`).
    pub sla_due_at: Option<DateTime<Utc>>,
    /// 마지막 갱신 시각 (DB `updated_at` 보조 필드).
    pub updated_at: DateTime<Utc>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl ListingReviewQueue {
    /// Pending 상태 신규 큐 생성. SLA = `submitted_at + 12h`.
    ///
    /// `decision = None`, `reviewer_id = None`, `reviewer_note = None`,
    /// `decided_at = None`, `version = 1`. `updated_at == submitted_at`.
    ///
    /// # Errors
    ///
    /// `auto_check_score` 가 `Some` 이고 100 초과 시 [`LrqError::AutoCheckScoreOutOfRange`].
    pub fn try_new_pending(
        id: Id<LrqMarker>,
        listing_id: Id<ListingMarker>,
        auto_check_score: Option<u8>,
        auto_check_flags: Option<serde_json::Value>,
        submitted_at: DateTime<Utc>,
    ) -> Result<Self, LrqError> {
        if let Some(score) = auto_check_score {
            if score > MAX_AUTO_CHECK_SCORE {
                return Err(LrqError::AutoCheckScoreOutOfRange {
                    actual: u32::from(score),
                });
            }
        }
        let sla_due_at = submitted_at.checked_add_signed(Duration::hours(SLA_HOURS));
        Ok(Self {
            id,
            listing_id,
            submitted_at,
            auto_check_score,
            auto_check_flags,
            reviewer_id: None,
            reviewer_note: None,
            decision: None,
            decided_at: None,
            sla_due_at,
            updated_at: submitted_at,
            version: 1,
        })
    }

    /// 결정 전 (pending) 상태인지 검사.
    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.decision.is_none()
    }

    /// 검토 메모 정규화 + 검증 (≤2000자, `required_for` 가 `Some` 이면 비어있으면 안 됨).
    fn normalize_note(
        note: Option<String>,
        required_for: Option<&'static str>,
    ) -> Result<Option<String>, LrqError> {
        let trimmed = note.map(|n| n.trim().to_owned());
        match (trimmed, required_for) {
            (None, Some(action)) => Err(LrqError::EmptyReviewerNote { action }),
            (Some(s), Some(action)) if s.is_empty() => Err(LrqError::EmptyReviewerNote { action }),
            (Some(s), _) => {
                let len = s.chars().count();
                if len > MAX_REVIEWER_NOTE_LEN {
                    return Err(LrqError::ReviewerNoteTooLong { actual: len });
                }
                Ok(Some(s))
            }
            (None, None) => Ok(None),
        }
    }

    /// 내부 헬퍼 — 결정 + `version` bump + `updated_at`/`decided_at` 갱신.
    /// 이미 결정된 경우 [`LrqError::AlreadyDecided`] 로 거부.
    fn apply_decision(
        &mut self,
        decision: LrqDecision,
        reviewer: Id<UserMarker>,
        note: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<(), LrqError> {
        if self.decision.is_some() {
            return Err(LrqError::AlreadyDecided);
        }
        self.decision = Some(decision);
        self.reviewer_id = Some(reviewer);
        self.reviewer_note = note;
        self.decided_at = Some(at);
        self.updated_at = at;
        self.version += 1;
        Ok(())
    }

    /// Pending → `Approve`. 어드민 승인 (메모는 선택).
    ///
    /// # Errors
    ///
    /// 이미 결정된 경우 [`LrqError::AlreadyDecided`].
    /// `note` 가 2000자 초과 시 [`LrqError::ReviewerNoteTooLong`].
    pub fn decide_approve(
        &mut self,
        reviewer: Id<UserMarker>,
        note: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<(), LrqError> {
        // 결정 적용 전에 메모 검증을 먼저 수행해 부분 mutation 방지.
        if self.decision.is_some() {
            return Err(LrqError::AlreadyDecided);
        }
        let normalized = Self::normalize_note(note, None)?;
        self.apply_decision(LrqDecision::Approve, reviewer, normalized, at)
    }

    /// Pending → `Reject`. 어드민 거부 (메모 필수).
    ///
    /// # Errors
    ///
    /// 이미 결정된 경우 [`LrqError::AlreadyDecided`].
    /// `note` 가 비어있으면 [`LrqError::EmptyReviewerNote`].
    /// `note` 가 2000자 초과 시 [`LrqError::ReviewerNoteTooLong`].
    pub fn decide_reject(
        &mut self,
        reviewer: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), LrqError> {
        if self.decision.is_some() {
            return Err(LrqError::AlreadyDecided);
        }
        let normalized = Self::normalize_note(Some(note), Some("reject"))?;
        self.apply_decision(LrqDecision::Reject, reviewer, normalized, at)
    }

    /// Pending → `RequestChanges`. 어드민이 매물 정보 수정 요청 (메모 필수).
    ///
    /// # Errors
    ///
    /// 이미 결정된 경우 [`LrqError::AlreadyDecided`].
    /// `note` 가 비어있으면 [`LrqError::EmptyReviewerNote`].
    /// `note` 가 2000자 초과 시 [`LrqError::ReviewerNoteTooLong`].
    pub fn decide_request_changes(
        &mut self,
        reviewer: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), LrqError> {
        if self.decision.is_some() {
            return Err(LrqError::AlreadyDecided);
        }
        let normalized = Self::normalize_note(Some(note), Some("request_changes"))?;
        self.apply_decision(LrqDecision::RequestChanges, reviewer, normalized, at)
    }
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
