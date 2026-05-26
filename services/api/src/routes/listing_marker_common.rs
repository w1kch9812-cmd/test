//! Shared helpers for listing marker public map routes.

use std::sync::Arc;

use axum::http::StatusCode;
use listing_domain::repository::{
    ListingMarkerFilter, ListingRepository, ALL_ACTIVE_LISTING_MARKER_FILTER_HASH,
};

use crate::http::problem::{problem, ProblemResponse};

/// Validate the public marker filter hash character set before repository lookup.
#[must_use]
pub fn is_stable_filter_hash(value: &str) -> bool {
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
}

/// Resolve a public listing marker filter hash to the typed filter payload.
///
/// Built-in filters remain code-owned. User-composed filters are resolved from the registry, so
/// public tile/count routes never infer filter semantics from an opaque hash string.
///
/// # Errors
///
/// Returns a problem response when the hash is unknown or the registry lookup fails.
pub async fn resolve_listing_marker_filter(
    repo: &Arc<dyn ListingRepository>,
    filter_hash: &str,
) -> Result<ListingMarkerFilter, ProblemResponse> {
    if filter_hash == ALL_ACTIVE_LISTING_MARKER_FILTER_HASH {
        return Ok(ListingMarkerFilter::AllActive);
    }

    let spec = repo
        .resolve_listing_marker_filter(filter_hash)
        .await
        .map_err(|e| {
            tracing::warn!(error = %e, filter_hash, "listing marker filter lookup failed");
            problem(
                "map/listing-marker-filter-unavailable",
                "listing marker filter is unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
                None,
            )
        })?;

    spec.map(ListingMarkerFilter::Normalized).ok_or_else(|| {
        problem(
            "map/listing-marker-filter-not-found",
            "listing marker filter was not found",
            StatusCode::NOT_FOUND,
            None,
        )
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

    use std::sync::Arc;

    use async_trait::async_trait;
    use listing_domain::entity::Listing;
    use listing_domain::repository::{
        CardSearchQuery, ListingCardSummary, ListingDetail, ListingMarkerCount,
        ListingMarkerFilter, ListingMarkerFilterSpec, ListingMarkerMask, ListingMarkerMaskQuery,
        ListingMarkerRegisteredFilter, ListingMarkerTile, ListingMarkerTileQuery,
        ListingParcelDenormalize, ListingRepository, NormalizedListingMarkerFilterSpec, RepoError,
    };
    use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
    use shared_kernel::listing_status::ListingStatus;
    use shared_kernel::listing_type::ListingType;
    use shared_kernel::mutation::MutationContext;
    use shared_kernel::transaction_type::TransactionType;

    use super::*;

    #[tokio::test]
    async fn resolves_registered_listing_filter_hash_from_repository() {
        let filter = ListingMarkerFilterSpec {
            types: vec![ListingType::Warehouse],
            transactions: vec![TransactionType::Sale],
            min_area_m2: Some(300),
            max_area_m2: Some(400),
            min_price_krw: Some(100_000_000),
            max_price_krw: Some(900_000_000),
        }
        .try_normalized()
        .expect("normalized filter");
        let filter_hash = filter.filter_hash();
        let repo: Arc<dyn ListingRepository> = Arc::new(FakeListingRepository {
            filter_hash: filter_hash.clone(),
            filter: filter.clone(),
        });

        let resolved = resolve_listing_marker_filter(&repo, &filter_hash)
            .await
            .expect("registered filter");

        assert_eq!(resolved, ListingMarkerFilter::Normalized(filter));
    }

    struct FakeListingRepository {
        filter_hash: String,
        filter: NormalizedListingMarkerFilterSpec,
    }

    #[async_trait]
    impl ListingRepository for FakeListingRepository {
        async fn find(&self, _id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError> {
            panic!("unused")
        }

        async fn find_card_summaries(
            &self,
            _query: CardSearchQuery,
        ) -> Result<(Vec<ListingCardSummary>, u64), RepoError> {
            panic!("unused")
        }

        async fn find_listing_marker_tile(
            &self,
            _query: ListingMarkerTileQuery,
        ) -> Result<ListingMarkerTile, RepoError> {
            panic!("unused")
        }

        async fn find_listing_marker_mask(
            &self,
            _query: ListingMarkerMaskQuery,
        ) -> Result<ListingMarkerMask, RepoError> {
            panic!("unused")
        }

        async fn upsert_listing_marker_projection(
            &self,
            _id: &Id<ListingIdMarker>,
        ) -> Result<(), RepoError> {
            panic!("unused")
        }

        async fn count_listing_markers(
            &self,
            _filter: NormalizedListingMarkerFilterSpec,
        ) -> Result<ListingMarkerCount, RepoError> {
            panic!("unused")
        }

        async fn register_listing_marker_filter(
            &self,
            _filter: NormalizedListingMarkerFilterSpec,
        ) -> Result<ListingMarkerRegisteredFilter, RepoError> {
            panic!("unused")
        }

        async fn resolve_listing_marker_filter(
            &self,
            filter_hash: &str,
        ) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError> {
            Ok((filter_hash == self.filter_hash).then(|| self.filter.clone()))
        }

        async fn find_by_owner(
            &self,
            _owner_id: &Id<UserMarker>,
            _status: Option<ListingStatus>,
        ) -> Result<Vec<Listing>, RepoError> {
            panic!("unused")
        }

        async fn save(&self, _listing: &Listing, _ctx: MutationContext) -> Result<(), RepoError> {
            panic!("unused")
        }

        async fn find_detail_by_id(
            &self,
            _id: &Id<ListingIdMarker>,
            _viewer_user_id: &Id<UserMarker>,
        ) -> Result<Option<ListingDetail>, RepoError> {
            panic!("unused")
        }

        async fn increment_view_count(&self, _id: &Id<ListingIdMarker>) -> Result<(), RepoError> {
            panic!("unused")
        }

        async fn update_parcel_denormalize(
            &self,
            _id: &Id<ListingIdMarker>,
            _denormalize: &ListingParcelDenormalize,
        ) -> Result<(), RepoError> {
            panic!("unused")
        }
    }
}
