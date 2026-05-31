use shared_kernel::id::{Id, UserMarker};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::listing_type::ListingType;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;

/// Card list search query.
#[derive(Debug, Clone)]
pub struct CardSearchQuery {
    /// Exact PNU filter.
    pub pnu: Option<Pnu>,
    /// Administrative code prefix filter.
    pub admin_code_prefix: Option<String>,
    /// Land use type filter.
    pub land_use_type: Option<LandUseType>,
    /// Listing type filter.
    pub types: Option<Vec<ListingType>>,
    /// Transaction type filter.
    pub transactions: Option<Vec<TransactionType>>,
    /// Minimum area in square meters.
    pub min_area_m2: Option<f64>,
    /// Maximum area in square meters.
    pub max_area_m2: Option<f64>,
    /// Minimum price in KRW.
    pub min_price_krw: Option<i64>,
    /// Maximum price in KRW.
    pub max_price_krw: Option<i64>,
    /// Zero-based page number.
    pub page: u32,
    /// Page size.
    pub size: u32,
    /// Sort order.
    pub sort: CardSearchSort,
    /// Viewer id used for bookmark joins.
    pub viewer_user_id: Id<UserMarker>,
}

/// Card list sort order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CardSearchSort {
    /// Newest listings first.
    #[default]
    CreatedAtDesc,
    /// Lowest price first.
    PriceAsc,
    /// Highest price first.
    PriceDesc,
    /// Smallest area first.
    AreaAsc,
    /// Largest area first.
    AreaDesc,
}
