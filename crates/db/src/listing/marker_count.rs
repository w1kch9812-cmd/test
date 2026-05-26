use listing_domain::repository::{
    ListingMarkerCount, NormalizedListingMarkerFilterSpec, RepoError,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn count_listing_markers(
    pool: &PgPool,
    filter: NormalizedListingMarkerFilterSpec,
) -> Result<ListingMarkerCount, RepoError> {
    let types = filter
        .types
        .iter()
        .map(|value| value.as_str().to_owned())
        .collect::<Vec<_>>();
    let transactions = filter
        .transactions
        .iter()
        .map(|value| value.as_str().to_owned())
        .collect::<Vec<_>>();

    let row = sqlx::query(
        r"
        select
            count(*)::int8 as total_count,
            max(projection_version)::int8 as projection_version,
            max(anchor_snapshot_id) as anchor_snapshot_id
        from listing_marker_projection
        where listing_status = 'active'
          and visibility_scope = 'public'
          and (cardinality($1::text[]) = 0 or listing_type = any($1::text[]))
          and (cardinality($2::text[]) = 0 or transaction_type = any($2::text[]))
          and ($3::numeric is null or area_m2 >= $3::numeric)
          and ($4::numeric is null or area_m2 <= $4::numeric)
          and ($5::bigint is null or price_krw >= $5::bigint)
          and ($6::bigint is null or price_krw <= $6::bigint)
        ",
    )
    .bind(types)
    .bind(transactions)
    .bind(filter.min_area_m2)
    .bind(filter.max_area_m2)
    .bind(filter.min_price_krw)
    .bind(filter.max_price_krw)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerCount {
        total_count: row.try_get("total_count").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
