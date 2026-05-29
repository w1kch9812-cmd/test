//! PNU-based parcel information lookup port.
//!
//! Gongzzang owns the listing denormalization need, but Platform Core owns
//! canonical Catalog parcel data. This crate keeps only the Gongzzang-facing
//! port and projection shape; runtime HTTP adapters live in `services/api`.

#![forbid(unsafe_code)]

pub mod info;
pub mod lookup;
pub mod noop_lookup;

pub use info::{GosiYearMonth, ParcelInfo};
pub use lookup::{LookupError, ParcelInfoLookup};
pub use noop_lookup::NoOpParcelInfoLookup;
