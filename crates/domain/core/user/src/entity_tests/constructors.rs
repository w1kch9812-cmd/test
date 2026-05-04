//! `User::try_new` (minimal 6-arg) 생성자 + `UserKind` 동작 검증.

use chrono::Utc;
use shared_kernel::id::{Id, UserMarker};

use super::super::{User, UserKind};
use super::fixtures::sample_email;
use crate::errors::UserError;

// ── try_new (minimal, 6 args) ─────────────────────────────────────────────

#[allow(clippy::cognitive_complexity)]
#[test]
fn try_new_happy_path() {
    let id = Id::<UserMarker>::new();
    let now = Utc::now();
    let user = User::try_new(
        id.clone(),
        "zitadel-sub-1",
        sample_email(),
        "Alice",
        UserKind::Individual,
        now,
    )
    .expect("valid");
    assert_eq!(user.id, id);
    assert_eq!(user.display_name, "Alice");
    assert_eq!(user.version, 1);
    assert_eq!(user.created_at, now);
    assert_eq!(user.updated_at, now);
    // Optional fields default
    assert!(user.phone_kr_hash.is_none());
    assert!(user.business_number.is_none());
    assert!(user.business_verified_at.is_none());
    assert!(user.broker_license_number.is_none());
    assert!(user.broker_verified_at.is_none());
    assert!(user.roles.is_empty());
    assert!(user.nice_verified_at.is_none());
    assert!(user.marketing_consent_at.is_none());
    assert!(user.last_login_at.is_none());
    assert!(user.deleted_at.is_none());
}

#[test]
fn rejects_empty_display_name() {
    let err = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        "",
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::EmptyDisplayName));
}

#[test]
fn rejects_whitespace_only_display_name() {
    let err = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        "   ",
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::EmptyDisplayName));
}

#[test]
fn rejects_too_long_display_name() {
    let long = "X".repeat(101);
    let err = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        &long,
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::DisplayNameTooLong { actual: 101 }));
}

#[test]
fn accepts_exactly_100_char_display_name() {
    let exactly = "X".repeat(100);
    let user = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        &exactly,
        UserKind::Individual,
        Utc::now(),
    )
    .expect("100 chars OK");
    assert_eq!(user.display_name, exactly);
}

#[test]
fn rejects_empty_zitadel_sub() {
    let err = User::try_new(
        Id::new(),
        "",
        sample_email(),
        "Alice",
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::EmptyZitadelSub));
}

#[test]
fn rejects_whitespace_only_zitadel_sub() {
    let err = User::try_new(
        Id::new(),
        "   ",
        sample_email(),
        "Alice",
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::EmptyZitadelSub));
}

#[test]
fn rejects_too_long_zitadel_sub() {
    let long = "X".repeat(256);
    let err = User::try_new(
        Id::new(),
        &long,
        sample_email(),
        "Alice",
        UserKind::Individual,
        Utc::now(),
    )
    .unwrap_err();
    assert!(matches!(err, UserError::ZitadelSubTooLong { actual: 256 }));
}

#[test]
fn accepts_exactly_255_char_zitadel_sub() {
    let exactly = "X".repeat(255);
    let user = User::try_new(
        Id::new(),
        &exactly,
        sample_email(),
        "Alice",
        UserKind::Individual,
        Utc::now(),
    )
    .expect("255 chars OK");
    assert_eq!(user.zitadel_sub, exactly);
}

#[test]
fn user_kind_serializes_snake_case() {
    let json = serde_json::to_string(&UserKind::Individual).expect("ok");
    assert_eq!(json, r#""individual""#);
    let json = serde_json::to_string(&UserKind::Corporation).expect("ok");
    assert_eq!(json, r#""corporation""#);
}

#[test]
fn user_kind_as_str() {
    assert_eq!(UserKind::Individual.as_str(), "individual");
    assert_eq!(UserKind::Corporation.as_str(), "corporation");
}

#[test]
fn user_serde_roundtrip_minimal() {
    let now = Utc::now();
    let user = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        "Alice",
        UserKind::Individual,
        now,
    )
    .expect("valid");
    let json = serde_json::to_string(&user).expect("serialize");
    let back: User = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(user, back);
}

#[test]
fn corporation_user_kind_works() {
    let user = User::try_new(
        Id::new(),
        "sub",
        sample_email(),
        "Acme Co.",
        UserKind::Corporation,
        Utc::now(),
    )
    .expect("valid");
    assert_eq!(user.user_kind, UserKind::Corporation);
}
