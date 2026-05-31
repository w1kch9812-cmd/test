//! Gongzzang-owned listing marker delta PBF route.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use listing_domain::repository::{ListingMarkerOverlayTileQuery, LISTING_MARKER_TILE_CONTENT_TYPE};
use serde::Deserialize;

use crate::http::problem::{problem, ProblemResponse};
use crate::listing_marker_serving::ListingMarkerServingGateway;

const DELTA_CACHE_CONTROL: &str = "public, max-age=5, stale-while-revalidate=10";

/// Shared state for listing marker delta routes.
#[derive(Clone)]
pub struct ListingMarkerDeltasState {
    /// Gongzzang marker serving gateway.
    pub serving: Arc<ListingMarkerServingGateway>,
}

/// `GET /map/v1/marker-deltas/listing/:z/:x/:y.pbf` query parameters.
#[derive(Debug, Deserialize)]
pub struct ListingMarkerDeltaHttpQuery {
    /// Projection version of the already loaded base tile.
    pub base_version: Option<i64>,
}

/// `GET /map/v1/marker-deltas/listing/:z/:x/:y.pbf`.
#[tracing::instrument(skip(state), fields(
    z = %z_raw,
    x = %x_raw,
    y = %y_pbf,
    base_version = ?q.base_version,
))]
pub async fn get_listing_marker_deltas(
    State(state): State<ListingMarkerDeltasState>,
    Path((z_raw, x_raw, y_pbf)): Path<(String, String, String)>,
    Query(q): Query<ListingMarkerDeltaHttpQuery>,
) -> Result<Response, ProblemResponse> {
    let query = parse_listing_marker_delta_query(&z_raw, &x_raw, &y_pbf, q.base_version)?;

    let deltas = state
        .serving
        .find_listing_marker_deltas(query)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "listing marker delta query failed");
            if e.to_string().contains("budget violation") {
                problem(
                    "map/listing-marker-delta-unrepresentable",
                    "listing marker deltas cannot be represented truthfully",
                    StatusCode::UNPROCESSABLE_ENTITY,
                    None,
                )
            } else {
                problem(
                    "map/listing-marker-delta-unavailable",
                    "listing marker deltas are unavailable",
                    StatusCode::SERVICE_UNAVAILABLE,
                    None,
                )
            }
        })?;

    tracing::info!(
        layer = deltas.layer_name,
        anchor_snapshot_id = ?deltas.anchor_snapshot_id,
        projection_version = ?deltas.projection_version,
        feature_count = deltas.feature_count,
        tile_byte_size = deltas.bytes.len(),
        "listing marker delta encoded"
    );

    Ok(delta_response(deltas.bytes))
}

fn delta_response(bytes: Vec<u8>) -> Response {
    let mut response = (StatusCode::OK, bytes).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(LISTING_MARKER_TILE_CONTENT_TYPE),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static(DELTA_CACHE_CONTROL),
    );
    response
}

fn parse_listing_marker_delta_query(
    z_raw: &str,
    x_raw: &str,
    y_pbf: &str,
    base_version: Option<i64>,
) -> Result<ListingMarkerOverlayTileQuery, ProblemResponse> {
    let y_raw = y_pbf.strip_suffix(".pbf").ok_or_else(|| {
        problem(
            "map/listing-marker-delta-path",
            "tile path must end with .pbf",
            StatusCode::BAD_REQUEST,
            None,
        )
    })?;
    let z = z_raw.parse::<u8>().map_err(|e| {
        problem(
            "map/listing-marker-delta-coordinate",
            "tile z coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let x = x_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-delta-coordinate",
            "tile x coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let y = y_raw.parse::<u32>().map_err(|e| {
        problem(
            "map/listing-marker-delta-coordinate",
            "tile y coordinate is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    ListingMarkerOverlayTileQuery::try_new(z, x, y, base_version).map_err(|e| {
        problem(
            "map/listing-marker-delta-coordinate",
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
    fn parses_listing_marker_delta_path_with_pbf_suffix() {
        let query =
            parse_listing_marker_delta_query("14", "8780", "6345.pbf", Some(42)).expect("query");

        assert_eq!(query.z, 14);
        assert_eq!(query.x, 8780);
        assert_eq!(query.y, 6345);
        assert_eq!(query.base_version, Some(42));
    }

    #[test]
    fn rejects_listing_marker_delta_path_without_pbf_suffix() {
        let err = parse_listing_marker_delta_query("14", "8780", "6345", None)
            .expect_err("missing pbf suffix must fail");

        assert_eq!(err.status, StatusCode::BAD_REQUEST);
    }

    #[test]
    fn delta_response_headers_are_mvt() {
        let response = delta_response(vec![1, 2, 3]);

        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static(LISTING_MARKER_TILE_CONTENT_TYPE))
        );
        assert_eq!(
            response.headers().get(header::CACHE_CONTROL),
            Some(&HeaderValue::from_static(DELTA_CACHE_CONTROL))
        );
    }
}
