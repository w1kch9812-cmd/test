//! Narrow parcel information needed for Gongzzang listing denormalization.
//!
//! This is not the canonical Catalog parcel aggregate. Canonical parcel facts
//! live in Platform Core; this struct is the Gongzzang-owned projection shape
//! consumed by listing creation and the parcel summary API.

use serde::{Deserialize, Serialize};
use shared_kernel::admin_division::AdminDivision;
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::zoning::Zoning;

/// Official land price notice year/month lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GosiYearMonth {
    /// Four-digit notice year.
    pub year: u16,
    /// Notice month, from 1 to 12.
    pub month: u8,
}

/// Parcel information subset used by Gongzzang.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParcelInfo {
    /// Administrative hierarchy derived from the PNU.
    pub admin: AdminDivision,
    /// Gongzzang-facing land-use classification.
    pub land_use_type: LandUseType,
    /// Zoning, when Platform Core publishes a zoning source.
    pub zoning: Option<Zoning>,
    /// Official land price in KRW per square meter, when available.
    pub official_land_price_per_m2: Option<MoneyKrw>,
    /// Official land price notice year/month lineage.
    pub gosi_year_month: Option<GosiYearMonth>,
}
