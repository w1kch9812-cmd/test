//! `Listing` Aggregate 단위 테스트 — `entity.rs`/`errors.rs` 동작 검증.
//!
//! `entity.rs`에서 `#[path = "entity_tests.rs"] mod tests;` 형태로 포함해요.
//! 파일 자체가 테스트 모듈이므로 별도 `mod tests {}` 래퍼 없어요.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{DateTime, TimeZone, Utc};
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
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

// ── Fixtures ───────────────────────────────────────────────────────────────

fn sample_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()
}

fn sample_pnu() -> Pnu {
    Pnu::try_new("1111010100100010000").expect("valid")
}

fn sample_title() -> ListingTitle {
    ListingTitle::try_new("좋은 공장 매물").expect("valid")
}

fn sample_description() -> Description {
    Description::try_new("위치 좋고 면적 넓어요.").expect("valid")
}

fn sample_area() -> AreaM2 {
    AreaM2::try_new(330.0).expect("valid")
}

fn sample_price() -> MoneyKrw {
    MoneyKrw::try_new(500_000_000).expect("valid")
}

fn sample_deposit() -> MoneyKrw {
    MoneyKrw::try_new(20_000_000).expect("valid")
}

fn sample_monthly_rent() -> MoneyKrw {
    MoneyKrw::try_new(2_000_000).expect("valid")
}

/// `Sale` 거래 유형 happy path 빌드 — `deposit`/`monthly_rent` 모두 `None`.
fn build_sale() -> Listing {
    Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Factory,
        TransactionType::Sale,
        sample_price(),
        None,
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .expect("Sale + None + None is valid")
}

/// `MonthlyRent` 거래 유형 happy path 빌드 — `deposit`/`monthly_rent` 모두 `Some`.
fn build_monthly_rent() -> Listing {
    Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Warehouse,
        TransactionType::MonthlyRent,
        sample_price(),
        Some(sample_deposit()),
        Some(sample_monthly_rent()),
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .expect("MonthlyRent + Some + Some is valid")
}

/// `Jeonse` 거래 유형 happy path 빌드 — `deposit` `Some`, `monthly_rent` `None`.
fn build_jeonse() -> Listing {
    Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Office,
        TransactionType::Jeonse,
        sample_price(),
        Some(sample_deposit()),
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .expect("Jeonse + Some + None is valid")
}

// ── Happy paths (3) ─────────────────────────────────────────────────────────

#[test]
fn sale_with_no_deposit_no_monthly_rent_is_valid() {
    let l = build_sale();
    assert_eq!(l.transaction_type, TransactionType::Sale);
    assert!(l.deposit.is_none());
    assert!(l.monthly_rent.is_none());
}

#[test]
fn monthly_rent_with_deposit_and_monthly_rent_is_valid() {
    let l = build_monthly_rent();
    assert_eq!(l.transaction_type, TransactionType::MonthlyRent);
    assert!(l.deposit.is_some());
    assert!(l.monthly_rent.is_some());
}

#[test]
fn jeonse_with_deposit_only_is_valid() {
    let l = build_jeonse();
    assert_eq!(l.transaction_type, TransactionType::Jeonse);
    assert!(l.deposit.is_some());
    assert!(l.monthly_rent.is_none());
}

// ── V003_01 invariant violations (6) ────────────────────────────────────────

#[test]
fn sale_with_deposit_some_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Factory,
        TransactionType::Sale,
        sample_price(),
        Some(sample_deposit()),
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::Sale,
            deposit_required: false,
            monthly_rent_required: false,
        }
    ));
}

#[test]
fn sale_with_monthly_rent_some_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Factory,
        TransactionType::Sale,
        sample_price(),
        None,
        Some(sample_monthly_rent()),
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::Sale,
            ..
        }
    ));
}

#[test]
fn monthly_rent_without_deposit_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Warehouse,
        TransactionType::MonthlyRent,
        sample_price(),
        None,
        Some(sample_monthly_rent()),
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::MonthlyRent,
            deposit_required: true,
            monthly_rent_required: true,
        }
    ));
}

#[test]
fn monthly_rent_without_monthly_rent_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Warehouse,
        TransactionType::MonthlyRent,
        sample_price(),
        Some(sample_deposit()),
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::MonthlyRent,
            ..
        }
    ));
}

#[test]
fn jeonse_without_deposit_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Office,
        TransactionType::Jeonse,
        sample_price(),
        None,
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::Jeonse,
            deposit_required: true,
            monthly_rent_required: false,
        }
    ));
}

#[test]
fn jeonse_with_monthly_rent_some_is_rejected() {
    let err = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Office,
        TransactionType::Jeonse,
        sample_price(),
        Some(sample_deposit()),
        Some(sample_monthly_rent()),
        sample_area(),
        sample_title(),
        sample_description(),
        sample_now(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch {
            transaction_type: TransactionType::Jeonse,
            ..
        }
    ));
}

// ── Default initial values ───────────────────────────────────────────────────

#[test]
fn draft_starts_in_draft_status() {
    let l = build_sale();
    assert_eq!(l.status, ListingStatus::Draft);
}

#[test]
fn draft_default_contact_visibility_is_login_required() {
    let l = build_sale();
    assert_eq!(l.contact_visibility, ContactVisibility::LoginRequired);
}

#[test]
fn draft_starts_with_zero_view_count() {
    let l = build_sale();
    assert_eq!(l.view_count, 0);
}

#[test]
fn draft_starts_with_zero_bookmark_count() {
    let l = build_sale();
    assert_eq!(l.bookmark_count, 0);
}

#[test]
fn draft_starts_with_version_one() {
    let l = build_sale();
    assert_eq!(l.version, 1);
}

#[test]
fn draft_created_at_equals_updated_at_at_now() {
    let now = sample_now();
    let l = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        sample_pnu(),
        ListingType::Factory,
        TransactionType::Sale,
        sample_price(),
        None,
        None,
        sample_area(),
        sample_title(),
        sample_description(),
        now,
    )
    .expect("valid");
    assert_eq!(l.created_at, now);
    assert_eq!(l.updated_at, now);
}

#[test]
fn draft_expires_at_is_none() {
    let l = build_sale();
    assert!(l.expires_at.is_none());
}

// ── Serde roundtrip ──────────────────────────────────────────────────────────

#[test]
fn serde_json_roundtrip_preserves_listing() {
    let original = build_monthly_rent();
    let json = serde_json::to_string(&original).expect("serialize");
    let decoded: Listing = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(decoded, original);
}
