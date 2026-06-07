use std::sync::Arc;

use chrono::{DateTime, Utc};
use listing_photo_domain::entity::ListingPhoto;
use listing_photo_domain::events::{ListingPhotoUploadConfirmed, ListingPhotoUploadConfirmedFacts};
use shared_kernel::domain_event::DomainEvent;

pub(super) fn upload_confirmed_event_for_photo(
    photo: &ListingPhoto,
    file_size_bytes: i64,
    occurred_at: DateTime<Utc>,
) -> Arc<dyn DomainEvent> {
    Arc::new(ListingPhotoUploadConfirmed::new(
        ListingPhotoUploadConfirmedFacts {
            photo_id: photo.id.clone(),
            listing_id: photo.listing_id.clone(),
            r2_key: photo.r2_key.clone(),
            content_type: photo.content_type,
            file_size_bytes,
            occurred_at,
        },
    ))
}

#[cfg(test)]
#[path = "photo_events_tests.rs"]
mod tests;
