//! Gongzzang-owned listing marker count route.

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::http::problem::{problem, ProblemResponse};
use crate::listing_marker_serving::ListingMarkerServingGateway;
use crate::routes::listing_marker_common::{is_stable_filter_hash, resolve_listing_marker_filter};

/// Shared state for listing marker count routes.
#[derive(Clone)]
pub struct ListingMarkerCountsState {
    /// Gongzzang marker serving gateway.
    pub serving: Arc<ListingMarkerServingGateway>,
}

/// `GET /map/v1/marker-counts/listing` query parameters.
#[derive(Debug, Deserialize)]
pub struct ListingMarkerCountHttpQuery {
    /// Registered listing marker filter hash.
    pub filter_hash: Option<String>,
}

/// Exact listing marker count response.
#[derive(Debug, Serialize)]
pub struct ListingMarkerCountResponse {
    /// Exact public marker count for the filter.
    pub total_count: i64,
    /// Highest projection version included in the count result.
    pub projection_version: Option<i64>,
    /// Highest anchor snapshot identity included in the count result.
    pub anchor_snapshot_id: Option<String>,
}

/// `GET /map/v1/marker-counts/listing`.
#[tracing::instrument(skip(state), fields(filter_hash = ?query.filter_hash))]
pub async fn get_listing_marker_count(
    State(state): State<ListingMarkerCountsState>,
    Query(query): Query<ListingMarkerCountHttpQuery>,
) -> Result<Json<ListingMarkerCountResponse>, ProblemResponse> {
    let filter_hash = query
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

    let filter = resolve_listing_marker_filter(&state.serving, filter_hash).await?;

    let count = state
        .serving
        .count_listing_markers(filter_hash, filter.into_spec())
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "listing marker count query failed");
            problem(
                "map/listing-marker-count-unavailable",
                "listing marker count is unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;

    Ok(Json(ListingMarkerCountResponse {
        total_count: count.total_count,
        projection_version: count.projection_version,
        anchor_snapshot_id: count.anchor_snapshot_id,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_preserves_projection_metadata() {
        let response = ListingMarkerCountResponse {
            total_count: 12,
            projection_version: Some(42),
            anchor_snapshot_id: Some("snapshot-test-v1".to_owned()),
        };

        assert_eq!(response.total_count, 12);
        assert_eq!(response.projection_version, Some(42));
        assert_eq!(
            response.anchor_snapshot_id.as_deref(),
            Some("snapshot-test-v1")
        );
    }
}
