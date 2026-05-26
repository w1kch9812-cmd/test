//! Gongzzang-owned listing marker filter registration route.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use listing_domain::repository::{ListingMarkerFilterSpec, ListingRepository};
use serde::{Deserialize, Serialize};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;

use crate::http::problem::{problem, ProblemResponse};

/// Shared state for listing marker filter registration routes.
#[derive(Clone)]
pub struct ListingMarkerFiltersState {
    /// Gongzzang listing repository.
    pub listing_repo: Arc<dyn ListingRepository>,
}

/// `POST /map/v1/marker-filters/listing` payload.
#[derive(Debug, Deserialize)]
pub struct ListingMarkerFilterRequest {
    /// Listing asset types selected by the user.
    pub types: Vec<ListingType>,
    /// Transaction types selected by the user.
    pub transactions: Vec<TransactionType>,
    /// Inclusive minimum area in square meters.
    pub min_area_m2: Option<i64>,
    /// Inclusive maximum area in square meters.
    pub max_area_m2: Option<i64>,
    /// Inclusive minimum price in Korean won.
    pub min_price_krw: Option<i64>,
    /// Inclusive maximum price in Korean won.
    pub max_price_krw: Option<i64>,
}

/// Registered listing marker filter response.
#[derive(Debug, Serialize)]
pub struct ListingMarkerFilterResponse {
    /// Stable filter hash used by public marker routes.
    pub filter_hash: String,
}

/// `POST /map/v1/marker-filters/listing`.
#[tracing::instrument(skip(state, request))]
pub async fn post_listing_marker_filter(
    State(state): State<ListingMarkerFiltersState>,
    Json(request): Json<ListingMarkerFilterRequest>,
) -> Result<Json<ListingMarkerFilterResponse>, ProblemResponse> {
    let normalized = ListingMarkerFilterSpec {
        types: request.types,
        transactions: request.transactions,
        min_area_m2: request.min_area_m2,
        max_area_m2: request.max_area_m2,
        min_price_krw: request.min_price_krw,
        max_price_krw: request.max_price_krw,
    }
    .try_normalized()
    .map_err(|e| {
        problem(
            "map/listing-marker-filter-invalid",
            "listing marker filter is invalid",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let registered = state
        .listing_repo
        .register_listing_marker_filter(normalized)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, "listing marker filter registration failed");
            problem(
                "map/listing-marker-filter-unavailable",
                "listing marker filter is unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;

    Ok(Json(ListingMarkerFilterResponse {
        filter_hash: registered.filter_hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_response_hash_uses_stable_prefix() {
        let response = ListingMarkerFilterResponse {
            filter_hash: "lst_filter_v1_abc".to_owned(),
        };

        assert!(response.filter_hash.starts_with("lst_filter_v1_"));
    }
}
