#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{TimeZone, Utc};
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::events::LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE;
use serde_json::json;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};

use super::upload_confirmed_event_for_photo;

#[test]
fn upload_confirmed_event_for_photo_maps_confirmed_photo_to_outbox_domain_event() {
    let photo_id = Id::<ListingPhotoMarker>::new();
    let listing_id = Id::<ListingMarker>::new();
    let confirmed_at = Utc.with_ymd_and_hms(2026, 6, 7, 13, 0, 0).single().unwrap();
    let mut photo = ListingPhoto::try_new(
        photo_id.clone(),
        listing_id.clone(),
        "media/listing-photo/listings/lst_test/photos/lph_test.png",
        None,
        None,
        0,
        None,
        None,
        None,
        PhotoContentType::Png,
        confirmed_at,
    )
    .unwrap();
    photo
        .confirm_upload(None, None, 2048, confirmed_at)
        .unwrap();

    let event = upload_confirmed_event_for_photo(&photo, 2048, confirmed_at);

    assert_eq!(
        event.event_type(),
        LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE
    );
    assert_eq!(event.aggregate_id(), photo_id.as_str());
    assert_eq!(event.occurred_at(), confirmed_at);
    assert_eq!(
        event.payload(),
        json!({
            "photo_id": photo_id.as_str(),
            "listing_id": listing_id.as_str(),
            "r2_key": "media/listing-photo/listings/lst_test/photos/lph_test.png",
            "content_type": "image/png",
            "file_size_bytes": 2048,
        })
    );
}
