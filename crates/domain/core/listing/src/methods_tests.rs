//! `Listing` 도메인 메서드 단위 테스트 — 상태 전이 + counter.
//!
//! `entity.rs`에서 `#[path = "methods_tests.rs"] mod methods_tests;`로 포함.
//! 파일 자체가 테스트 모듈이라 별도 `mod tests {}` 래퍼 없어요.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{DateTime, Duration, TimeZone, Utc};
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;

use super::Listing;
use crate::errors::ListingError;

// ── Fixtures ────────────────────────────────────────────────────────────────

fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()
}

fn later(secs: i64) -> DateTime<Utc> {
    t0() + Duration::seconds(secs)
}

/// `Draft` 상태 샘플 매물 — `Sale` 거래.
fn sample_draft(now: DateTime<Utc>) -> Listing {
    Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        Pnu::try_new("1111010100100010000").expect("pnu"),
        ListingType::Factory,
        TransactionType::Sale,
        MoneyKrw::try_new(500_000_000).expect("price"),
        None,
        None,
        AreaM2::try_new(250.0).expect("area"),
        ListingTitle::try_new("샘플 매물").expect("title"),
        Description::try_new("위치 좋아요.").expect("desc"),
        None,
        now,
    )
    .expect("valid draft")
}

/// `PendingReview` 상태 매물 (Draft → `submit_for_review`).
fn pending_review(now: DateTime<Utc>) -> Listing {
    let mut l = sample_draft(now);
    l.submit_for_review(now).expect("draft -> pending");
    l
}

/// `Active` 상태 매물 (Draft → `PendingReview` → Active).
fn active(now: DateTime<Utc>) -> Listing {
    let mut l = pending_review(now);
    l.approve(now).expect("pending -> active");
    l
}

/// `Sold` 상태 매물 (Active → Sold).
fn sold(now: DateTime<Utc>) -> Listing {
    let mut l = active(now);
    l.mark_sold(now).expect("active -> sold");
    l
}

/// `Expired` 상태 매물 (Active → Expired).
fn expired(now: DateTime<Utc>) -> Listing {
    let mut l = active(now);
    l.expire(now).expect("active -> expired");
    l
}

/// `Rejected` 상태 매물 (Draft → `PendingReview` → Rejected).
fn rejected(now: DateTime<Utc>) -> Listing {
    let mut l = pending_review(now);
    l.reject(now).expect("pending -> rejected");
    l
}

// ── submit_for_review ───────────────────────────────────────────────────────

#[test]
fn submit_for_review_from_draft_succeeds_and_bumps_version() {
    let mut l = sample_draft(t0());
    let when = later(60);
    l.submit_for_review(when).expect("ok");
    assert_eq!(l.status, ListingStatus::PendingReview);
    assert_eq!(l.version, 2);
    assert_eq!(l.updated_at, when);
}

#[test]
fn submit_for_review_from_active_fails() {
    let mut l = active(t0());
    let err = l.submit_for_review(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Active,
            to: ListingStatus::PendingReview,
        }
    ));
}

#[test]
fn submit_for_review_from_sold_fails() {
    let mut l = sold(t0());
    let err = l.submit_for_review(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Sold,
            to: ListingStatus::PendingReview,
        }
    ));
}

// ── approve ─────────────────────────────────────────────────────────────────

#[test]
fn approve_from_pending_review_succeeds() {
    let mut l = pending_review(t0());
    let v_before = l.version;
    let when = later(120);
    l.approve(when).expect("ok");
    assert_eq!(l.status, ListingStatus::Active);
    assert_eq!(l.version, v_before + 1);
    assert_eq!(l.updated_at, when);
}

#[test]
fn approve_from_draft_fails() {
    let mut l = sample_draft(t0());
    let err = l.approve(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Draft,
            to: ListingStatus::Active,
        }
    ));
}

// ── reject ──────────────────────────────────────────────────────────────────

#[test]
fn reject_from_pending_review_succeeds() {
    let mut l = pending_review(t0());
    let v_before = l.version;
    l.reject(later(60)).expect("ok");
    assert_eq!(l.status, ListingStatus::Rejected);
    assert_eq!(l.version, v_before + 1);
}

#[test]
fn reject_from_draft_fails() {
    let mut l = sample_draft(t0());
    let err = l.reject(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Draft,
            to: ListingStatus::Rejected,
        }
    ));
}

// ── revise_after_rejection ──────────────────────────────────────────────────

#[test]
fn revise_after_rejection_from_rejected_returns_to_draft() {
    let mut l = rejected(t0());
    let v_before = l.version;
    l.revise_after_rejection(later(60)).expect("ok");
    assert_eq!(l.status, ListingStatus::Draft);
    assert_eq!(l.version, v_before + 1);
}

#[test]
fn revise_after_rejection_from_draft_fails() {
    let mut l = sample_draft(t0());
    let err = l.revise_after_rejection(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Draft,
            to: ListingStatus::Draft,
        }
    ));
}

// ── mark_sold ───────────────────────────────────────────────────────────────

#[test]
fn mark_sold_from_active_succeeds() {
    let mut l = active(t0());
    let v_before = l.version;
    l.mark_sold(later(60)).expect("ok");
    assert_eq!(l.status, ListingStatus::Sold);
    assert_eq!(l.version, v_before + 1);
}

#[test]
fn mark_sold_from_pending_review_fails() {
    let mut l = pending_review(t0());
    let err = l.mark_sold(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::PendingReview,
            to: ListingStatus::Sold,
        }
    ));
}

#[test]
fn mark_sold_from_sold_fails() {
    let mut l = sold(t0());
    let err = l.mark_sold(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Sold,
            to: ListingStatus::Sold,
        }
    ));
}

// ── expire ──────────────────────────────────────────────────────────────────

#[test]
fn expire_from_active_succeeds() {
    let mut l = active(t0());
    let v_before = l.version;
    l.expire(later(60)).expect("ok");
    assert_eq!(l.status, ListingStatus::Expired);
    assert_eq!(l.version, v_before + 1);
}

#[test]
fn expire_from_draft_fails() {
    let mut l = sample_draft(t0());
    let err = l.expire(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Draft,
            to: ListingStatus::Expired,
        }
    ));
}

#[test]
fn expire_from_expired_fails() {
    let mut l = expired(t0());
    let err = l.expire(later(60)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::InvalidTransition {
            from: ListingStatus::Expired,
            to: ListingStatus::Expired,
        }
    ));
}

// ── Terminal states reject all transitions ───────────────────────────────────

#[test]
fn sold_rejects_all_transitions() {
    let now = t0();
    for op in [
        Listing::submit_for_review as fn(&mut Listing, DateTime<Utc>) -> _,
        Listing::approve,
        Listing::reject,
        Listing::revise_after_rejection,
        Listing::mark_sold,
        Listing::expire,
    ] {
        let mut l = sold(now);
        assert!(op(&mut l, later(10)).is_err(), "Sold should reject all");
    }
}

#[test]
fn expired_rejects_all_transitions() {
    let now = t0();
    for op in [
        Listing::submit_for_review as fn(&mut Listing, DateTime<Utc>) -> _,
        Listing::approve,
        Listing::reject,
        Listing::revise_after_rejection,
        Listing::mark_sold,
        Listing::expire,
    ] {
        let mut l = expired(now);
        assert!(op(&mut l, later(10)).is_err(), "Expired should reject all");
    }
}

// ── Failure must NOT mutate state/version ───────────────────────────────────

#[test]
fn invalid_transition_does_not_mutate_state_or_version() {
    let mut l = sample_draft(t0());
    let v_before = l.version;
    let updated_before = l.updated_at;
    let _ = l.approve(later(60)).unwrap_err();
    assert_eq!(l.status, ListingStatus::Draft);
    assert_eq!(l.version, v_before);
    assert_eq!(l.updated_at, updated_before);
}

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
