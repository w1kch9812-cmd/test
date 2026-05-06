//! `PgListingPhotoRepository` — `ListingPhotoRepository` `Postgres` 구현체
//! (spec § 5.1, 12 필드 + soft-delete + `ON DELETE CASCADE` from `listing`).
//!
//! `find_by_listing` 은 `deleted_at is null` 필터 + `display_order asc` 정렬.
//! `save` 는 upsert (`on conflict (id) do update`). `delete` 는 hard delete —
//! 일반 흐름은 `soft_delete` 후 별도 archive job, 본 메서드는 관리/테스트용.
//!
//! SP5-iv: `save` 와 `delete` 모두 트랜잭션 안에서 `audit_log` + `outbox_event`
//! 를 함께 기록 — `MutationContext` 패턴 (`PgAdminActionRepository` 와 동일).
//! hard delete 도 audit 대상 (`action` 은 caller 가 명시 — `"delete"` 권장).

// `PgListingPhotoRepository` 처럼 모듈명 반복은 의도된 공개 API 형태.
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use listing_photo_domain::entity::{ListingPhoto, PhotoContentType};
use listing_photo_domain::repository::{ListingPhotoRepository, RepoError};
use shared_kernel::id::{AuditLogMarker, Id, ListingMarker, ListingPhotoMarker, OutboxEventMarker};
use shared_kernel::mutation::MutationContext;
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

    /// 트랜잭션 안에서 `listing_photo` UPSERT + `audit_log` + `outbox_event` 를 함께 기록.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, photo, ctx), fields(
        photo_id = %photo.id.as_str(),
        order = photo.display_order,
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save(&self, photo: &ListingPhoto, ctx: MutationContext) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state snapshot (None if INSERT — 새 row).
        let before_state = crate::audit_state::read_listing_photo_json(&mut tx, &photo.id).await?;

        // 1. listing_photo UPSERT.
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
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        // 2a. SP-Obs T4: after_state snapshot + metadata merge.
        let after_state_raw =
            crate::audit_state::read_listing_photo_json(&mut tx, &photo.id).await?;
        let after_state =
            crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

        // 2b. audit_log INSERT — same tx.
        write_audit_log(&mut tx, photo.id.as_str(), &ctx, before_state, after_state).await?;

        // 3. outbox_event INSERT for each ctx.events — same tx.
        write_outbox_events(&mut tx, photo.id.as_str(), &ctx).await?;

        // 4. commit.
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    /// 트랜잭션 안에서 `listing_photo` hard delete + `audit_log` 기록.
    /// `ctx.events` 가 있으면 `outbox_event` 도 같은 tx 에 기록.
    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        photo_id = %id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn delete(
        &self,
        id: &Id<ListingPhotoMarker>,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state snapshot (DELETE 시 audit chain 의 핵심 — row
        // 가 사라지기 전 마지막 상태 보존).
        let before_state = crate::audit_state::read_listing_photo_json(&mut tx, id).await?;

        // 1. DELETE listing_photo.
        let result = sqlx::query("delete from listing_photo where id = $1")
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        // 2a. SP-Obs T4: after_state = None (row 가 더 이상 없음). metadata 만 wrap.
        let after_state = crate::audit_state::merge_metadata(None, ctx.metadata.as_ref());

        // 2b. audit_log INSERT — same tx.
        write_audit_log(&mut tx, id.as_str(), &ctx, before_state, after_state).await?;

        // 3. outbox_event INSERT for each ctx.events — same tx.
        write_outbox_events(&mut tx, id.as_str(), &ctx).await?;

        // 4. commit.
        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}

/// `audit_log` 1 row INSERT — `resource_kind = 'listing_photo'`.
///
/// SP-Obs T4: caller 가 `before_state` (snapshot before mutation) +
/// `after_state` (after, with `__metadata__` merged) 를 만들어 전달.
async fn write_audit_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    photo_id: &str,
    ctx: &MutationContext,
    before_state: Option<serde_json::Value>,
    after_state: Option<serde_json::Value>,
) -> Result<(), RepoError> {
    let audit_id = Id::<AuditLogMarker>::new();
    let occurred_at = ctx.occurred_at.unwrap_or_else(Utc::now);
    sqlx::query(
        r"
        insert into audit_log (
            id, actor_id, action, resource_kind, resource_id,
            before_state, after_state,
            ip_address, user_agent,
            correlation_id, created_at
        )
        values ($1, $2, $3, 'listing_photo', $4, $5, $6, $7::inet, $8, $9, $10)
        ",
    )
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(photo_id)
    .bind(&before_state)
    .bind(&after_state)
    .bind(ctx.client_ip.as_deref())
    .bind(ctx.user_agent.as_deref())
    .bind(&ctx.correlation_id)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    Ok(())
}

/// `outbox_event` row INSERT — `aggregate_kind = 'listing_photo'`, `ctx.events` 마다 1 row.
async fn write_outbox_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    photo_id: &str,
    ctx: &MutationContext,
) -> Result<(), RepoError> {
    for event in &ctx.events {
        let outbox_id = Id::<OutboxEventMarker>::new();
        sqlx::query(
            r"
            insert into outbox_event (
                id, aggregate_kind, aggregate_id, event_type, payload,
                correlation_id, created_at, published_at
            )
            values ($1, 'listing_photo', $2, $3, $4, $5, $6, NULL)
            ",
        )
        .bind(outbox_id.as_str())
        .bind(photo_id)
        .bind(event.event_type())
        .bind(event.payload())
        .bind(&ctx.correlation_id)
        .bind(event.occurred_at())
        .execute(&mut **tx)
        .await
        .map_err(map_sqlx_err)?;
    }
    Ok(())
}
