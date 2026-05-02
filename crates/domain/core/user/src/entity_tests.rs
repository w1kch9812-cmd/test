//! `User` Aggregate 단위 테스트 — `entity.rs`/`methods.rs`/`errors.rs` 동작 검증.
//!
//! `entity.rs`에서 `#[path = "entity_tests.rs"] mod tests;` 형태로 포함해요. 파일 자체가
//! 테스트 모듈이므로 별도 `mod tests {}` 래퍼 없어요.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn sample_email() -> Email {
    Email::try_new("alice@example.com").expect("valid")
}

// 64-char hex (lowercase). SHA-256 of "test" — fixture only.
const SAMPLE_PHONE_HASH: &str = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";

fn sample_business_number() -> BusinessNumber {
    BusinessNumber::try_new("1234567891").expect("valid")
}

fn sample_broker_license() -> BrokerLicense {
    BrokerLicense::try_new("11-2024-12345").expect("valid")
}

fn sample_user(now: DateTime<Utc>) -> User {
    User::try_new(
        Id::new(),
        "zitadel-sub",
        sample_email(),
        "Alice",
        UserKind::Individual,
        now,
    )
    .expect("valid")
}

// ── try_new (minimal, 6 args) ─────────────────────────────────────────────

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

// ── Domain mutation methods (T9) ──────────────────────────────────────────

#[test]
fn verify_business_sets_fields_and_bumps_version() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let later = now + chrono::Duration::seconds(60);
    let bn = sample_business_number();

    user.verify_business(bn.clone(), later);

    assert_eq!(user.business_number, Some(bn));
    assert_eq!(user.business_verified_at, Some(later));
    assert_eq!(user.version, 2);
    assert_eq!(user.updated_at, later);
}

#[test]
fn verify_business_idempotent_updates_to_latest() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let bn = sample_business_number();
    let t1 = now + chrono::Duration::seconds(60);
    let t2 = now + chrono::Duration::seconds(120);

    user.verify_business(bn.clone(), t1);
    user.verify_business(bn.clone(), t2);

    assert_eq!(user.business_verified_at, Some(t2));
    assert_eq!(user.version, 3);
    assert_eq!(user.updated_at, t2);
}

#[test]
fn revoke_business_verification_clears_timestamp_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let bn = sample_business_number();
    user.verify_business(bn, now + chrono::Duration::seconds(30));

    let revoke_at = now + chrono::Duration::seconds(60);
    user.revoke_business_verification(revoke_at);

    assert_eq!(user.business_verified_at, None);
    // business_number는 보존 (재검증 가능).
    assert!(user.business_number.is_some());
    assert_eq!(user.version, 3);
    assert_eq!(user.updated_at, revoke_at);
}

#[test]
fn verify_broker_sets_license_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let lic = sample_broker_license();
    let later = now + chrono::Duration::seconds(60);

    user.verify_broker(lic.clone(), later);

    assert_eq!(user.broker_license_number, Some(lic));
    assert_eq!(user.broker_verified_at, Some(later));
    assert_eq!(user.version, 2);
    assert_eq!(user.updated_at, later);
}

#[test]
fn revoke_broker_verification_clears_timestamp_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    user.verify_broker(sample_broker_license(), now + chrono::Duration::seconds(30));

    let revoke_at = now + chrono::Duration::seconds(60);
    user.revoke_broker_verification(revoke_at);

    assert_eq!(user.broker_verified_at, None);
    assert!(user.broker_license_number.is_some());
    assert_eq!(user.version, 3);
}

#[test]
fn record_nice_verification_sets_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let later = now + chrono::Duration::seconds(60);

    user.record_nice_verification(later);

    assert_eq!(user.nice_verified_at, Some(later));
    assert_eq!(user.version, 2);
    assert_eq!(user.updated_at, later);
}

#[test]
fn record_marketing_consent_sets_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let later = now + chrono::Duration::seconds(60);

    user.record_marketing_consent(later);

    assert_eq!(user.marketing_consent_at, Some(later));
    assert_eq!(user.version, 2);
}

#[test]
fn revoke_marketing_consent_clears_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    user.record_marketing_consent(now + chrono::Duration::seconds(30));

    let revoke_at = now + chrono::Duration::seconds(60);
    user.revoke_marketing_consent(revoke_at);

    assert_eq!(user.marketing_consent_at, None);
    assert_eq!(user.version, 3);
}

#[test]
fn record_login_does_not_bump_version() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let initial_version = user.version;
    let later = now + chrono::Duration::seconds(60);

    user.record_login(later);

    assert_eq!(user.last_login_at, Some(later));
    assert_eq!(user.updated_at, later);
    // version은 의도적으로 bump 안 함 (로그인은 동시성 충돌 검사 대상 아님).
    assert_eq!(user.version, initial_version);
}

#[test]
fn add_role_appends_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let later = now + chrono::Duration::seconds(60);

    user.add_role(UserRole::Buyer, later);

    assert_eq!(user.roles, vec![UserRole::Buyer]);
    assert_eq!(user.version, 2);
}

#[test]
fn add_role_duplicate_is_noop() {
    let now = Utc::now();
    let mut user = sample_user(now);
    user.add_role(UserRole::Buyer, now + chrono::Duration::seconds(30));
    let version_after_first = user.version;
    let updated_after_first = user.updated_at;

    user.add_role(UserRole::Buyer, now + chrono::Duration::seconds(60));

    assert_eq!(user.roles, vec![UserRole::Buyer]);
    assert_eq!(user.version, version_after_first);
    assert_eq!(user.updated_at, updated_after_first);
}

#[test]
fn remove_role_removes_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    user.add_role(UserRole::Buyer, now + chrono::Duration::seconds(30));
    user.add_role(UserRole::Seller, now + chrono::Duration::seconds(45));

    let later = now + chrono::Duration::seconds(60);
    user.remove_role(UserRole::Buyer, later);

    assert_eq!(user.roles, vec![UserRole::Seller]);
    assert_eq!(user.version, 4);
    assert_eq!(user.updated_at, later);
}

#[test]
fn remove_role_missing_is_noop() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let initial_version = user.version;
    let initial_updated = user.updated_at;

    user.remove_role(UserRole::Admin, now + chrono::Duration::seconds(60));

    assert!(user.roles.is_empty());
    assert_eq!(user.version, initial_version);
    assert_eq!(user.updated_at, initial_updated);
}

#[test]
fn has_role_reports_membership() {
    let now = Utc::now();
    let mut user = sample_user(now);
    assert!(!user.has_role(UserRole::Buyer));

    user.add_role(UserRole::Buyer, now + chrono::Duration::seconds(30));

    assert!(user.has_role(UserRole::Buyer));
    assert!(!user.has_role(UserRole::Admin));
}

#[test]
fn soft_delete_sets_deleted_at_and_bumps() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let later = now + chrono::Duration::seconds(60);

    user.soft_delete(later);

    assert_eq!(user.deleted_at, Some(later));
    assert_eq!(user.version, 2);
    assert_eq!(user.updated_at, later);
    assert!(!user.is_active());
}

#[test]
fn soft_delete_idempotent() {
    let now = Utc::now();
    let mut user = sample_user(now);
    let t1 = now + chrono::Duration::seconds(60);
    let t2 = now + chrono::Duration::seconds(120);

    user.soft_delete(t1);
    let version_after_first = user.version;
    let deleted_at_after_first = user.deleted_at;

    user.soft_delete(t2);

    assert_eq!(user.deleted_at, deleted_at_after_first);
    assert_eq!(user.version, version_after_first);
}

#[test]
fn is_active_reflects_deleted_at() {
    let now = Utc::now();
    let mut user = sample_user(now);
    assert!(user.is_active());

    user.soft_delete(now + chrono::Duration::seconds(60));

    assert!(!user.is_active());
}

#[test]
fn is_business_verified_reflects_timestamp() {
    let now = Utc::now();
    let mut user = sample_user(now);
    assert!(!user.is_business_verified());

    user.verify_business(sample_business_number(), now + chrono::Duration::seconds(60));

    assert!(user.is_business_verified());
}

#[test]
fn is_broker_reflects_timestamp() {
    let now = Utc::now();
    let mut user = sample_user(now);
    assert!(!user.is_broker());

    user.verify_broker(sample_broker_license(), now + chrono::Duration::seconds(60));

    assert!(user.is_broker());
}
