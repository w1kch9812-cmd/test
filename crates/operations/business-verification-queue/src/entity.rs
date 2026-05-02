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
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn sample_business_number() -> BusinessNumber {
        // 표본: 첫 3자리 ≥ 101, NTS 체크섬 OK (shared-kernel 테스트와 동일한 값).
        BusinessNumber::try_new("1234567891").expect("valid sample BN")
    }

    fn sample_documents() -> serde_json::Value {
        serde_json::json!(["bvq/abc/biz_reg.pdf", "bvq/abc/cert.png"])
    }

    fn sample_documents_v2() -> serde_json::Value {
        serde_json::json!(["bvq/abc/biz_reg_v2.pdf"])
    }

    fn make_pending(at: DateTime<Utc>) -> BusinessVerificationQueue {
        BusinessVerificationQueue::try_new_pending(
            Id::new(),
            Id::new(),
            sample_business_number(),
            sample_documents(),
            at,
        )
    }

    // ── try_new_pending ───────────────────────────────────────────

    #[test]
    fn try_new_pending_initial_status_is_pending() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert_eq!(bvq.status, BvqStatus::Pending);
    }

    #[test]
    fn try_new_pending_version_is_1() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert_eq!(bvq.version, 1);
    }

    #[test]
    fn try_new_pending_sla_is_submitted_plus_24h() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert_eq!(bvq.sla_due_at, Some(now + Duration::hours(24)));
    }

    #[test]
    fn try_new_pending_reviewer_fields_are_none() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert!(bvq.reviewer_id.is_none());
        assert!(bvq.reviewer_note.is_none());
        assert!(bvq.reviewed_at.is_none());
    }

    #[test]
    fn try_new_pending_updated_at_equals_submitted_at() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert_eq!(bvq.updated_at, now);
        assert_eq!(bvq.submitted_at, now);
    }

    #[test]
    fn try_new_pending_documents_preserved() {
        let now = Utc::now();
        let bvq = make_pending(now);
        assert_eq!(bvq.submitted_documents, sample_documents());
    }

    // ── approve ───────────────────────────────────────────────────

    #[test]
    fn approve_happy_path_transitions_and_records_reviewer() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(1);
        bvq.approve(reviewer.clone(), Some("OK".to_owned()), later)
            .expect("approve ok");
        assert_eq!(bvq.status, BvqStatus::Approved);
        assert_eq!(bvq.reviewer_id, Some(reviewer));
        assert_eq!(bvq.reviewer_note.as_deref(), Some("OK"));
        assert_eq!(bvq.reviewed_at, Some(later));
        assert_eq!(bvq.updated_at, later);
    }

    #[test]
    fn approve_bumps_version() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let v0 = bvq.version;
        bvq.approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        assert_eq!(bvq.version, v0 + 1);
    }

    #[test]
    fn approve_accepts_none_note() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve with none note ok");
        assert_eq!(bvq.status, BvqStatus::Approved);
        assert!(bvq.reviewer_note.is_none());
    }

    // ── reject ────────────────────────────────────────────────────

    #[test]
    fn reject_happy_path_transitions_and_records_note() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(2);
        bvq.reject(
            reviewer.clone(),
            "사업자등록증 위조 의심돼요".to_owned(),
            later,
        )
        .expect("reject ok");
        assert_eq!(bvq.status, BvqStatus::Rejected);
        assert_eq!(bvq.reviewer_id, Some(reviewer));
        assert_eq!(
            bvq.reviewer_note.as_deref(),
            Some("사업자등록증 위조 의심돼요")
        );
        assert_eq!(bvq.reviewed_at, Some(later));
        assert_eq!(bvq.version, 2);
    }

    #[test]
    fn reject_without_note_errors() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let err = bvq
            .reject(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::EmptyReviewerNote { action: "reject" }
        ));
        // 상태가 그대로여야 해요 (전이 실패 시 mutation 0).
        assert_eq!(bvq.status, BvqStatus::Pending);
        assert_eq!(bvq.version, 1);
    }

    #[test]
    fn reject_with_whitespace_only_note_errors() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let err = bvq
            .reject(Id::new(), "   ".to_owned(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::EmptyReviewerNote { action: "reject" }
        ));
    }

    // ── request_more_info ─────────────────────────────────────────

    #[test]
    fn request_more_info_happy_path() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(3);
        bvq.request_more_info(reviewer.clone(), "추가 서류 필요해요".to_owned(), later)
            .expect("request_more_info ok");
        assert_eq!(bvq.status, BvqStatus::NeedsMoreInfo);
        assert_eq!(bvq.reviewer_id, Some(reviewer));
        assert_eq!(bvq.reviewer_note.as_deref(), Some("추가 서류 필요해요"));
        assert_eq!(bvq.reviewed_at, Some(later));
        assert_eq!(bvq.version, 2);
    }

    #[test]
    fn request_more_info_without_note_errors() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let err = bvq
            .request_more_info(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::EmptyReviewerNote {
                action: "request_more_info"
            }
        ));
        assert_eq!(bvq.status, BvqStatus::Pending);
    }

    // ── resubmit ──────────────────────────────────────────────────

    #[test]
    fn resubmit_clears_reviewer_and_replaces_documents() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        // 먼저 NeedsMoreInfo 로 보낸 뒤 resubmit.
        bvq.request_more_info(
            Id::new(),
            "더 필요해요".to_owned(),
            now + Duration::hours(1),
        )
        .expect("rmi ok");
        assert_eq!(bvq.status, BvqStatus::NeedsMoreInfo);
        assert!(bvq.reviewer_id.is_some());
        assert!(bvq.reviewer_note.is_some());
        assert!(bvq.reviewed_at.is_some());

        let resubmit_at = now + Duration::hours(5);
        bvq.resubmit(sample_documents_v2(), resubmit_at)
            .expect("resubmit ok");
        assert_eq!(bvq.status, BvqStatus::Pending);
        assert_eq!(bvq.submitted_documents, sample_documents_v2());
        assert!(bvq.reviewer_id.is_none());
        assert!(bvq.reviewer_note.is_none());
        assert!(bvq.reviewed_at.is_none());
        assert_eq!(bvq.updated_at, resubmit_at);
    }

    #[test]
    fn resubmit_bumps_version() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.request_more_info(
            Id::new(),
            "더 필요해요".to_owned(),
            now + Duration::hours(1),
        )
        .expect("rmi ok");
        let v_before_resubmit = bvq.version;
        bvq.resubmit(sample_documents_v2(), now + Duration::hours(2))
            .expect("resubmit ok");
        assert_eq!(bvq.version, v_before_resubmit + 1);
    }

    // ── 4 disallowed transitions ──────────────────────────────────

    #[test]
    fn approved_terminal_cannot_be_rejected() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        let err = bvq
            .reject(Id::new(), "too late".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::InvalidTransition {
                from: BvqStatus::Approved,
                to: BvqStatus::Rejected
            }
        ));
    }

    #[test]
    fn approved_terminal_cannot_request_more_info() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        let err = bvq
            .request_more_info(Id::new(), "more".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::InvalidTransition {
                from: BvqStatus::Approved,
                to: BvqStatus::NeedsMoreInfo
            }
        ));
    }

    #[test]
    fn rejected_terminal_cannot_be_approved() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.reject(Id::new(), "no good".to_owned(), now + Duration::hours(1))
            .expect("reject ok");
        let err = bvq
            .approve(Id::new(), None, now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::InvalidTransition {
                from: BvqStatus::Rejected,
                to: BvqStatus::Approved
            }
        ));
    }

    #[test]
    fn rejected_terminal_cannot_be_resubmitted() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.reject(Id::new(), "no good".to_owned(), now + Duration::hours(1))
            .expect("reject ok");
        let err = bvq
            .resubmit(sample_documents_v2(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::InvalidTransition {
                from: BvqStatus::Rejected,
                to: BvqStatus::Pending
            }
        ));
    }

    // ── reviewer_note 길이 ────────────────────────────────────────

    #[test]
    fn reject_with_2000_char_note_accepted() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let exactly = "X".repeat(2000);
        bvq.reject(Id::new(), exactly.clone(), now + Duration::hours(1))
            .expect("2000 ok");
        assert_eq!(bvq.reviewer_note.as_deref(), Some(exactly.as_str()));
    }

    #[test]
    fn reject_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let too_long = "X".repeat(2001);
        let err = bvq
            .reject(Id::new(), too_long, now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::ReviewerNoteTooLong { actual: 2001 }
        ));
        // 전이 전에 검증 실패 — 상태 유지.
        assert_eq!(bvq.status, BvqStatus::Pending);
    }

    #[test]
    fn approve_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        let too_long = "X".repeat(2001);
        let err = bvq
            .approve(Id::new(), Some(too_long), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            BvqError::ReviewerNoteTooLong { actual: 2001 }
        ));
        assert_eq!(bvq.status, BvqStatus::Pending);
    }

    // ── 상태 머신 happy → terminal cycle ──────────────────────────

    #[test]
    fn full_cycle_rmi_then_resubmit_then_approve() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.request_more_info(Id::new(), "더 필요".to_owned(), now + Duration::hours(1))
            .expect("rmi ok");
        bvq.resubmit(sample_documents_v2(), now + Duration::hours(2))
            .expect("resubmit ok");
        assert_eq!(bvq.status, BvqStatus::Pending);
        bvq.approve(Id::new(), None, now + Duration::hours(3))
            .expect("approve ok");
        assert_eq!(bvq.status, BvqStatus::Approved);
        assert_eq!(bvq.version, 4); // 1 + 3 transitions
    }

    // ── serde ──────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_pending() {
        let now = Utc::now();
        let bvq = make_pending(now);
        let json = serde_json::to_string(&bvq).expect("serialize");
        let back: BusinessVerificationQueue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bvq, back);
    }

    #[test]
    fn serde_roundtrip_approved() {
        let now = Utc::now();
        let mut bvq = make_pending(now);
        bvq.approve(Id::new(), Some("OK".to_owned()), now + Duration::hours(1))
            .expect("approve ok");
        let json = serde_json::to_string(&bvq).expect("serialize");
        let back: BusinessVerificationQueue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(bvq, back);
    }
}
