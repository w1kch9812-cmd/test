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

use super::super::{Listing, ListingUpdate};
use super::fixtures::{later, pending_review, sample_draft, t0};
use crate::errors::ListingError;

// ── update_editable_fields (SP6-iv) ────────────────────────────────────────

#[test]
fn update_editable_fields_draft_title_only_bumps_version_keeps_others() {
    let mut l = sample_draft(t0());
    let original_price = l.price;
    let original_area = l.area;

    let update = ListingUpdate {
        title: Some(ListingTitle::try_new("새 제목").unwrap()),
        ..Default::default()
    };
    l.update_editable_fields(update, later(1)).unwrap();

    assert_eq!(l.title.as_str(), "새 제목");
    assert_eq!(l.price, original_price);
    assert_eq!(l.area, original_area);
    assert_eq!(l.version, 2);
    assert_eq!(l.updated_at, later(1));
    assert_eq!(l.status, ListingStatus::Draft); // 상태 불변
}

#[test]
fn update_editable_fields_rejected_state_allowed() {
    let mut l = sample_draft(t0());
    l.submit_for_review(later(1)).unwrap();
    l.reject(later(2)).unwrap();
    assert_eq!(l.status, ListingStatus::Rejected);

    let update = ListingUpdate {
        description: Some(Description::try_new("수정된 설명").unwrap()),
        ..Default::default()
    };
    l.update_editable_fields(update, later(3)).unwrap();

    assert_eq!(l.description.as_str(), "수정된 설명");
    assert_eq!(l.status, ListingStatus::Rejected); // 상태 불변
}

#[test]
fn update_editable_fields_active_state_returns_immutable_state() {
    let mut l = sample_draft(t0());
    l.submit_for_review(later(1)).unwrap();
    l.approve(later(2)).unwrap();
    assert_eq!(l.status, ListingStatus::Active);

    let update = ListingUpdate {
        title: Some(ListingTitle::try_new("바꾸려고").unwrap()),
        ..Default::default()
    };
    let err = l.update_editable_fields(update, later(3)).unwrap_err();
    assert_eq!(
        err,
        ListingError::ImmutableState {
            current: ListingStatus::Active,
        }
    );
}

#[test]
fn update_editable_fields_pending_review_returns_immutable_state() {
    let mut l = pending_review(t0());
    let update = ListingUpdate {
        price: Some(MoneyKrw::try_new(700_000_000).unwrap()),
        ..Default::default()
    };
    let err = l.update_editable_fields(update, later(1)).unwrap_err();
    assert!(matches!(err, ListingError::ImmutableState { .. }));
}

#[test]
fn update_editable_fields_clearing_deposit_on_jeonse_returns_mismatch() {
    // Jeonse 매물 = deposit 필수. 그 deposit 를 clear 시도 → 거부.
    let mut l = Listing::try_new_draft(
        Id::<ListingMarker>::new(),
        Id::<UserMarker>::new(),
        Pnu::try_new("1111010100100010000").unwrap(),
        ListingType::Factory,
        TransactionType::Jeonse,
        MoneyKrw::try_new(500_000_000).unwrap(),      // price
        Some(MoneyKrw::try_new(50_000_000).unwrap()), // deposit
        None,
        AreaM2::try_new(250.0).unwrap(),
        ListingTitle::try_new("Jeonse 매물").unwrap(),
        Description::try_new("desc").unwrap(),
        t0(),
    )
    .expect("valid jeonse draft");

    let update = ListingUpdate {
        deposit: Some(None), // clear 시도
        ..Default::default()
    };
    let err = l.update_editable_fields(update, later(1)).unwrap_err();
    assert!(matches!(
        err,
        ListingError::TransactionFieldsMismatch { .. }
    ));
}

#[test]
fn update_editable_fields_partial_update_does_not_clear_unspecified() {
    let mut l = sample_draft(t0()); // Sale, deposit=None, monthly_rent=None
    let original_title = l.title.clone();

    let update = ListingUpdate {
        // title 명시 안 함 → 그대로 유지
        price: Some(MoneyKrw::try_new(999_999_999).unwrap()),
        ..Default::default()
    };
    l.update_editable_fields(update, later(1)).unwrap();

    assert_eq!(l.title, original_title);
    assert_eq!(l.price.as_i64(), 999_999_999);
}

#[test]
fn update_editable_fields_contact_visibility_changes() {
    let mut l = sample_draft(t0());
    let update = ListingUpdate {
        contact_visibility: Some(ContactVisibility::VerifiedOnly),
        ..Default::default()
    };
    l.update_editable_fields(update, later(1)).unwrap();
    assert_eq!(l.contact_visibility, ContactVisibility::VerifiedOnly);
}

#[test]
fn update_editable_fields_no_op_update_still_bumps_version() {
    // 빈 update — 모두 None — 도 version bump + updated_at 갱신.
    // 이 동작은 *의도된 것*: client 가 PATCH 호출했다는 사실 자체가 audit 가치.
    let mut l = sample_draft(t0());
    let original_version = l.version;
    l.update_editable_fields(ListingUpdate::default(), later(1))
        .unwrap();
    assert_eq!(l.version, original_version + 1);
    assert_eq!(l.updated_at, later(1));
}
