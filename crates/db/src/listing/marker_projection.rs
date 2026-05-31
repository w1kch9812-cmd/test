use listing_domain::repository::RepoError;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use sqlx::{Executor, PgPool, Postgres, Row, Transaction};

use crate::error_map::map_sqlx_err;

pub(super) async fn upsert_listing_marker_projection(
    pool: &PgPool,
    id: &Id<ListingIdMarker>,
) -> Result<(), RepoError> {
    sync_listing_marker_projection(pool, id, true, true).await
}

pub(super) async fn sync_listing_marker_projection_after_save(
    tx: &mut Transaction<'_, Postgres>,
    id: &Id<ListingIdMarker>,
) -> Result<(), RepoError> {
    sync_listing_marker_projection(&mut **tx, id, false, false).await
}

#[expect(
    clippy::too_many_lines,
    reason = "projection SQL and write decision stay together to preserve one transactional read/write plan"
)]
async fn sync_listing_marker_projection<'e, E>(
    executor: E,
    id: &Id<ListingIdMarker>,
    include_inactive_without_existing_projection: bool,
    require_anchor_for_any_status: bool,
) -> Result<(), RepoError>
where
    E: Executor<'e, Database = Postgres>,
{
    let row = sqlx::query(
        r"
        with candidate as (
            select
                l.id,
                l.parcel_pnu,
                l.status,
                l.listing_type,
                l.transaction_type,
                l.price_krw,
                l.area_m2,
                l.updated_at,
                l.version,
                a.anchor_point,
                a.anchor_snapshot_id,
                a.source_geometry_version,
                a.source_geometry_checksum_sha256,
                existing.listing_id is not null as had_existing,
                existing.listing_status as old_listing_status,
                existing.visibility_scope as old_visibility_scope
            from listing l
            left join listing_marker_projection existing on existing.listing_id = l.id
            inner join parcel_marker_anchor a on a.pnu = l.parcel_pnu
            where l.id = $1
              and (
                  $2::boolean
                  or l.status = 'active'
                  or existing.listing_id is not null
              )
        ),
        projected as (
            select
                'lm_' || id as marker_id,
                id as listing_id,
                parcel_pnu as pnu,
                anchor_point,
                anchor_snapshot_id,
                source_geometry_version,
                source_geometry_checksum_sha256,
                version as source_listing_version,
                least(
                    16383,
                    greatest(0, floor(((ST_X(anchor_point) + 180.0) / 360.0) * 16384.0)::integer)
                ) as z14_tile_x,
                least(
                    16383,
                    greatest(
                        0,
                        floor(
                            (
                                (
                                    1.0 - (
                                        ln(
                                            tan(radians(ST_Y(anchor_point)))
                                            + (1.0 / cos(radians(ST_Y(anchor_point))))
                                        ) / pi()
                                    )
                                ) / 2.0
                            ) * 16384.0
                        )::integer
                    )
                ) as z14_tile_y,
                status as listing_status,
                case when status = 'active' then 'public' else 'owner_private' end as visibility_scope,
                listing_type,
                transaction_type,
                price_krw,
                area_m2,
                updated_at as listing_updated_at,
                had_existing,
                old_listing_status,
                old_visibility_scope
            from candidate
        ),
        upserted as (
            insert into listing_marker_projection (
                marker_id,
                listing_id,
                pnu,
                anchor_point,
                anchor_snapshot_id,
                source_geometry_version,
                source_geometry_checksum_sha256,
                source_listing_version,
                projection_version,
                z14_tile_x,
                z14_tile_y,
                listing_status,
                visibility_scope,
                listing_type,
                transaction_type,
                price_krw,
                area_m2,
                rank_score,
                listing_updated_at,
                updated_at
            )
            select
                marker_id,
                listing_id,
                pnu,
                anchor_point,
                anchor_snapshot_id,
                source_geometry_version,
                source_geometry_checksum_sha256,
                source_listing_version,
                1,
                z14_tile_x,
                z14_tile_y,
                listing_status,
                visibility_scope,
                listing_type,
                transaction_type,
                price_krw,
                area_m2,
                0,
                listing_updated_at,
                now()
            from projected
            on conflict (listing_id) do update set
                marker_id = excluded.marker_id,
                pnu = excluded.pnu,
                anchor_point = excluded.anchor_point,
                anchor_snapshot_id = excluded.anchor_snapshot_id,
                source_geometry_version = excluded.source_geometry_version,
                source_geometry_checksum_sha256 = excluded.source_geometry_checksum_sha256,
                source_listing_version = excluded.source_listing_version,
                projection_version = listing_marker_projection.projection_version + 1,
                z14_tile_x = excluded.z14_tile_x,
                z14_tile_y = excluded.z14_tile_y,
                listing_status = excluded.listing_status,
                visibility_scope = excluded.visibility_scope,
                listing_type = excluded.listing_type,
                transaction_type = excluded.transaction_type,
                price_krw = excluded.price_krw,
                area_m2 = excluded.area_m2,
                rank_score = excluded.rank_score,
                listing_updated_at = excluded.listing_updated_at,
                updated_at = now()
            where listing_marker_projection.pnu is distinct from excluded.pnu
               or not ST_Equals(listing_marker_projection.anchor_point, excluded.anchor_point)
               or listing_marker_projection.anchor_snapshot_id is distinct from excluded.anchor_snapshot_id
               or listing_marker_projection.source_geometry_version is distinct from excluded.source_geometry_version
               or listing_marker_projection.source_geometry_checksum_sha256 is distinct from excluded.source_geometry_checksum_sha256
               or listing_marker_projection.source_listing_version is distinct from excluded.source_listing_version
               or listing_marker_projection.z14_tile_x is distinct from excluded.z14_tile_x
               or listing_marker_projection.z14_tile_y is distinct from excluded.z14_tile_y
               or listing_marker_projection.listing_status is distinct from excluded.listing_status
               or listing_marker_projection.visibility_scope is distinct from excluded.visibility_scope
               or listing_marker_projection.listing_type is distinct from excluded.listing_type
               or listing_marker_projection.transaction_type is distinct from excluded.transaction_type
               or listing_marker_projection.price_krw is distinct from excluded.price_krw
               or listing_marker_projection.area_m2 is distinct from excluded.area_m2
               or listing_marker_projection.rank_score is distinct from excluded.rank_score
               or listing_marker_projection.listing_updated_at is distinct from excluded.listing_updated_at
            returning
                marker_id,
                listing_id,
                pnu,
                anchor_snapshot_id,
                projection_version,
                z14_tile_x,
                z14_tile_y,
                listing_status,
                visibility_scope
        ),
        changed as (
            select
                u.marker_id,
                u.listing_id,
                u.pnu,
                u.anchor_snapshot_id,
                u.projection_version,
                u.z14_tile_x,
                u.z14_tile_y,
                u.listing_status,
                u.visibility_scope,
                p.had_existing,
                p.old_listing_status,
                p.old_visibility_scope,
                (
                    p.old_listing_status = 'active'
                    and p.old_visibility_scope = 'public'
                ) as old_public,
                (
                    u.listing_status = 'active'
                    and u.visibility_scope = 'public'
                ) as new_public
            from upserted u
            join projected p on p.listing_id = u.listing_id
        ),
        inserted_delta as (
            insert into listing_marker_delta_log (
                marker_id,
                listing_id,
                pnu,
                z14_tile_x,
                z14_tile_y,
                projection_version,
                anchor_snapshot_id,
                change_kind
            )
            select
                marker_id,
                listing_id,
                pnu,
                z14_tile_x,
                z14_tile_y,
                projection_version,
                anchor_snapshot_id,
                case
                    when old_public then 'updated_public'
                    else 'became_public'
                end
            from changed
            where new_public
            on conflict do nothing
            returning 1
        ),
        inserted_tombstone as (
            insert into listing_marker_tombstone_log (
                marker_id,
                listing_id,
                pnu,
                z14_tile_x,
                z14_tile_y,
                projection_version,
                anchor_snapshot_id,
                reason
            )
            select
                marker_id,
                listing_id,
                pnu,
                z14_tile_x,
                z14_tile_y,
                projection_version,
                anchor_snapshot_id,
                case listing_status
                    when 'sold' then 'sold'
                    when 'expired' then 'expired'
                    when 'rejected' then 'rejected'
                    else 'private'
                end
            from changed
            where old_public and not new_public
            on conflict do nothing
            returning 1
        ),
        inserted_dirty_tile as (
            insert into listing_marker_dirty_tile_queue (
                tile_z,
                tile_x,
                tile_y,
                reason,
                priority
            )
            select
                dirty_zoom.tile_z,
                z14_tile_x / (1 << (14 - dirty_zoom.tile_z)),
                z14_tile_y / (1 << (14 - dirty_zoom.tile_z)),
                case when old_public and not new_public then 'tombstone' else 'delta' end,
                case when old_public and not new_public then 10 else 100 end
            from changed
            cross join (
                values (0), (6), (10), (11), (12), (13), (14)
            ) as dirty_zoom(tile_z)
            where old_public or new_public
            on conflict do nothing
            returning 1
        )
        select
            (select count(*)::int8 from candidate) as candidate_count,
            (select count(*)::int8 from inserted_delta) as inserted_delta_count,
            (select count(*)::int8 from inserted_tombstone) as inserted_tombstone_count,
            (select count(*)::int8 from inserted_dirty_tile) as inserted_dirty_tile_count,
            exists(select 1 from listing where id = $1) as listing_exists,
            exists(
                select 1
                from listing l
                where l.id = $1
                  and ($3::boolean or l.status = 'active')
                  and not exists (
                      select 1
                      from parcel_marker_anchor a
                      where a.pnu = l.parcel_pnu
                  )
            ) as required_anchor_missing
        ",
    )
    .bind(id.as_str())
    .bind(include_inactive_without_existing_projection)
    .bind(require_anchor_for_any_status)
    .fetch_one(executor)
    .await
    .map_err(map_sqlx_err)?;

    let candidate_count: i64 = row.try_get("candidate_count").map_err(map_sqlx_err)?;
    if candidate_count > 0 {
        return Ok(());
    }

    let listing_exists: bool = row.try_get("listing_exists").map_err(map_sqlx_err)?;
    if !listing_exists {
        return Err(RepoError::NotFound);
    }

    let required_anchor_missing: bool = row
        .try_get("required_anchor_missing")
        .map_err(map_sqlx_err)?;
    if required_anchor_missing {
        return Err(RepoError::Database(format!(
            "listing marker projection missing PNU anchor: listing_id={}",
            id.as_str()
        )));
    }

    Ok(())
}
