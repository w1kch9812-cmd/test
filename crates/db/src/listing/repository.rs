#![allow(clippy::needless_pass_by_value)]

use async_trait::async_trait;
use listing_domain::entity::Listing;
use listing_domain::repository::{
    CardSearchQuery, ListingCardSummary, ListingDetail, ListingMarkerCount, ListingMarkerDeltas,
    ListingMarkerMask, ListingMarkerMaskQuery, ListingMarkerOverlayTileQuery,
    ListingMarkerRegisteredFilter, ListingMarkerTile, ListingMarkerTileQuery,
    ListingMarkerTombstones, ListingParcelDenormalize, ListingRepository,
    NormalizedListingMarkerFilterSpec, RepoError,
};
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::mutation::MutationContext;
use tracing::instrument;

use super::rows::{row_to_listing, LISTING_FULL_COLUMNS};
use super::{
    card_summaries, detail, marker_count, marker_delta, marker_filter_registry, marker_mask,
    marker_projection, marker_tile, marker_tombstone, persistence, PgListingRepository,
};
use crate::error_map::map_sqlx_err;

#[async_trait]
impl ListingRepository for PgListingRepository {
    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn find(&self, id: &Id<ListingIdMarker>) -> Result<Option<Listing>, RepoError> {
        let sql = format!("select {LISTING_FULL_COLUMNS} from listing where id = $1");
        let row = sqlx::query(&sql)
            .bind(id.as_str())
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        row.as_ref().map(row_to_listing).transpose()
    }

    #[instrument(skip(self, query))]
    async fn find_card_summaries(
        &self,
        query: CardSearchQuery,
    ) -> Result<(Vec<ListingCardSummary>, u64), RepoError> {
        card_summaries::find_card_summaries(&self.pool, query).await
    }

    #[instrument(skip(self), fields(
        z = query.z,
        x = query.x,
        y = query.y,
        filter_hash = query.filter.hash(),
    ))]
    async fn find_listing_marker_tile(
        &self,
        query: ListingMarkerTileQuery,
    ) -> Result<ListingMarkerTile, RepoError> {
        marker_tile::find_listing_marker_tile(&self.pool, query).await
    }

    #[instrument(skip(self), fields(
        z = query.z,
        x = query.x,
        y = query.y,
        filter_hash = query.filter.hash(),
        base_version = ?query.base_version,
    ))]
    async fn find_listing_marker_mask(
        &self,
        query: ListingMarkerMaskQuery,
    ) -> Result<ListingMarkerMask, RepoError> {
        marker_mask::find_listing_marker_mask(&self.pool, query).await
    }

    #[instrument(skip(self), fields(
        z = query.z,
        x = query.x,
        y = query.y,
        base_version = ?query.base_version,
    ))]
    async fn find_listing_marker_tombstones(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerTombstones, RepoError> {
        marker_tombstone::find_listing_marker_tombstones(&self.pool, query).await
    }

    #[instrument(skip(self), fields(
        z = query.z,
        x = query.x,
        y = query.y,
        base_version = ?query.base_version,
    ))]
    async fn find_listing_marker_deltas(
        &self,
        query: ListingMarkerOverlayTileQuery,
    ) -> Result<ListingMarkerDeltas, RepoError> {
        marker_delta::find_listing_marker_deltas(&self.pool, query).await
    }

    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn upsert_listing_marker_projection(
        &self,
        id: &Id<ListingIdMarker>,
    ) -> Result<(), RepoError> {
        marker_projection::upsert_listing_marker_projection(&self.pool, id).await
    }

    #[instrument(skip(self, filter), fields(filter_hash = %filter.filter_hash()))]
    async fn count_listing_markers(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerCount, RepoError> {
        marker_count::count_listing_markers(&self.pool, filter).await
    }

    #[instrument(skip(self, filter), fields(filter_hash = %filter.filter_hash()))]
    async fn register_listing_marker_filter(
        &self,
        filter: NormalizedListingMarkerFilterSpec,
    ) -> Result<ListingMarkerRegisteredFilter, RepoError> {
        marker_filter_registry::register_listing_marker_filter(&self.pool, filter).await
    }

    #[instrument(skip(self), fields(filter_hash = %filter_hash))]
    async fn resolve_listing_marker_filter(
        &self,
        filter_hash: &str,
    ) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError> {
        marker_filter_registry::resolve_listing_marker_filter(&self.pool, filter_hash).await
    }

    #[instrument(skip(self), fields(owner_id = %owner_id.as_str()))]
    async fn find_by_owner(
        &self,
        owner_id: &Id<UserMarker>,
        status: Option<ListingStatus>,
    ) -> Result<Vec<Listing>, RepoError> {
        let rows = if let Some(s) = status {
            let sql = format!(
                "select {LISTING_FULL_COLUMNS} from listing \
                 where owner_id = $1 and status = $2 order by created_at desc"
            );
            sqlx::query(&sql)
                .bind(owner_id.as_str())
                .bind(s.as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_err)?
        } else {
            let sql = format!(
                "select {LISTING_FULL_COLUMNS} from listing \
                 where owner_id = $1 order by created_at desc"
            );
            sqlx::query(&sql)
                .bind(owner_id.as_str())
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx_err)?
        };
        rows.iter().map(row_to_listing).collect()
    }

    #[instrument(skip(self, listing, ctx), fields(
        listing_id = %listing.id.as_str(),
        version = listing.version,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, listing: &Listing, ctx: MutationContext) -> Result<(), RepoError> {
        persistence::save(&self.pool, listing, ctx).await
    }

    #[instrument(skip(self), fields(
        listing_id = %id.as_str(),
        viewer = %viewer_user_id.as_str(),
    ))]
    async fn find_detail_by_id(
        &self,
        id: &Id<ListingIdMarker>,
        viewer_user_id: &Id<UserMarker>,
    ) -> Result<Option<ListingDetail>, RepoError> {
        detail::find_detail_by_id(&self.pool, id, viewer_user_id).await
    }

    #[instrument(skip(self), fields(listing_id = %id.as_str()))]
    async fn increment_view_count(&self, id: &Id<ListingIdMarker>) -> Result<(), RepoError> {
        let result = sqlx::query(
            r"
            UPDATE listing
            SET view_count = view_count + 1, updated_at = now()
            WHERE id = $1
            ",
        )
        .bind(id.as_str())
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }

    #[instrument(skip(self), fields(listing_id = %id))]
    async fn update_parcel_denormalize(
        &self,
        id: &Id<ListingIdMarker>,
        denormalize: &ListingParcelDenormalize,
    ) -> Result<(), RepoError> {
        let result = sqlx::query(
            r"
            UPDATE listing
            SET admin_code = $2,
                parcel_land_use_type = $3,
                parcel_zoning = $4,
                parcel_lookup_at = now()
            WHERE id = $1
            ",
        )
        .bind(id.as_str())
        .bind(denormalize.admin_code.as_str())
        .bind(denormalize.land_use_type.as_str())
        .bind(denormalize.zoning.as_ref().map(|z| z.as_str()))
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
