//! Domain events emitted by the listing photo aggregate boundary.

use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use shared_kernel::domain_event::DomainEvent;
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};

use crate::entity::PhotoContentType;

/// Event type emitted after an uploaded listing photo object has been verified.
pub const LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE: &str = "listing_photo.upload_confirmed";

/// Listing photo upload-confirmed domain event.
///
/// This event intentionally carries only Gongzzang-owned listing photo facts. Platform Core
/// lakehouse registry names, namespaces, or endpoints are derived by the worker-side integration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingPhotoUploadConfirmed {
    photo_id: Id<ListingPhotoMarker>,
    listing_id: Id<ListingMarker>,
    r2_key: String,
    content_type: PhotoContentType,
    file_size_bytes: i64,
    occurred_at: DateTime<Utc>,
}

impl ListingPhotoUploadConfirmed {
    /// Build the event from a verified storage object and its owning listing photo.
    #[must_use]
    pub const fn new(
        photo_id: Id<ListingPhotoMarker>,
        listing_id: Id<ListingMarker>,
        r2_key: String,
        content_type: PhotoContentType,
        file_size_bytes: i64,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            photo_id,
            listing_id,
            r2_key,
            content_type,
            file_size_bytes,
            occurred_at,
        }
    }
}

impl DomainEvent for ListingPhotoUploadConfirmed {
    fn event_type(&self) -> &'static str {
        LISTING_PHOTO_UPLOAD_CONFIRMED_EVENT_TYPE
    }

    fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    fn aggregate_id(&self) -> String {
        self.photo_id.as_str().to_owned()
    }

    fn payload(&self) -> Value {
        json!({
            "photo_id": self.photo_id.as_str(),
            "listing_id": self.listing_id.as_str(),
            "r2_key": self.r2_key,
            "content_type": self.content_type.as_str(),
            "file_size_bytes": self.file_size_bytes,
        })
    }
}
