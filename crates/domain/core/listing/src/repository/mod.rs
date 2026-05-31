//! `ListingRepository` port and listing read contracts.
//!
//! This module keeps the public `listing_domain::repository::*` API stable while splitting
//! read models, search queries, and marker tile contracts into smaller implementation files.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::mutation::MutationContext;
use thiserror::Error;

use crate::entity::Listing;

mod marker_tile;
mod read_models;
mod search;

pub use crate::marker_filter::{
    ListingMarkerFilter, ListingMarkerFilterError, ListingMarkerFilterSpec,
    NormalizedListingMarkerFilterSpec, ALL_ACTIVE_LISTING_MARKER_FILTER_HASH,
};
pub use marker_tile::{
    ListingMarkerCount, ListingMarkerDeltas, ListingMarkerMask, ListingMarkerMaskEncoding,
    ListingMarkerMaskQuery, ListingMarkerOverlayTileQuery, ListingMarkerRegisteredFilter,
    ListingMarkerTile, ListingMarkerTileQuery, ListingMarkerTileQueryError,
    ListingMarkerTombstones, LISTING_MARKER_DELTA_TILE_LAYER, LISTING_MARKER_TILE_CONTENT_TYPE,
    LISTING_MARKER_TILE_EXACT_MIN_ZOOM, LISTING_MARKER_TILE_LAYER, LISTING_MARKER_TILE_MAX_ZOOM,
    LISTING_MARKER_TILE_MIN_ZOOM,
};
pub use read_models::{
    ListingCardSummary, ListingDetail, ListingParcelDenormalize, ListingPhotoSummary,
};
pub use search::{CardSearchQuery, CardSearchSort};

/// Listing persistence and read-model port.
#[async_trait]
pub trait ListingRepository: Send + Sync {
    /// Find a listing aggregate by id.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the backing repository fails.
    async fn find(&self, id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError>;

    /// Return active listing card summaries and the total count for a search query.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the backing repository fails.
    async fn find_card_summaries(
        &self,
        query: CardSearchQuery,
    ) -> Result<(Vec<ListingCardSummary>, u64), RepoError>;

    /// Return a Gongzzang-owned listing marker `MVT/PBF` tile.
    ///
    /// Marker positions must come from the platform-core `PNU` anchor projection, not from
    /// listing-owned coordinates. A successful tile must preserve
    /// `represented_count == eligible_count`.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] for backing store failures, incomplete anchor coverage, or
    /// completeness invariant failures.
    async fn find_listing_marker_tile(
        &self,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError>;

    /// Return marker ids matching a filter for a loaded listing marker tile.
    ///
    /// The mask is a serving optimization and must not expose canonical coordinates.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_mask(
        &self,
        query: ListingMarkerMaskQuery,
    ) -> Result<ListingMarkerMask, RepoError>;

    /// Return marker ids that must be hidden for a loaded listing marker tile.
    ///
    /// Tombstones prevent deleted, sold, rejected, expired, or private markers from remaining
    /// visible while cached base tiles age out.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError>;

    /// Return recent public marker changes for a loaded listing marker tile.
    ///
    /// Delta overlays improve write freshness before the base tile cache or artifact is refreshed.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn find_listing_marker_deltas(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerDeltas, RepoError>;

    /// Upsert the listing marker serving projection from listing semantics and PNU anchor data.
    ///
    /// The projection is not a coordinate source of truth. Marker position must be copied from the
    /// platform-core-owned `parcel_marker_anchor` read model.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::NotFound`] when the listing is absent, and [`RepoError::Database`] when
    /// the listing has no PNU anchor.
    async fn upsert_listing_marker_projection(
        &self,
        id: &Id<ListingIdMarker>,
    ) -> Result<(), RepoError>;

    /// Count public listing markers from the marker serving projection/index.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the projection index query fails.
    async fn count_listing_markers(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerCount, RepoError>;

    /// Register a normalized marker filter and return its stable hash.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the registry write fails.
    async fn register_listing_marker_filter(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerRegisteredFilter, RepoError>;

    /// Resolve a registered marker filter hash to its normalized payload.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the registry read fails.
    async fn resolve_listing_marker_filter(
        &self,
        filter_hash: &str,
    ) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError>;

    /// Find listings owned by a user, optionally constrained by listing status.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the backing repository fails.
    async fn find_by_owner(
        &self,
        owner_id: &Id<UserMarker>,
        status: Option<ListingStatus>,
    ) -> Result<Vec<Listing>, RepoError>;

    /// Insert or update a listing aggregate.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Conflict`] for optimistic-lock failures and [`RepoError::Database`] for
    /// backing repository failures.
    async fn save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError>;

    /// Find listing detail data for a viewer.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::Database`] when the backing repository fails.
    async fn find_detail_by_id(
        &self,
        id: &Id<ListingIdMarker>,
        viewer_user_id: &Id<UserMarker>,
    ) -> Result<Option<ListingDetail>, RepoError>;

    /// Increment the listing view counter.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::NotFound`] when the listing is absent and [`RepoError::Database`] when
    /// the backing repository fails.
    async fn increment_view_count(&self, id: &Id<ListingIdMarker>) -> Result<(), RepoError>;

    /// Update PNU lookup denormalized fields on the listing row.
    ///
    /// # Errors
    ///
    /// Returns [`RepoError::NotFound`] when the listing is absent and [`RepoError::Database`] when
    /// the backing repository fails.
    async fn update_parcel_denormalize(
        &self,
        id: &Id<ListingIdMarker>,
        denormalize: &ListingParcelDenormalize,
    ) -> Result<(), RepoError>;
}

/// Repository error.
#[derive(Debug, Error)]
pub enum RepoError {
    /// Aggregate was not found.
    #[error("not found")]
    NotFound,
    /// Optimistic lock version mismatch.
    #[error("conflict (version mismatch)")]
    Conflict,
    /// Backing database or repository failure.
    #[error("database error: {0}")]
    Database(String),
}
