use listing_domain::repository::{
    ListingMarkerDeltas, ListingMarkerOverlayTileQuery, RepoError, LISTING_MARKER_DELTA_TILE_LAYER,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn find_listing_marker_deltas(
    pool: &PgPool,
    query: ListingMarkerOverlayTileQuery,
) -> Result<ListingMarkerDeltas, RepoError> {
    let row = sqlx::query(
        r"
        with matching as (
            select
                p.marker_id,
                p.listing_id,
                p.pnu,
                p.anchor_point,
                p.anchor_snapshot_id,
                p.projection_version,
                p.rank_score,
                p.listing_type,
                p.transaction_type,
                p.price_krw,
                p.area_m2
            from listing_marker_delta_log d
            join listing_marker_projection p on p.marker_id = d.marker_id
            where d.expires_at > now()
              and p.listing_status = 'active'
              and p.visibility_scope = 'public'
              and ($4::bigint is null or p.projection_version > $4::bigint)
              and ST_Intersects(
                  ST_Transform(p.anchor_point, 3857),
                  ST_TileEnvelope($1, $2, $3)
              )
        ),
        features as (
            select
                marker_id as id,
                pnu,
                'listing_delta'::text as kind,
                1::int4 as count,
                rank_score as rank,
                listing_id::text as detail_ref,
                projection_version,
                anchor_snapshot_id,
                listing_type,
                transaction_type,
                price_krw,
                area_m2::float8 as area_m2,
                ST_AsMVTGeom(
                    ST_Transform(anchor_point, 3857),
                    ST_TileEnvelope($1, $2, $3),
                    4096,
                    256,
                    true
                ) as geom
            from matching
        )
        select
            coalesce(
                (select ST_AsMVT(features, $5, 4096, 'geom') from features),
                '\x'::bytea
            ) as bytes,
            (select count(*)::int8 from features where geom is not null) as feature_count,
            (select max(projection_version)::int8 from matching) as projection_version,
            (select max(anchor_snapshot_id) from matching) as anchor_snapshot_id
        ",
    )
    .bind(i32::from(query.z))
    .bind(i32::try_from(query.x).map_err(|error| RepoError::Database(error.to_string()))?)
    .bind(i32::try_from(query.y).map_err(|error| RepoError::Database(error.to_string()))?)
    .bind(query.base_version)
    .bind(LISTING_MARKER_DELTA_TILE_LAYER)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerDeltas {
        bytes: row.try_get("bytes").map_err(map_sqlx_err)?,
        layer_name: LISTING_MARKER_DELTA_TILE_LAYER,
        feature_count: row.try_get("feature_count").map_err(map_sqlx_err)?,
        projection_version: row.try_get("projection_version").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
