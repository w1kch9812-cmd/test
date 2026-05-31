use thiserror::Error;

use crate::marker_filter::ListingMarkerFilter;

/// Gongzzang listing marker vector-tile layer name.
pub const LISTING_MARKER_TILE_LAYER: &str = "listing";

/// Gongzzang listing marker delta vector-tile layer name.
pub const LISTING_MARKER_DELTA_TILE_LAYER: &str = "listing_delta";

/// Marker tile response content type.
pub const LISTING_MARKER_TILE_CONTENT_TYPE: &str = "application/vnd.mapbox-vector-tile";

/// Minimum zoom accepted by the Gongzzang listing marker tile API.
pub const LISTING_MARKER_TILE_MIN_ZOOM: u8 = 0;

/// Lowest zoom where exact listing marker features are preferred.
pub const LISTING_MARKER_TILE_EXACT_MIN_ZOOM: u8 = 14;

/// Maximum zoom accepted by the listing marker tile API.
pub const LISTING_MARKER_TILE_MAX_ZOOM: u8 = 22;

/// Validated tile query for the listing marker PBF surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTileQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Typed marker filter.
    pub filter: ListingMarkerFilter,
}

impl ListingMarkerTileQuery {
    /// Build a query without validation. Use only when inputs are already trusted.
    #[must_use]
    pub const fn new(z: u8, x: u32, y: u32, filter: ListingMarkerFilter) -> Self {
        Self { z, x, y, filter }
    }

    /// Validate public tile-coordinate input.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerTileQueryError`] when zoom or axis values are outside the vector-tile
    /// coordinate range.
    pub fn try_new(
        z: u8,
        x: u32,
        y: u32,
        filter: ListingMarkerFilter,
    ) -> Result<Self, ListingMarkerTileQueryError> {
        if !(LISTING_MARKER_TILE_MIN_ZOOM..=LISTING_MARKER_TILE_MAX_ZOOM).contains(&z) {
            return Err(ListingMarkerTileQueryError::InvalidZoom { z });
        }
        let axis_limit = 1_u32 << u32::from(z);
        if x >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidX { z, x });
        }
        if y >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidY { z, y });
        }
        Ok(Self::new(z, x, y, filter))
    }
}

/// Listing marker tile coordinate validation error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ListingMarkerTileQueryError {
    /// Zoom is outside the accepted MVT range.
    #[error("listing marker tile zoom out of range: {z}")]
    InvalidZoom {
        /// Invalid zoom.
        z: u8,
    },
    /// X coordinate is outside the zoom-dependent axis range.
    #[error("listing marker tile x out of range for z={z}: {x}")]
    InvalidX {
        /// Zoom.
        z: u8,
        /// Invalid x coordinate.
        x: u32,
    },
    /// Y coordinate is outside the zoom-dependent axis range.
    #[error("listing marker tile y out of range for z={z}: {y}")]
    InvalidY {
        /// Zoom.
        z: u8,
        /// Invalid y coordinate.
        y: u32,
    },
}

/// Gongzzang listing marker PBF tile plus server-side completeness metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTile {
    /// MVT/PBF response bytes.
    pub bytes: Vec<u8>,
    /// MVT source-layer name.
    pub layer_name: &'static str,
    /// Active listings selected for this tile and filter.
    pub eligible_count: i64,
    /// Listings represented by returned features or truthful aggregates.
    pub represented_count: i64,
    /// Raw feature count in the tile.
    pub feature_count: i64,
    /// Aggregate feature count in the tile.
    pub aggregate_count: i64,
    /// Anchor snapshot identity used by represented features.
    pub anchor_snapshot_id: Option<String>,
}

/// Listing marker mask request for a loaded tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerMaskQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Typed marker filter.
    pub filter: ListingMarkerFilter,
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

/// Listing marker mask encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListingMarkerMaskEncoding {
    /// `marker_ids` are the ids that should remain visible.
    Show,
    /// `marker_ids` are the ids that should be hidden.
    Hide,
}

impl ListingMarkerMaskEncoding {
    /// Stable JSON/API value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Show => "show",
            Self::Hide => "hide",
        }
    }
}

/// Listing marker mask response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerMask {
    /// Compact mask encoding.
    pub encoding: ListingMarkerMaskEncoding,
    /// Marker ids selected by the mask. Coordinates are intentionally absent.
    pub marker_ids: Vec<String>,
    /// Highest projection version included in this mask.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in this mask.
    pub anchor_snapshot_id: Option<String>,
}

/// Query for listing marker overlay records addressed by tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerOverlayTileQuery {
    /// Web mercator zoom.
    pub z: u8,
    /// Web mercator x coordinate.
    pub x: u32,
    /// Web mercator y coordinate.
    pub y: u32,
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

impl ListingMarkerOverlayTileQuery {
    /// Validate public overlay tile-coordinate input.
    ///
    /// # Errors
    ///
    /// Returns [`ListingMarkerTileQueryError`] when zoom or axis values are outside the vector-tile
    /// coordinate range.
    pub fn try_new(
        z: u8,
        x: u32,
        y: u32,
        base_version: Option<i64>,
    ) -> Result<Self, ListingMarkerTileQueryError> {
        if z > LISTING_MARKER_TILE_MAX_ZOOM {
            return Err(ListingMarkerTileQueryError::InvalidZoom { z });
        }
        let axis_limit = 1_u32 << u32::from(z);
        if x >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidX { z, x });
        }
        if y >= axis_limit {
            return Err(ListingMarkerTileQueryError::InvalidY { z, y });
        }
        Ok(Self {
            z,
            x,
            y,
            base_version,
        })
    }
}

/// Listing marker tombstone overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerTombstones {
    /// Marker ids that must be hidden by the client.
    pub marker_ids: Vec<String>,
    /// Highest projection version represented by this tombstone response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this tombstone response.
    pub anchor_snapshot_id: Option<String>,
}

/// Listing marker delta overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerDeltas {
    /// MVT/PBF response bytes for recently changed public markers.
    pub bytes: Vec<u8>,
    /// MVT source-layer name.
    pub layer_name: &'static str,
    /// Number of changed marker features represented.
    pub feature_count: i64,
    /// Highest projection version represented by this delta response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity represented by this delta response.
    pub anchor_snapshot_id: Option<String>,
}

/// Exact listing marker count and projection metadata for a normalized filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerCount {
    /// Exact public marker count for the filter.
    pub total_count: i64,
    /// Highest projection version included in the count result.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in the count result.
    pub anchor_snapshot_id: Option<String>,
}

/// Registered listing marker filter identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListingMarkerRegisteredFilter {
    /// Stable filter hash used by public tile/count/mask routes.
    pub filter_hash: String,
}

#[cfg(test)]
mod tests {
    use super::ListingMarkerOverlayTileQuery;

    #[test]
    fn listing_marker_overlay_query_rejects_out_of_range_tiles() {
        assert!(ListingMarkerOverlayTileQuery::try_new(23, 0, 0, None).is_err());
        assert!(ListingMarkerOverlayTileQuery::try_new(4, 16, 0, None).is_err());
        assert!(ListingMarkerOverlayTileQuery::try_new(4, 0, 16, None).is_err());
    }
}
