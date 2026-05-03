//! `BusinessVerificationQueue` Aggregate (4-status workflow + version OCC + 24h SLA).
//!
//! 사용자가 제출한 사업자등록증 등 문서 (R2 keys) 를 어드민이 검토해 승인 / 거부 /
//! 추가 자료 요청 처리하는 워크플로우를 모델링해요.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use shared_kernel::business_number::BusinessNumber;
use shared_kernel::id::{BvqMarker, Id, UserMarker};

use crate::errors::BvqError;
use crate::status::BvqStatus;

/// `reviewer_note` 최대 길이 (2000자).
const MAX_REVIEWER_NOTE_LEN: usize = 2000;
/// SLA 기간 (24시간).
const SLA_HOURS: i64 = 24;

/// 사업자 인증 큐 1건. 4-status workflow + version OCC.
///
/// `submitted_documents` 는 R2 객체 키들의 JSON 배열 형태 (예: `["bvq/abc/file1.pdf", ...]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BusinessVerificationQueue {
    /// 식별자 (`bvq_<26 ULID>`).
    pub id: Id<BvqMarker>,
    /// 제출 사용자 (FK → `user.id`).
    pub user_id: Id<UserMarker>,
    /// 사업자등록번호.
    pub business_number: BusinessNumber,
    /// 제출 문서 R2 keys (`JSONB`).
    pub submitted_documents: serde_json::Value,
    /// 현재 상태.
    pub status: BvqStatus,
    /// 검토 어드민 (FK → `user.id`). 검토 전에는 `None`.
    pub reviewer_id: Option<Id<UserMarker>>,
    /// 검토 메모 (≤2000자). `reject` / `request_more_info` 는 필수.
    pub reviewer_note: Option<String>,
    /// 제출 시각.
    pub submitted_at: DateTime<Utc>,
    /// 검토 완료 시각 (`approve` / `reject` / `request_more_info` 시 기록).
    pub reviewed_at: Option<DateTime<Utc>>,
    /// SLA 만료 시각 (`submitted_at + 24h`).
    pub sla_due_at: Option<DateTime<Utc>>,
    /// 마지막 갱신 시각 (DB `updated_at` 보조 필드).
    pub updated_at: DateTime<Utc>,
    /// Optimistic locking 버전.
    pub version: i64,
}

impl BusinessVerificationQueue {
    /// `Pending` 상태 신규 큐 생성. SLA = `submitted_at + 24h`.
    ///
    /// `status = Pending`, `reviewer_id = None`, `reviewer_note = None`,
    /// `reviewed_at = None`, `version = 1`. `updated_at == submitted_at`.
    #[must_use]
    pub fn try_new_pending(
        id: Id<BvqMarker>,
        user_id: Id<UserMarker>,
        business_number: BusinessNumber,
        submitted_documents: serde_json::Value,
        submitted_at: DateTime<Utc>,
    ) -> Self {
        let sla_due_at = submitted_at.checked_add_signed(Duration::hours(SLA_HOURS));
        Self {
            id,
            user_id,
            business_number,
            submitted_documents,
            status: BvqStatus::Pending,
            reviewer_id: None,
            reviewer_note: None,
            submitted_at,
            reviewed_at: None,
            sla_due_at,
            updated_at: submitted_at,
            version: 1,
        }
    }

    /// 내부 헬퍼 — 상태 전이 + `version` bump + `updated_at` 갱신.
    ///
    /// `BvqStatus::can_transition_to` 로 도메인 머신 검사.
    const fn transition_to(
        &mut self,
        target: BvqStatus,
        at: DateTime<Utc>,
    ) -> Result<(), BvqError> {
        if !self.status.can_transition_to(target) {
            return Err(BvqError::InvalidTransition {
                from: self.status,
                to: target,
            });
        }
        self.status = target;
        self.version += 1;
        self.updated_at = at;
        Ok(())
    }

    /// 검토 메모 정규화 + 검증 (≤2000자, `required_for` 가 `Some` 이면 비어있으면 안 됨).
    fn normalize_note(
        note: Option<String>,
        required_for: Option<&'static str>,
    ) -> Result<Option<String>, BvqError> {
        let trimmed = note.map(|n| n.trim().to_owned());
        match (trimmed, required_for) {
            (None, Some(action)) => Err(BvqError::EmptyReviewerNote { action }),
            (Some(s), Some(action)) if s.is_empty() => Err(BvqError::EmptyReviewerNote { action }),
            (Some(s), _) => {
                let len = s.chars().count();
                if len > MAX_REVIEWER_NOTE_LEN {
                    return Err(BvqError::ReviewerNoteTooLong { actual: len });
                }
                Ok(Some(s))
            }
            (None, None) => Ok(None),
        }
    }

    /// `Pending` → `Approved`. 어드민 승인 (메모는 선택).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Pending` 이 아니면 [`BvqError::InvalidTransition`].
    /// `note` 가 2000자 초과 시 [`BvqError::ReviewerNoteTooLong`].
    pub fn approve(
        &mut self,
        reviewer: Id<UserMarker>,
        note: Option<String>,
        at: DateTime<Utc>,
    ) -> Result<(), BvqError> {
        let normalized = Self::normalize_note(note, None)?;
        self.transition_to(BvqStatus::Approved, at)?;
        self.reviewer_id = Some(reviewer);
        self.reviewer_note = normalized;
        self.reviewed_at = Some(at);
        Ok(())
    }

    /// `Pending` → `Rejected`. 어드민 거부 (메모 필수).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Pending` 이 아니면 [`BvqError::InvalidTransition`].
    /// `note` 가 비어있으면 [`BvqError::EmptyReviewerNote`].
    /// `note` 가 2000자 초과 시 [`BvqError::ReviewerNoteTooLong`].
    pub fn reject(
        &mut self,
        reviewer: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), BvqError> {
        let normalized = Self::normalize_note(Some(note), Some("reject"))?;
        self.transition_to(BvqStatus::Rejected, at)?;
        self.reviewer_id = Some(reviewer);
        self.reviewer_note = normalized;
        self.reviewed_at = Some(at);
        Ok(())
    }

    /// `Pending` → `NeedsMoreInfo`. 어드민이 추가 자료 요청 (메모 필수).
    ///
    /// # Errors
    ///
    /// 현재 상태가 `Pending` 이 아니면 [`BvqError::InvalidTransition`].
    /// `note` 가 비어있으면 [`BvqError::EmptyReviewerNote`].
    /// `note` 가 2000자 초과 시 [`BvqError::ReviewerNoteTooLong`].
    pub fn request_more_info(
        &mut self,
        reviewer: Id<UserMarker>,
        note: String,
        at: DateTime<Utc>,
    ) -> Result<(), BvqError> {
        let normalized = Self::normalize_note(Some(note), Some("request_more_info"))?;
        self.transition_to(BvqStatus::NeedsMoreInfo, at)?;
        self.reviewer_id = Some(reviewer);
        self.reviewer_note = normalized;
        self.reviewed_at = Some(at);
        Ok(())
    }

    /// `NeedsMoreInfo` → `Pending`. 사용자가 새 문서로 재제출.
    ///
    /// `submitted_documents` 는 새 R2 keys 로 교체되고 reviewer 필드 (`reviewer_id`,
    /// `reviewer_note`, `reviewed_at`) 는 모두 `None` 으로 초기화돼요.
    ///
    /// # Errors
    ///
    /// 현재 상태가 `NeedsMoreInfo` 가 아니면 [`BvqError::InvalidTransition`].
    pub fn resubmit(
        &mut self,
        submitted_documents: serde_json::Value,
        at: DateTime<Utc>,
    ) -> Result<(), BvqError> {
        self.transition_to(BvqStatus::Pending, at)?;
        self.submitted_documents = submitted_documents;
        self.reviewer_id = None;
        self.reviewer_note = None;
        self.reviewed_at = None;
        Ok(())
    }
}

#[cfg(test)]
#[path = "entity_tests.rs"]
mod tests;
