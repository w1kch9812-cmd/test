use std::sync::Arc;

use listing_domain::repository::ListingRepository;
use listing_photo_domain::repository::ListingPhotoRepository;
use parcel_lookup::ParcelInfoLookup;

use crate::photo_upload::{ListingPhotoObjectVerifier, ListingPhotoUploadUrlIssuer};

/// 핸들러 공유 상태.
#[derive(Clone)]
pub struct ListingsState {
    /// `Listing` 저장소.
    pub listing_repo: Arc<dyn ListingRepository>,
    /// `ListingPhoto` 저장소.
    pub photo_repo: Arc<dyn ListingPhotoRepository>,
    /// PNU → 행정/지목/용도지역 lookup (ADR 0018, SP9 T4).
    /// production = `VWorldParcelInfoLookup`, dev/test = `NoOpParcelInfoLookup`.
    pub parcel_lookup: Arc<dyn ParcelInfoLookup>,
    /// Listing photo binary upload URL issuer.
    pub photo_upload_issuer: Arc<dyn ListingPhotoUploadUrlIssuer>,
    /// Listing photo binary object verifier.
    pub photo_object_verifier: Arc<dyn ListingPhotoObjectVerifier>,
}
