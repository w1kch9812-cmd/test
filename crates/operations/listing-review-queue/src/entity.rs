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
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn sample_flags() -> serde_json::Value {
        serde_json::json!(["suspected_duplicate", "price_anomaly"])
    }

    fn make_pending(at: DateTime<Utc>) -> ListingReviewQueue {
        ListingReviewQueue::try_new_pending(
            Id::new(),
            Id::new(),
            Some(80),
            Some(sample_flags()),
            at,
        )
        .expect("valid pending lrq")
    }

    // ── try_new_pending ───────────────────────────────────────────

    #[test]
    fn try_new_pending_decision_is_none() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert!(lrq.decision.is_none());
        assert!(lrq.is_pending());
    }

    #[test]
    fn try_new_pending_version_is_1() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert_eq!(lrq.version, 1);
    }

    #[test]
    fn try_new_pending_sla_is_submitted_plus_12h() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert_eq!(lrq.sla_due_at, Some(now + Duration::hours(12)));
    }

    #[test]
    fn try_new_pending_reviewer_fields_are_none() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert!(lrq.reviewer_id.is_none());
        assert!(lrq.reviewer_note.is_none());
        assert!(lrq.decided_at.is_none());
    }

    #[test]
    fn try_new_pending_updated_at_equals_submitted_at() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert_eq!(lrq.updated_at, now);
        assert_eq!(lrq.submitted_at, now);
    }

    #[test]
    fn try_new_pending_auto_check_preserved() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert_eq!(lrq.auto_check_score, Some(80));
        assert_eq!(lrq.auto_check_flags.as_ref(), Some(&sample_flags()));
    }

    #[test]
    fn try_new_pending_accepts_none_auto_check() {
        let now = Utc::now();
        let lrq = ListingReviewQueue::try_new_pending(Id::new(), Id::new(), None, None, now)
            .expect("none ok");
        assert!(lrq.auto_check_score.is_none());
        assert!(lrq.auto_check_flags.is_none());
        assert!(lrq.is_pending());
    }

    // ── auto_check_score boundary ─────────────────────────────────

    #[test]
    fn try_new_pending_score_0_accepted() {
        let now = Utc::now();
        let lrq = ListingReviewQueue::try_new_pending(Id::new(), Id::new(), Some(0), None, now)
            .expect("score 0 ok");
        assert_eq!(lrq.auto_check_score, Some(0));
    }

    #[test]
    fn try_new_pending_score_100_accepted() {
        let now = Utc::now();
        let lrq = ListingReviewQueue::try_new_pending(Id::new(), Id::new(), Some(100), None, now)
            .expect("score 100 ok");
        assert_eq!(lrq.auto_check_score, Some(100));
    }

    #[test]
    fn try_new_pending_score_101_errors() {
        let now = Utc::now();
        let err =
            ListingReviewQueue::try_new_pending(Id::new(), Id::new(), Some(101), None, now)
                .unwrap_err();
        assert!(matches!(
            err,
            LrqError::AutoCheckScoreOutOfRange { actual: 101 }
        ));
    }

    // ── is_pending ────────────────────────────────────────────────

    #[test]
    fn is_pending_true_before_decision() {
        let now = Utc::now();
        let lrq = make_pending(now);
        assert!(lrq.is_pending());
    }

    #[test]
    fn is_pending_false_after_approve() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        assert!(!lrq.is_pending());
    }

    // ── decide_approve ────────────────────────────────────────────

    #[test]
    fn approve_happy_path_records_reviewer_and_decision() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(1);
        lrq.decide_approve(reviewer.clone(), Some("OK".to_owned()), later)
            .expect("approve ok");
        assert_eq!(lrq.decision, Some(LrqDecision::Approve));
        assert_eq!(lrq.reviewer_id, Some(reviewer));
        assert_eq!(lrq.reviewer_note.as_deref(), Some("OK"));
        assert_eq!(lrq.decided_at, Some(later));
        assert_eq!(lrq.updated_at, later);
    }

    #[test]
    fn approve_bumps_version() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let v0 = lrq.version;
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        assert_eq!(lrq.version, v0 + 1);
    }

    #[test]
    fn approve_accepts_none_note() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve with none note ok");
        assert_eq!(lrq.decision, Some(LrqDecision::Approve));
        assert!(lrq.reviewer_note.is_none());
    }

    // ── decide_reject ─────────────────────────────────────────────

    #[test]
    fn reject_happy_path_records_note() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(2);
        lrq.decide_reject(reviewer.clone(), "허위 매물 의심돼요".to_owned(), later)
            .expect("reject ok");
        assert_eq!(lrq.decision, Some(LrqDecision::Reject));
        assert_eq!(lrq.reviewer_id, Some(reviewer));
        assert_eq!(lrq.reviewer_note.as_deref(), Some("허위 매물 의심돼요"));
        assert_eq!(lrq.decided_at, Some(later));
        assert_eq!(lrq.version, 2);
    }

    #[test]
    fn reject_without_note_errors() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let err = lrq
            .decide_reject(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            LrqError::EmptyReviewerNote { action: "reject" }
        ));
        // 결정 전 검증 실패 — pending 그대로.
        assert!(lrq.is_pending());
        assert_eq!(lrq.version, 1);
    }

    #[test]
    fn reject_with_whitespace_only_note_errors() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let err = lrq
            .decide_reject(Id::new(), "   ".to_owned(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            LrqError::EmptyReviewerNote { action: "reject" }
        ));
    }

    // ── decide_request_changes ────────────────────────────────────

    #[test]
    fn request_changes_happy_path() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let reviewer = Id::<UserMarker>::new();
        let later = now + Duration::hours(3);
        lrq.decide_request_changes(reviewer.clone(), "사진 다시 올려 주세요".to_owned(), later)
            .expect("request_changes ok");
        assert_eq!(lrq.decision, Some(LrqDecision::RequestChanges));
        assert_eq!(lrq.reviewer_id, Some(reviewer));
        assert_eq!(lrq.reviewer_note.as_deref(), Some("사진 다시 올려 주세요"));
        assert_eq!(lrq.decided_at, Some(later));
        assert_eq!(lrq.version, 2);
    }

    #[test]
    fn request_changes_without_note_errors() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let err = lrq
            .decide_request_changes(Id::new(), String::new(), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            LrqError::EmptyReviewerNote {
                action: "request_changes"
            }
        ));
        assert!(lrq.is_pending());
    }

    // ── once-only (AlreadyDecided) ────────────────────────────────

    #[test]
    fn approved_cannot_be_decided_again_to_reject() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        let err = lrq
            .decide_reject(Id::new(), "too late".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, LrqError::AlreadyDecided));
        assert_eq!(lrq.decision, Some(LrqDecision::Approve));
    }

    #[test]
    fn approved_cannot_be_decided_again_to_request_changes() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        let err = lrq
            .decide_request_changes(Id::new(), "more".to_owned(), now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, LrqError::AlreadyDecided));
    }

    #[test]
    fn rejected_cannot_be_decided_again_to_approve() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_reject(Id::new(), "no good".to_owned(), now + Duration::hours(1))
            .expect("reject ok");
        let err = lrq
            .decide_approve(Id::new(), None, now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, LrqError::AlreadyDecided));
    }

    #[test]
    fn approved_cannot_be_approved_again() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), None, now + Duration::hours(1))
            .expect("approve ok");
        let err = lrq
            .decide_approve(Id::new(), None, now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, LrqError::AlreadyDecided));
    }

    #[test]
    fn request_changes_cannot_be_decided_again() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_request_changes(Id::new(), "fix it".to_owned(), now + Duration::hours(1))
            .expect("rc ok");
        let err = lrq
            .decide_approve(Id::new(), None, now + Duration::hours(2))
            .unwrap_err();
        assert!(matches!(err, LrqError::AlreadyDecided));
    }

    // ── reviewer_note 길이 ────────────────────────────────────────

    #[test]
    fn reject_with_2000_char_note_accepted() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let exactly = "X".repeat(2000);
        lrq.decide_reject(Id::new(), exactly.clone(), now + Duration::hours(1))
            .expect("2000 ok");
        assert_eq!(lrq.reviewer_note.as_deref(), Some(exactly.as_str()));
    }

    #[test]
    fn reject_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let too_long = "X".repeat(2001);
        let err = lrq
            .decide_reject(Id::new(), too_long, now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            LrqError::ReviewerNoteTooLong { actual: 2001 }
        ));
        // 결정 전 검증 실패 — pending 유지.
        assert!(lrq.is_pending());
    }

    #[test]
    fn approve_with_2001_char_note_errors() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        let too_long = "X".repeat(2001);
        let err = lrq
            .decide_approve(Id::new(), Some(too_long), now + Duration::hours(1))
            .unwrap_err();
        assert!(matches!(
            err,
            LrqError::ReviewerNoteTooLong { actual: 2001 }
        ));
        assert!(lrq.is_pending());
    }

    // ── serde ──────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_pending() {
        let now = Utc::now();
        let lrq = make_pending(now);
        let json = serde_json::to_string(&lrq).expect("serialize");
        let back: ListingReviewQueue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(lrq, back);
    }

    #[test]
    fn serde_roundtrip_approved() {
        let now = Utc::now();
        let mut lrq = make_pending(now);
        lrq.decide_approve(Id::new(), Some("OK".to_owned()), now + Duration::hours(1))
            .expect("approve ok");
        let json = serde_json::to_string(&lrq).expect("serialize");
        let back: ListingReviewQueue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(lrq, back);
    }
}
