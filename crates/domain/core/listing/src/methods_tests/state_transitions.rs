use chrono::{DateTime, Utc};
use shared_kernel::listing_status::ListingStatus;

use super::super::Listing;
use super::fixtures::{active, expired, later, pending_review, rejected, sample_draft, sold, t0};
use crate::errors::ListingError;

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
