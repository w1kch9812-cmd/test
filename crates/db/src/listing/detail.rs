use listing_domain::repository::{ListingDetail, ListingPhotoSummary, RepoError};
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use sqlx::{PgPool, Row};

use super::rows::{row_to_listing, LISTING_FULL_COLUMNS_WITH_L_ALIAS};
use crate::error_map::map_sqlx_err;

pub(super) async fn find_detail_by_id(
    pool: &PgPool,
    id: &Id<ListingIdMarker>,
    viewer_user_id: &Id<UserMarker>,
) -> Result<Option<ListingDetail>, RepoError> {
    let detail_sql = format!(
        r"
        SELECT {LISTING_FULL_COLUMNS_WITH_L_ALIAS},
               COALESCE(bm.cnt, 0)::int8 AS jc_bookmark_count,
               CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END AS jc_is_bookmarked
        FROM listing l
        LEFT JOIN (
            SELECT listing_id, COUNT(*)::int8 AS cnt
            FROM bookmark_listing
            WHERE listing_id = $1
            GROUP BY listing_id
        ) bm ON bm.listing_id = l.id
        LEFT JOIN bookmark_listing ub
          ON ub.listing_id = l.id AND ub.user_id = $2
        WHERE l.id = $1
        "
    );
    let row_opt = sqlx::query(&detail_sql)
        .bind(id.as_str())
        .bind(viewer_user_id.as_str())
        .fetch_optional(pool)
        .await
        .map_err(map_sqlx_err)?;
    let Some(row) = row_opt else {
        return Ok(None);
    };

    let listing = row_to_listing(&row)?;
    let bookmark_count: i64 = row.try_get("jc_bookmark_count").unwrap_or(0_i64);
    let is_bookmarked: bool = row.try_get("jc_is_bookmarked").unwrap_or(false);

    let photo_rows = sqlx::query(
        r"
        SELECT id, r2_key, thumbnail_r2_key, caption, display_order, content_type
        FROM listing_photo
        WHERE listing_id = $1 AND deleted_at IS NULL AND file_size_bytes IS NOT NULL
        ORDER BY display_order ASC
        ",
    )
    .bind(id.as_str())
    .fetch_all(pool)
    .await
    .map_err(map_sqlx_err)?;

    let photos: Vec<ListingPhotoSummary> = photo_rows
        .iter()
        .map(|r| {
            Ok::<_, RepoError>(ListingPhotoSummary {
                photo_id: r.try_get("id").map_err(map_sqlx_err)?,
                r2_key: r.try_get("r2_key").map_err(map_sqlx_err)?,
                thumbnail_r2_key: r.try_get("thumbnail_r2_key").map_err(map_sqlx_err)?,
                caption: r.try_get("caption").map_err(map_sqlx_err)?,
                display_order: r.try_get("display_order").map_err(map_sqlx_err)?,
                content_type: r.try_get("content_type").map_err(map_sqlx_err)?,
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(Some(ListingDetail {
        listing,
        photos,
        bookmark_count,
        is_bookmarked,
    }))
}
