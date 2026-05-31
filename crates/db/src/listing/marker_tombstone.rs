use listing_domain::repository::{
    ListingMarkerOverlayTileQuery, ListingMarkerTombstones, RepoError,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn find_listing_marker_tombstones(
    pool: &PgPool,
    query: ListingMarkerOverlayTileQuery,
) -> Result<ListingMarkerTombstones, RepoError> {
    let row = sqlx::query(
        r"
        with matching as (
            select marker_id, projection_version, anchor_snapshot_id
            from listing_marker_tombstone_log
            where expires_at > now()
              and ($4::bigint is null or projection_version > $4::bigint)
              and ST_Intersects(
                  ST_Transform(
                      ST_SetSRID(ST_MakePoint(
                          ((z14_tile_x::float8 + 0.5) / 16384.0) * 360.0 - 180.0,
                          degrees(
                              atan(
                                  sinh(
                                      pi() * (
                                          1.0 - 2.0 * (
                                              (z14_tile_y::float8 + 0.5) / 16384.0
                                          )
                                      )
                                  )
                              )
                          )
                      ), 4326),
                      3857
                  ),
                  ST_TileEnvelope($1, $2, $3)
              )
        )
        select
            coalesce(array_agg(marker_id order by marker_id), array[]::text[]) as marker_ids,
            max(projection_version)::int8 as projection_version,
            max(anchor_snapshot_id) as anchor_snapshot_id
        from matching
        ",
    )
    .bind(i32::from(query.z))
    .bind(i32::try_from(query.x).map_err(|error| RepoError::Database(error.to_string()))?)
    .bind(i32::try_from(query.y).map_err(|error| RepoError::Database(error.to_string()))?)
    .bind(query.base_version)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerTombstones {
        marker_ids: row.try_get("marker_ids").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
