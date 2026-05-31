//! Postgres implementation for the listing repository.
//!
//! The public repository type stays in this module. SQL-heavy operations live in
//! focused private modules so row conversion, read projections, marker tiles,
//! and transactional persistence can evolve independently.

#![allow(clippy::module_name_repetitions)]

mod card_summaries;
mod detail;
mod marker_count;
mod marker_delta;
mod marker_filter_registry;
mod marker_mask;
mod marker_projection;
mod marker_tile;
mod marker_tombstone;
mod persistence;
mod repository;
mod rows;

use sqlx::PgPool;

/// `Listing` aggregate repository backed by Postgres.
#[derive(Debug, Clone)]
pub struct PgListingRepository {
    pool: PgPool,
}

impl PgListingRepository {
    /// Create a repository using the shared Postgres pool.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
