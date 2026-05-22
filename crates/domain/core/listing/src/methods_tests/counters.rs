use shared_kernel::listing_status::ListingStatus;

use super::fixtures::{later, sample_draft, t0};

// ── increment_view_count ────────────────────────────────────────────────────

#[test]
fn increment_view_count_increments_and_does_not_bump_version() {
    let mut l = sample_draft(t0());
    let v_before = l.version;
    let when = later(1);
    l.increment_view_count(when);
    assert_eq!(l.view_count, 1);
    assert_eq!(l.version, v_before, "version must NOT bump");
    assert_eq!(l.updated_at, when);
}

#[test]
fn increment_view_count_repeated_accumulates() {
    let mut l = sample_draft(t0());
    for _ in 0..10 {
        l.increment_view_count(later(1));
    }
    assert_eq!(l.view_count, 10);
    assert_eq!(l.version, 1);
}

#[test]
fn increment_view_count_at_u64_max_saturates() {
    let mut l = sample_draft(t0());
    l.view_count = u64::MAX;
    l.increment_view_count(later(1));
    assert_eq!(l.view_count, u64::MAX, "saturating_add must not overflow");
}

// ── record_bookmark / release_bookmark (deprecated SP6-iii — 호출 보존) ─────
//
// SP6-iii 가 denormalized counter 를 JOIN COUNT 로 대체. 본 메서드들 호출자
// 0 이지만 FU 70 의 schema 컬럼 제거 시까지 유지. 본 모듈 단위 테스트 보존.
// ── record_bookmark ─────────────────────────────────────────────────────────

#[test]
fn record_bookmark_increments_and_does_not_bump_version() {
    let mut l = sample_draft(t0());
    let v_before = l.version;
    let when = later(1);
    l.record_bookmark(when);
    assert_eq!(l.bookmark_count, 1);
    assert_eq!(l.version, v_before);
    assert_eq!(l.updated_at, when);
}

#[test]
fn record_bookmark_at_u64_max_saturates() {
    let mut l = sample_draft(t0());
    l.bookmark_count = u64::MAX;
    l.record_bookmark(later(1));
    assert_eq!(l.bookmark_count, u64::MAX);
}

// ── release_bookmark ────────────────────────────────────────────────────────

#[test]
fn release_bookmark_decrements_and_does_not_bump_version() {
    let mut l = sample_draft(t0());
    l.bookmark_count = 3;
    let v_before = l.version;
    let when = later(1);
    l.release_bookmark(when);
    assert_eq!(l.bookmark_count, 2);
    assert_eq!(l.version, v_before);
    assert_eq!(l.updated_at, when);
}

#[test]
fn release_bookmark_at_zero_saturates_to_zero() {
    let mut l = sample_draft(t0());
    assert_eq!(l.bookmark_count, 0);
    l.release_bookmark(later(1));
    assert_eq!(l.bookmark_count, 0, "saturating_sub must not underflow");
}

// ── Full lifecycle (happy path) ─────────────────────────────────────────────

#[test]
fn full_lifecycle_draft_to_sold_bumps_version_each_step() {
    let mut l = sample_draft(t0());
    assert_eq!(l.version, 1);
    l.submit_for_review(later(1)).unwrap();
    assert_eq!(l.version, 2);
    l.approve(later(2)).unwrap();
    assert_eq!(l.version, 3);
    l.mark_sold(later(3)).unwrap();
    assert_eq!(l.version, 4);
    assert_eq!(l.status, ListingStatus::Sold);
}

#[test]
fn rejected_then_revise_then_resubmit_works() {
    let mut l = sample_draft(t0());
    l.submit_for_review(later(1)).unwrap();
    l.reject(later(2)).unwrap();
    assert_eq!(l.status, ListingStatus::Rejected);
    l.revise_after_rejection(later(3)).unwrap();
    assert_eq!(l.status, ListingStatus::Draft);
    l.submit_for_review(later(4)).unwrap();
    assert_eq!(l.status, ListingStatus::PendingReview);
}
