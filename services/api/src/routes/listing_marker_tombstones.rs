//! Gongzzang-owned listing marker tombstone route.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use listing_domain::repository::ListingMarkerOverlayTileQuery;
use serde::{Deserialize, Serialize};

use crate::http::problem::{problem, ProblemResponse};
use crate::listing_marker_serving::ListingMarkerServingGateway;

/// Shared state for listing marker tombstone routes.
#[derive(Clone)]
pub struct ListingMarkerTombstonesState {
    /// Gongzzang marker serving gateway.
    pub serving: Arc<ListingMarkerServingGateway>,
}

/// `GET /map/v1/marker-tombstones/listing/:z/:x/:y` query parameters.
#[derive(Debug, Deserialize)]
pub struct ListingMarkerTombstoneHttpQuery {
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

/// Listing marker tombstone response.
#[derive(Debug, Serialize)]
pub struct ListingMarkerTombstoneResponse {
    /// Tombstones always identify marker ids to hide.
    pub encoding: String,
    /// Marker ids that must be hidden.
    pub marker_ids: Vec<String>,
    /// Highest projection version included in this tombstone response.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in this tombstone response.
    pub anchor_snapshot_id: Option<String>,
}

/// `GET /map/v1/marker-tombstones/listing/:z/:x/:y`.
#[tracing::instrument(skip(state), fields(
    z = %z_raw,
    x = %x_raw,
    y = %y_raw,
    base_version = ?q.base_version,
))]
pub async fn get_listing_marker_tombstones(
    State(state): State<ListingMarkerTombstonesState>,
    Path((z_raw, x_raw, y_raw)): Path<(String, String, String)>,
    Query(q): Query<ListingMarkerTombstoneHttpQuery>,
) -> Result<Json<ListingMarkerTombstoneResponse>, ProblemResponse> {
    let query = parse_listing_marker_tombstone_query(&z_raw, &x_raw, &y_raw, q.base_version)?;

    let tombstones = state
        .serving
        .find_listing_marker_tombstones(query)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "listing marker tombstone query failed");
            if e.to_string().contains("budget violation") {
                problem(
                    "map/listing-marker-tombstone-unrepresentable",
                    "listing marker tombstones cannot be represented truthfully",
                    StatusCode::UNPROCESSABLE_ENTITY,
                    None,
                )
            } else {
                problem(
                    "map/listing-marker-tombstone-unavailable",
                    "listing marker tombstones are unavailable",
                    StatusCode::SERVICE_UNAVAILABLE,
                    None,
                )
            }
        })?;

    Ok(Json(ListingMarkerTombstoneResponse {
        encoding: "hide".to_owned(),
        marker_ids: tombstones.marker_ids,
        projection_version: tombstones.projection_version,
        anchor_snapshot_id: tombstones.anchor_snapshot_id,
    }))
}

fn parse_listing_marker_tombstone_query(
    z_raw: &str,
    x_raw: &str,
    y_raw: &str,
    base_version: Option<i64>,
) -> Result<ListingMarkerOverlayTileQuery, ProblemResponse> {
    let z = z_raw.parse::<u8>().map_err(|e| {
        problem(
            "map/listing-marker-tombstone-coordinate",
            "tile z coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let x = x_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-tombstone-coordinate",
            "tile x coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let y = y_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-tombstone-coordinate",
            "tile y coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    ListingMarkerOverlayTileQuery::try_new(z, x, y, base_version).map_err(|e| {
        problem(
            "map/listing-marker-tombstone-coordinate",
            "tile coordinate is outside the supported range",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;

    #[test]
    fn tombstone_response_uses_hide_encoding() {
        let response = ListingMarkerTombstoneResponse {
            encoding: "hide".to_owned(),
            marker_ids: vec!["lm_test".to_owned()],
            projection_version: Some(2),
            anchor_snapshot_id: Some("snapshot-test-v1".to_owned()),
        };

        assert_eq!(response.encoding, "hide");
        assert_eq!(response.marker_ids, vec!["lm_test"]);
        assert_eq!(response.projection_version, Some(2));
    }

    #[test]
    fn parse_tombstone_query_rejects_invalid_coordinates() {
        let err = parse_listing_marker_tombstone_query("23", "0", "0", None)
            .expect_err("z above max must fail");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }
}
