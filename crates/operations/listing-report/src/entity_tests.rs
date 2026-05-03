//! `ListingReport` Aggregate 테스트 (entity 가 500 줄 임계 근접 — `#[path]` 분리).

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
