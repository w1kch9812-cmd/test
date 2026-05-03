//! `PgListingPhotoRepository` — `ListingPhotoRepository` `Postgres` 구현체
//! (spec § 5.1, 12 필드 + soft-delete + `ON DELETE CASCADE` from `listing`).
//!
//! `find_by_listing` 은 `deleted_at is null` 필터 + `display_order asc` 정렬.
//! `save` 는 upsert (`on conflict (id) do update`). `delete` 는 hard delete —
//! 일반 흐름은 `soft_delete` 후 별도 archive job, 본 메서드는 관리/테스트용.

// `PgListingPhotoRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::id::{Id, ListingMarker, ListingPhotoMarker};
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `ListingPhoto` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgListingPhotoRepository {
    pool: PgPool,
}

impl PgListingPhotoRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

/// `select` 절에서 모든 `listing_photo` 컬럼을 일관되게 가져오기 위한 상수.
const PHOTO_COLUMNS: &str = "id, listing_id, r2_key, thumbnail_r2_key, caption, \
    display_order, width_px, height_px, file_size_bytes, \
    content_type, uploaded_at, deleted_at";

fn parse_content_type(s: &str) -> Result<PhotoContentType, RepoError> {
    match s {
        "image/jpeg" => Ok(PhotoContentType::Jpeg),
        "image/png" => Ok(PhotoContentType::Png),
        "image/webp" => Ok(PhotoContentType::Webp),
        other => Err(RepoError::Database(format!(
            "unexpected content_type in DB: {other}"
        ))),
    }
}

/// `PgRow` 를 `ListingPhoto` 로 변환해요. 12 필드 모두 round-trip.
fn row_to_photo(row: &PgRow) -> Result<ListingPhoto, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_id_str: String = row
        .try_get("listing_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let r2_key: String = row
        .try_get("r2_key")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let thumbnail_r2_key: Option<String> = row
        .try_get("thumbnail_r2_key")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let caption: Option<String> = row
        .try_get("caption")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let display_order: i32 = row
        .try_get("display_order")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let width_px: Option<i32> = row
        .try_get("width_px")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let height_px: Option<i32> = row
        .try_get("height_px")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let file_size_bytes: Option<i64> = row
        .try_get("file_size_bytes")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let content_type_str: String = row
        .try_get("content_type")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let uploaded_at: DateTime<Utc> = row
        .try_get("uploaded_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let deleted_at: Option<DateTime<Utc>> = row
        .try_get("deleted_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<ListingPhotoMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("malformed photo id in DB: {e}")))?;
    let listing_id = Id::<ListingMarker>::try_from_str(&listing_id_str)
        .map_err(|e| RepoError::Database(format!("malformed listing_id in DB: {e}")))?;
    let content_type = parse_content_type(&content_type_str)?;

    Ok(ListingPhoto {
        id,
        listing_id,
        r2_key,
        thumbnail_r2_key,
        caption,
        display_order,
        width_px,
        height_px,
        file_size_bytes,
        content_type,
        uploaded_at,
        deleted_at,
    })
}

#[async_trait]
impl ListingPhotoRepository for PgListingPhotoRepository {
    #[instrument(skip(self), fields(listing_id = %listing_id.as_str()))]
    async fn find_by_listing(
        &self,
        listing_id: &Id<ListingMarker>,
    ) -> Result<Vec<ListingPhoto>, RepoError> {
        let sql = format!(
            "select {PHOTO_COLUMNS} from listing_photo \
             where listing_id = $1 and deleted_at is null \
             order by display_order asc"
        );
        let rows = sqlx::query(&sql)
            .bind(listing_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_photo).collect()
    }

    #[instrument(skip(self, photo), fields(photo_id = %photo.id.as_str(), order = photo.display_order))]
    async fn save(&self, photo: &ListingPhoto) -> Result<(), RepoError> {
        sqlx::query(
            r"
            insert into listing_photo (
                id, listing_id, r2_key, thumbnail_r2_key, caption,
                display_order, width_px, height_px, file_size_bytes,
                content_type, uploaded_at, deleted_at
            )
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            on conflict (id) do update set
                r2_key = excluded.r2_key,
                thumbnail_r2_key = excluded.thumbnail_r2_key,
                caption = excluded.caption,
                display_order = excluded.display_order,
                width_px = excluded.width_px,
                height_px = excluded.height_px,
                file_size_bytes = excluded.file_size_bytes,
                content_type = excluded.content_type,
                deleted_at = excluded.deleted_at
            ",
        )
        .bind(photo.id.as_str())
        .bind(photo.listing_id.as_str())
        .bind(&photo.r2_key)
        .bind(&photo.thumbnail_r2_key)
        .bind(&photo.caption)
        .bind(photo.display_order)
        .bind(photo.width_px)
        .bind(photo.height_px)
        .bind(photo.file_size_bytes)
        .bind(photo.content_type.as_str())
        .bind(photo.uploaded_at)
        .bind(photo.deleted_at)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx_err)?;
        Ok(())
    }

    #[instrument(skip(self), fields(photo_id = %id.as_str()))]
    async fn delete(&self, id: &Id<ListingPhotoMarker>) -> Result<(), RepoError> {
        let result = sqlx::query("delete from listing_photo where id = $1")
            .bind(id.as_str())
            .execute(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }
        Ok(())
    }
}
