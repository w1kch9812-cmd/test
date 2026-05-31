use shared_kernel::admin_division::EupmyeondongCode;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::transaction_type::TransactionType;
use shared_kernel::zoning::Zoning;

use crate::entity::Listing;

/// PNU lookup denormalized fields accepted by `update_parcel_denormalize`.
///
/// The PNU itself remains listing identity. This struct only carries lookup facts that can be
/// refreshed from the parcel/PNU source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingParcelDenormalize {
    /// Eight-digit administrative division code.
    pub admin_code: EupmyeondongCode,
    /// Land use type.
    pub land_use_type: LandUseType,
    /// Zoning from V-World parcel lookup when available.
    pub zoning: Option<Zoning>,
}

/// Listing detail page response projection.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingDetail {
    /// Full listing aggregate.
    pub listing: Listing,
    /// Active photos ordered by display order.
    pub photos: Vec<ListingPhotoSummary>,
    /// Bookmark count from the bookmark relation.
    pub bookmark_count: i64,
    /// Whether the viewer bookmarked this listing.
    pub is_bookmarked: bool,
}

/// Photo projection used by listing detail responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingPhotoSummary {
    /// Photo id.
    pub photo_id: String,
    /// R2 object key.
    pub r2_key: String,
    /// Optional thumbnail R2 object key.
    pub thumbnail_r2_key: Option<String>,
    /// Optional caption.
    pub caption: Option<String>,
    /// Display order.
    pub display_order: i32,
    /// MIME content type.
    pub content_type: String,
}

/// Listing card projection used by search and map-side list views.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingCardSummary {
    /// Listing id.
    pub id: Id<ListingIdMarker>,
    /// Listing title.
    pub title: String,
    /// Listing type.
    pub listing_type: ListingType,
    /// Transaction type.
    pub transaction_type: TransactionType,
    /// Main price.
    pub price: MoneyKrw,
    /// Deposit for rent or jeonse listings.
    pub deposit: Option<MoneyKrw>,
    /// Monthly rent amount.
    pub monthly_rent: Option<MoneyKrw>,
    /// Area in square meters.
    pub area_m2: f64,
    /// Thumbnail URL when available.
    pub thumbnail_url: Option<String>,
    /// View count.
    pub view_count: i64,
    /// Bookmark count from the bookmark relation.
    pub bookmark_count: i64,
    /// Whether the viewer bookmarked this listing.
    pub is_bookmarked: bool,
    /// Creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,
}
