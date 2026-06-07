#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{TimeZone, Utc};
use serde_json::json;
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};

use crate::entity::PhotoContentType;
use crate::events::{
    ListingPhotoUploadConfirmed, ListingPhotoUploadConfirmedFacts,
    LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE,
};

#[test]
fn listing_photo_upload_confirmed_event_shape_preserves_domain_and_storage_metadata() {
    let photo_id = Id::<ListingPhotoMarker>::new();
    let listing_id = Id::<ListingMarker>::new();
    let occurred_at = Utc
        .with_ymd_and_hms(2026, 6, 7, 12, 30, 0)
        .single()
        .unwrap();

    let event = ListingPhotoUploadConfirmed::new(ListingPhotoUploadConfirmedFacts {
        photo_id: photo_id.clone(),
        listing_id: listing_id.clone(),
        r2_key: "media/listing-photo/listings/lst_test/photos/lph_test.webp".to_owned(),
        content_type: PhotoContentType::Webp,
        file_size_bytes: 123_456,
        occurred_at,
    });

    assert_eq!(
        event.event_type(),
        LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE
    );
    assert_eq!(event.aggregate_id(), photo_id.as_str());
    assert_eq!(event.occurred_at(), occurred_at);
    assert_eq!(
        event.payload(),
        json!({
            "photo_id": photo_id.as_str(),
            "listing_id": listing_id.as_str(),
            "r2_key": "media/listing-photo/listings/lst_test/photos/lph_test.webp",
            "content_type": "image/webp",
            "file_size_bytes": 123_456,
        })
    );
}

#[test]
fn listing_photo_upload_confirmed_event_does_not_embed_platform_core_registry_details() {
    let event = ListingPhotoUploadConfirmed::new(ListingPhotoUploadConfirmedFacts {
        photo_id: Id::<ListingPhotoMarker>::new(),
        listing_id: Id::<ListingMarker>::new(),
        r2_key: "media/listing-photo/listings/lst_test/photos/lph_test.jpg".to_owned(),
        content_type: PhotoContentType::Jpeg,
        file_size_bytes: 10,
        occurred_at: Utc::now(),
    });

    let payload = event.payload();
    assert!(payload.get("qualified_name").is_none());
    assert!(payload.get("registry_endpoint").is_none());
    assert!(payload.get("namespace").is_none());
}
