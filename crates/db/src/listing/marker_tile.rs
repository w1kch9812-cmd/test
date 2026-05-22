use listing_domain::repository::{
    ListingMarkerTile, ListingMarkerTileQuery, RepoError, LISTING_MARKER_TILE_LAYER,
};
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn find_listing_marker_tile(
    pool: &PgPool,
    query: ListingMarkerTileQuery,
) -> Result<ListingMarkerTile, RepoError> {
    let row = sqlx::query(
        r"
        WITH unanchored_active AS (
            SELECT COUNT(*)::int8 AS count
            FROM listing l
            LEFT JOIN parcel_marker_anchor a ON a.pnu = l.parcel_pnu
            WHERE l.status = 'active'
              AND a.pnu IS NULL
        ),
        eligible AS (
            SELECT
                l.id,
                l.parcel_pnu AS pnu,
                a.anchor_point,
                a.anchor_snapshot_id,
                row_number() OVER (ORDER BY l.created_at DESC, l.id ASC)::int4 AS rank
            FROM listing l
            INNER JOIN parcel_marker_anchor a ON a.pnu = l.parcel_pnu
            WHERE l.status = 'active'
              AND ST_Intersects(
                  ST_Transform(a.anchor_point, 3857),
                  ST_TileEnvelope($1, $2, $3)
              )
        ),
        features AS (
            SELECT
                id,
                pnu,
                'listing'::text AS kind,
                1::int4 AS count,
                rank,
                id::text AS detail_ref,
                anchor_snapshot_id,
                ST_AsMVTGeom(
                    ST_Transform(anchor_point, 3857),
                    ST_TileEnvelope($1, $2, $3),
                    4096,
                    256,
                    true
                ) AS geom
            FROM eligible
        ),
        represented AS (
            SELECT *
            FROM features
            WHERE geom IS NOT NULL
        ),
        tile AS (
            SELECT ST_AsMVT(represented, $4, 4096, 'geom') AS bytes
            FROM represented
        )
        SELECT
            COALESCE((SELECT bytes FROM tile), '\x'::bytea) AS bytes,
            (SELECT count FROM unanchored_active) AS unanchored_active_count,
            (SELECT COUNT(*)::int8 FROM eligible) AS eligible_count,
            (SELECT COUNT(*)::int8 FROM represented) AS represented_count,
            (SELECT COUNT(*)::int8 FROM represented) AS feature_count,
            0::int8 AS aggregate_count,
            (SELECT max(anchor_snapshot_id) FROM represented) AS anchor_snapshot_id
        ",
    )
    .bind(i32::from(query.z))
    .bind(i32::try_from(query.x).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(i32::try_from(query.y).map_err(|e| RepoError::Database(e.to_string()))?)
    .bind(LISTING_MARKER_TILE_LAYER)
    .fetch_one(pool)
    .await
    .map_err(map_sqlx_err)?;

    let unanchored_active_count: i64 = row
        .try_get("unanchored_active_count")
        .map_err(map_sqlx_err)?;
    if unanchored_active_count > 0 {
        return Err(RepoError::Database(format!(
            "listing marker tile completeness violation: unanchored_active_count={unanchored_active_count}"
        )));
    }

    let eligible_count: i64 = row.try_get("eligible_count").map_err(map_sqlx_err)?;
    let represented_count: i64 = row.try_get("represented_count").map_err(map_sqlx_err)?;
    if eligible_count != represented_count {
        return Err(RepoError::Database(format!(
            "listing marker tile completeness violation: eligible_count={eligible_count}, represented_count={represented_count}"
        )));
    }

    Ok(ListingMarkerTile {
        bytes: row.try_get("bytes").map_err(map_sqlx_err)?,
        layer_name: LISTING_MARKER_TILE_LAYER,
        eligible_count,
        represented_count,
        feature_count: row.try_get("feature_count").map_err(map_sqlx_err)?,
        aggregate_count: row.try_get("aggregate_count").map_err(map_sqlx_err)?,
        anchor_snapshot_id: row.try_get("anchor_snapshot_id").map_err(map_sqlx_err)?,
    })
}
