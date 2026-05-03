//! `User::try_new_full` (full 14-arg) 생성자 + `UserRole` + 전체 serde round-trip.

use chrono::Utc;
use shared_kernel::id::{Id, UserMarker};

use super::super::{User, UserKind, UserRole};
use super::fixtures::{
    sample_broker_license, sample_business_number, sample_email, SAMPLE_PHONE_HASH,
};
use crate::errors::UserError;

// ── try_new_full (full, 14 args) ──────────────────────────────────────────

#[test]
fn try_new_full_happy_path_all_some() {
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub-full",
        sample_email(),
        Some(SAMPLE_PHONE_HASH.to_owned()),
        "Alice",
        UserKind::Corporation,
        Some(sample_business_number()),
        Some(now),
        Some(sample_broker_license()),
        Some(now),
        vec![UserRole::Buyer, UserRole::Broker],
        Some(now),
        Some(now),
        now,
    )
    .expect("valid full");
    assert_eq!(user.phone_kr_hash.as_deref(), Some(SAMPLE_PHONE_HASH));
    assert!(user.business_number.is_some());
    assert_eq!(user.business_verified_at, Some(now));
    assert!(user.broker_license_number.is_some());
    assert_eq!(user.broker_verified_at, Some(now));
    assert_eq!(user.roles.len(), 2);
    assert_eq!(user.nice_verified_at, Some(now));
    assert_eq!(user.marketing_consent_at, Some(now));
    assert_eq!(user.last_login_at, None);
    assert_eq!(user.deleted_at, None);
    assert_eq!(user.version, 1);
}

#[test]
fn try_new_full_happy_path_all_none_matches_try_new() {
    let now = Utc::now();
    let id = Id::<UserMarker>::new();
    let from_full = User::try_new_full(
        id.clone(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .expect("valid");
    let from_min = User::try_new(
        id,
        "sub",
        sample_email(),
        "Alice",
        UserKind::Individual,
        now,
    )
    .expect("valid");
    assert_eq!(from_full, from_min);
}

#[test]
fn rejects_phone_hash_wrong_length() {
    let now = Utc::now();
    let too_short = "a".repeat(63);
    let err = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        Some(too_short),
        "Alice",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .unwrap_err();
    assert!(matches!(err, UserError::InvalidPhoneHash));
}

#[test]
fn rejects_phone_hash_non_hex() {
    let now = Utc::now();
    // 64 chars but contains 'g' (not hex).
    let bad = "g".repeat(64);
    let err = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        Some(bad),
        "Alice",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .unwrap_err();
    assert!(matches!(err, UserError::InvalidPhoneHash));
}

#[test]
fn accepts_uppercase_phone_hash() {
    // SHA-256 hex digits A-F should be accepted (is_ascii_hexdigit).
    let now = Utc::now();
    let upper = "9F86D081884C7D659A2FEAA0C55AD015A3BF4F1B2B0B822CD15D6C15B0F00A08";
    let user = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        Some(upper.to_owned()),
        "Alice",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .expect("uppercase hex OK");
    assert_eq!(user.phone_kr_hash.as_deref(), Some(upper));
}

#[test]
fn rejects_business_verified_without_business_number() {
    let now = Utc::now();
    let err = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Corporation,
        None, // no business_number
        Some(now),
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .unwrap_err();
    assert!(matches!(err, UserError::BusinessVerificationInconsistent));
}

#[test]
fn rejects_broker_verified_without_broker_license() {
    let now = Utc::now();
    let err = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Individual,
        None,
        None,
        None, // no broker_license_number
        Some(now),
        Vec::new(),
        None,
        None,
        now,
    )
    .unwrap_err();
    assert!(matches!(err, UserError::BrokerVerificationInconsistent));
}

#[test]
fn accepts_business_verification_with_number() {
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Corporation,
        Some(sample_business_number()),
        Some(now),
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .expect("valid");
    assert!(user.business_number.is_some());
    assert_eq!(user.business_verified_at, Some(now));
}

#[test]
fn accepts_broker_verification_with_license() {
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Individual,
        None,
        None,
        Some(sample_broker_license()),
        Some(now),
        vec![UserRole::Broker],
        None,
        None,
        now,
    )
    .expect("valid");
    assert!(user.broker_license_number.is_some());
    assert_eq!(user.broker_verified_at, Some(now));
}

#[test]
fn business_number_some_without_verified_is_ok() {
    // 사업자번호는 입력했지만 아직 검증 전 — 합법.
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Corporation,
        Some(sample_business_number()),
        None, // not yet verified
        None,
        None,
        Vec::new(),
        None,
        None,
        now,
    )
    .expect("valid");
    assert!(user.business_number.is_some());
    assert_eq!(user.business_verified_at, None);
}

#[test]
fn roles_with_multiple_values() {
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub",
        sample_email(),
        None,
        "Alice",
        UserKind::Individual,
        None,
        None,
        None,
        None,
        vec![UserRole::Buyer, UserRole::Seller, UserRole::Operator],
        None,
        None,
        now,
    )
    .expect("valid");
    assert_eq!(user.roles.len(), 3);
    assert_eq!(user.roles[0], UserRole::Buyer);
    assert_eq!(user.roles[1], UserRole::Seller);
    assert_eq!(user.roles[2], UserRole::Operator);
}

// ── UserRole ──────────────────────────────────────────────────────────────

#[test]
fn user_role_as_str_all_variants() {
    assert_eq!(UserRole::Buyer.as_str(), "Buyer");
    assert_eq!(UserRole::Seller.as_str(), "Seller");
    assert_eq!(UserRole::Broker.as_str(), "Broker");
    assert_eq!(UserRole::Developer.as_str(), "Developer");
    assert_eq!(UserRole::Enterprise.as_str(), "Enterprise");
    assert_eq!(UserRole::Operator.as_str(), "Operator");
    assert_eq!(UserRole::Admin.as_str(), "Admin");
}

#[test]
fn user_role_serde_roundtrip() {
    for role in [
        UserRole::Buyer,
        UserRole::Seller,
        UserRole::Broker,
        UserRole::Developer,
        UserRole::Enterprise,
        UserRole::Operator,
        UserRole::Admin,
    ] {
        let json = serde_json::to_string(&role).expect("serialize");
        let back: UserRole = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(role, back);
    }
}

// ── Full serde round-trip ─────────────────────────────────────────────────

#[test]
fn user_serde_roundtrip_full() {
    let now = Utc::now();
    let user = User::try_new_full(
        Id::new(),
        "sub-full",
        sample_email(),
        Some(SAMPLE_PHONE_HASH.to_owned()),
        "Alice",
        UserKind::Corporation,
        Some(sample_business_number()),
        Some(now),
        Some(sample_broker_license()),
        Some(now),
        vec![UserRole::Buyer, UserRole::Broker],
        Some(now),
        Some(now),
        now,
    )
    .expect("valid");
    let json = serde_json::to_string(&user).expect("serialize");
    let back: User = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(user, back);
}
