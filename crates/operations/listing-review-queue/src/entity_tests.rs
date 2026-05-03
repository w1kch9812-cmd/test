//! `ListingReviewQueue` Aggregate 테스트 (entity 가 500 줄 임계 근접 — `#[path]` 분리).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn sample_flags() -> serde_json::Value {
    serde_json::json!(["suspected_duplicate", "price_anomaly"])
}

fn make_pending(at: DateTime<Utc>) -> ListingReviewQueue {
    ListingReviewQueue::try_new_pending(Id::new(), Id::new(), Some(80), Some(sample_flags()), at)
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
    let err = ListingReviewQueue::try_new_pending(Id::new(), Id::new(), Some(101), None, now)
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
