//! `Bookmark` 도메인 (Insights BC, RDS 동적).
//!
//! 두 종류 북마크:
//! - `BookmarkListing` — 매물 (`Listing` FK + composite PK)
//! - `BookmarkExternal` — 외부 `R2` entity polymorphic (`Parcel`/`CourtAuction`/`Manufacturer`/`IndustrialComplex`)

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod errors;
pub mod external;
pub mod external_kind;
pub mod listing;
pub mod repository;
