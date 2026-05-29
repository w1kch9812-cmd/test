use chrono::{DateTime, Duration, TimeZone, Utc};
use shared_kernel::area::AreaM2;
use shared_kernel::description::Description;
use shared_kernel::id::{Id, ListingMarker, UserMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;

use super::super::Listing;

pub(super) fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 5, 2, 12, 0, 0).unwrap()
}

pub(super) fn later(secs: i64) -> DateTime<Utc> {
    t0() + Duration::seconds(secs)
}

/// `Draft` 상태 샘플 매물 — `Sale` 거래.
pub(super) fn sample_draft(now: DateTime<Utc>) -> Listing {
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
        now,
    )
    .expect("valid draft")
}

/// `PendingReview` 상태 매물 (Draft → `submit_for_review`).
pub(super) fn pending_review(now: DateTime<Utc>) -> Listing {
    let mut l = sample_draft(now);
    l.submit_for_review(now).expect("draft -> pending");
    l
}

/// `Active` 상태 매물 (Draft → `PendingReview` → Active).
pub(super) fn active(now: DateTime<Utc>) -> Listing {
    let mut l = pending_review(now);
    l.approve(now).expect("pending -> active");
    l
}

/// `Sold` 상태 매물 (Active → Sold).
pub(super) fn sold(now: DateTime<Utc>) -> Listing {
    let mut l = active(now);
    l.mark_sold(now).expect("active -> sold");
    l
}

/// `Expired` 상태 매물 (Active → Expired).
pub(super) fn expired(now: DateTime<Utc>) -> Listing {
    let mut l = active(now);
    l.expire(now).expect("active -> expired");
    l
}

/// `Rejected` 상태 매물 (Draft → `PendingReview` → Rejected).
pub(super) fn rejected(now: DateTime<Utc>) -> Listing {
    let mut l = pending_review(now);
    l.reject(now).expect("pending -> rejected");
    l
}
