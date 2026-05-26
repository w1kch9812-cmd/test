use listing_domain::repository::{
    ListingMarkerMask, ListingMarkerMaskEncoding, ListingMarkerMaskQuery, RepoError,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn find_listing_marker_mask(
    pool: &PgPool,
    query: ListingMarkerMaskQuery,
) -> Result<ListingMarkerMask, RepoError> {
    let filter = query.filter.into_spec();
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
        WITH matching AS (
            SELECT
                marker_id,
                projection_version,
                anchor_snapshot_id
            FROM listing_marker_projection
            WHERE listing_status = 'active'
              AND visibility_scope = 'public'
              AND (cardinality($4::text[]) = 0 OR listing_type = ANY($4::text[]))
              AND (cardinality($5::text[]) = 0 OR transaction_type = ANY($5::text[]))
              AND ($6::numeric IS NULL OR area_m2 >= $6::numeric)
              AND ($7::numeric IS NULL OR area_m2 <= $7::numeric)
              AND ($8::bigint IS NULL OR price_krw >= $8::bigint)
              AND ($9::bigint IS NULL OR price_krw <= $9::bigint)
              AND ST_Intersects(
                  ST_Transform(anchor_point, 3857),
                  ST_TileEnvelope($1, $2, $3)
              )
        )
        SELECT
            COALESCE(array_agg(marker_id ORDER BY marker_id), ARRAY[]::text[]) AS marker_ids,
            max(projection_version)::int8 AS projection_version,
            max(anchor_snapshot_id) AS anchor_snapshot_id
        FROM matching
        ",
    )
    .bind(i32::from(query.z))
    .bind(i32::try_from(query.x).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(i32::try_from(query.y).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(types)
    .bind(transactions)
    .bind(filter.min_area_m2)
    .bind(filter.max_area_m2)
    .bind(filter.min_price_krw)
    .bind(filter.max_price_krw)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerMask {
        encoding: ListingMarkerMaskEncoding::Show,
        marker_ids: row.try_get("marker_ids").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
