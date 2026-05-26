//! Gongzzang-owned listing marker mask route.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use listing_domain::repository::{
    ListingMarkerFilter, ListingMarkerMaskQuery, ListingMarkerTileQuery, ListingRepository,
};
use serde::{Deserialize, Serialize};

use crate::http::problem::{problem, ProblemResponse};
use crate::routes::listing_marker_common::{is_stable_filter_hash, resolve_listing_marker_filter};

/// Shared state for listing marker mask routes.
#[derive(Clone)]
pub struct ListingMarkerMasksState {
    /// Gongzzang listing repository.
    pub listing_repo: Arc<dyn ListingRepository>,
}

/// `GET /map/v1/marker-masks/listing/:z/:x/:y` query parameters.
#[derive(Debug, Deserialize)]
pub struct ListingMarkerMaskHttpQuery {
    /// Registered listing marker filter hash.
    pub filter_hash: Option<String>,
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

/// Listing marker mask response.
#[derive(Debug, Serialize)]
pub struct ListingMarkerMaskResponse {
    /// Mask encoding. Initially `show`.
    pub encoding: String,
    /// Marker ids selected by the mask.
    pub marker_ids: Vec<String>,
    /// Highest projection version included in this mask.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in this mask.
    pub anchor_snapshot_id: Option<String>,
}

/// `GET /map/v1/marker-masks/listing/:z/:x/:y`.
#[tracing::instrument(skip(state), fields(
    z = %z_raw,
    x = %x_raw,
    y = %y_raw,
    filter_hash = ?q.filter_hash,
    base_version = ?q.base_version,
))]
pub async fn get_listing_marker_mask(
    State(state): State<ListingMarkerMasksState>,
    Path((z_raw, x_raw, y_raw)): Path<(String, String, String)>,
    Query(q): Query<ListingMarkerMaskHttpQuery>,
) -> Result<Json<ListingMarkerMaskResponse>, ProblemResponse> {
    let filter_hash = q
        .filter_hash
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            problem(
                "map/listing-marker-filter-missing",
                "filter_hash is required",
                StatusCode::BAD_REQUEST,
                None,
            )
        })?;

    if !is_stable_filter_hash(filter_hash) {
        return Err(problem(
            "map/listing-marker-filter-malformed",
            "filter_hash is malformed",
            StatusCode::BAD_REQUEST,
            None,
        ));
    }

    let filter = resolve_listing_marker_filter(&state.listing_repo, filter_hash).await?;
    let mask_query =
        parse_listing_marker_mask_query(&z_raw, &x_raw, &y_raw, filter, q.base_version)?;

    let mask = state
        .listing_repo
        .find_listing_marker_mask(mask_query)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "listing marker mask query failed");
            problem(
                "map/listing-marker-mask-unavailable",
                "listing marker mask is unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;

    if q.base_version.is_some() && mask.projection_version != q.base_version {
        return Err(problem(
            "map/listing-marker-mask-stale",
            "listing marker base tile version is stale",
            StatusCode::CONFLICT,
            Some(format!(
                "base_version={:?}, projection_version={:?}",
                q.base_version, mask.projection_version
            )),
        ));
    }

    Ok(Json(ListingMarkerMaskResponse {
        encoding: mask.encoding.as_str().to_owned(),
        marker_ids: mask.marker_ids,
        projection_version: mask.projection_version,
        anchor_snapshot_id: mask.anchor_snapshot_id,
    }))
}

fn parse_listing_marker_mask_query(
    z_raw: &str,
    x_raw: &str,
    y_raw: &str,
    filter: ListingMarkerFilter,
    base_version: Option<i64>,
) -> Result<ListingMarkerMaskQuery, ProblemResponse> {
    let z = z_raw.parse::<u8>().map_err(|e| {
        problem(
            "map/listing-marker-mask-coordinate",
            "tile z coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let x = x_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-mask-coordinate",
            "tile x coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let y = y_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-mask-coordinate",
            "tile y coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    ListingMarkerTileQuery::try_new(z, x, y, filter.clone()).map_err(|e| {
        problem(
            "map/listing-marker-mask-coordinate",
            "tile coordinate is outside the supported range",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    Ok(ListingMarkerMaskQuery {
        z,
        x,
        y,
        filter,
        base_version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_response_uses_stable_encoding() {
        let response = ListingMarkerMaskResponse {
            encoding: "show".to_owned(),
            marker_ids: vec!["lm_test".to_owned()],
            projection_version: Some(1),
            anchor_snapshot_id: Some("snapshot-test-v1".to_owned()),
        };

        assert_eq!(response.encoding, "show");
        assert_eq!(response.marker_ids, vec!["lm_test"]);
        assert_eq!(response.projection_version, Some(1));
    }
}
