//! Spatial query scopes shared by product-side read ports.
//!
//! Public map traffic should use tile-shaped contracts. Internal product ports may still need
//! `PNU` or administrative scopes, so this type makes the query shape explicit instead of
//! leaking raw `bbox` terminology into market-domain contracts.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::admin_division::{EupmyeondongCode, SidoCode, SigunguCode};
use crate::pnu::Pnu;

/// Maximum Web Mercator tile zoom accepted by product read ports.
pub const MAX_MAP_TILE_Z: u8 = 22;

/// Slippy-map tile coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapTile {
    /// Zoom level.
    z: u8,
    /// X coordinate at `z`.
    x: u32,
    /// Y coordinate at `z`.
    y: u32,
}

impl MapTile {
    /// Create a validated tile coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`MapTileError`] when `z` is above [`MAX_MAP_TILE_Z`] or when `x`/`y` are outside
    /// the axis range for the zoom.
    pub fn try_new(z: u8, x: u32, y: u32) -> Result<Self, MapTileError> {
        if z > MAX_MAP_TILE_Z {
            return Err(MapTileError::ZoomOutOfRange {
                z,
                max: MAX_MAP_TILE_Z,
            });
        }
        let axis_limit = 1_u32 << z;
        if x >= axis_limit {
            return Err(MapTileError::XOutOfRange { z, x, axis_limit });
        }
        if y >= axis_limit {
            return Err(MapTileError::YOutOfRange { z, y, axis_limit });
        }
        Ok(Self { z, x, y })
    }

    /// Zoom level.
    #[must_use]
    pub const fn z(&self) -> u8 {
        self.z
    }

    /// X coordinate.
    #[must_use]
    pub const fn x(&self) -> u32 {
        self.x
    }

    /// Y coordinate.
    #[must_use]
    pub const fn y(&self) -> u32 {
        self.y
    }
}

/// Tile coordinate validation error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum MapTileError {
    /// Zoom is above the supported maximum.
    #[error("tile z must be <= {max}, got {z}")]
    ZoomOutOfRange {
        /// Actual zoom.
        z: u8,
        /// Maximum accepted zoom.
        max: u8,
    },
    /// X is outside the zoom axis.
    #[error("tile x out of range for z={z}: x={x}, axis_limit={axis_limit}")]
    XOutOfRange {
        /// Zoom level.
        z: u8,
        /// Actual x coordinate.
        x: u32,
        /// Exclusive axis upper bound.
        axis_limit: u32,
    },
    /// Y is outside the zoom axis.
    #[error("tile y out of range for z={z}: y={y}, axis_limit={axis_limit}")]
    YOutOfRange {
        /// Zoom level.
        z: u8,
        /// Actual y coordinate.
        y: u32,
        /// Exclusive axis upper bound.
        axis_limit: u32,
    },
}

/// Product-side spatial query scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SpatialScope {
    /// Single parcel identity.
    Pnu(Pnu),
    /// Eupmyeondong administrative area.
    Eupmyeondong(EupmyeondongCode),
    /// Sigungu administrative area.
    Sigungu(SigunguCode),
    /// Sido administrative area.
    Sido(SidoCode),
    /// Tile-shaped map scope.
    Tile(MapTile),
}

impl SpatialScope {
    /// Build a `PNU` scope.
    #[must_use]
    pub const fn pnu(pnu: Pnu) -> Self {
        Self::Pnu(pnu)
    }

    /// Build an eupmyeondong scope.
    #[must_use]
    pub const fn eupmyeondong(code: EupmyeondongCode) -> Self {
        Self::Eupmyeondong(code)
    }

    /// Build a sigungu scope.
    #[must_use]
    pub const fn sigungu(code: SigunguCode) -> Self {
        Self::Sigungu(code)
    }

    /// Build a sido scope.
    #[must_use]
    pub const fn sido(code: SidoCode) -> Self {
        Self::Sido(code)
    }

    /// Build a tile scope.
    #[must_use]
    pub const fn tile(tile: MapTile) -> Self {
        Self::Tile(tile)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn map_tile_accepts_axis_boundary_minus_one() {
        let tile = MapTile::try_new(14, 16_383, 16_383).unwrap();
        assert_eq!(tile.z(), 14);
        assert_eq!(tile.x(), 16_383);
        assert_eq!(tile.y(), 16_383);
    }

    #[test]
    fn map_tile_rejects_zoom_above_supported_max() {
        assert_eq!(
            MapTile::try_new(23, 0, 0),
            Err(MapTileError::ZoomOutOfRange {
                z: 23,
                max: MAX_MAP_TILE_Z,
            })
        );
    }

    #[test]
    fn map_tile_rejects_axis_limit() {
        assert_eq!(
            MapTile::try_new(2, 4, 0),
            Err(MapTileError::XOutOfRange {
                z: 2,
                x: 4,
                axis_limit: 4,
            })
        );
        assert_eq!(
            MapTile::try_new(2, 0, 4),
            Err(MapTileError::YOutOfRange {
                z: 2,
                y: 4,
                axis_limit: 4,
            })
        );
    }

    #[test]
    fn spatial_scope_wraps_tile() {
        let tile = MapTile::try_new(12, 1, 2).unwrap();
        assert!(matches!(SpatialScope::tile(tile), SpatialScope::Tile(t) if t == tile));
    }
}
