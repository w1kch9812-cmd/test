//! `ListingPhoto` Aggregate (Core BC, RDS 동적 + R2 binary).
//!
//! 매물 사진 — 메타데이터는 RDS, 실제 바이너리는 R2 (`r2_key` 참조).
//! `ListingPhoto` is owned by `Listing` (FK + ON DELETE CASCADE).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod entity;
pub mod errors;
pub mod events;
pub mod repository;

#[cfg(test)]
mod events_tests;
