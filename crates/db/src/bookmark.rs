//! `BookmarkRepository` `Postgres` 구현체 (SP5-ii).
//!
//! 두 Aggregate 처리:
//! - `BookmarkListing` — composite PK `(user_id, listing_id)`. UPSERT.
//! - `BookmarkExternal` — single PK `id`. UNIQUE `(user_id, target_kind, target_id)`.
//!
//! 모든 mutation 은 SP5-iv 의 transactional `audit_log` + `outbox_event` 패턴 따라요.

#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use bookmark_domain::external::BookmarkExternal;
use bookmark_domain::external_kind::BookmarkExternalKind;
use bookmark_domain::listing::BookmarkListing;
use bookmark_domain::repository::{BookmarkRepository, RepoError};
use chrono::{DateTime, Utc};
use shared_kernel::id::{
    AuditLogMarker, BookmarkExternalMarker, Id, ListingMarker, OutboxEventMarker, UserMarker,
};
use shared_kernel::mutation::MutationContext;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use tracing::instrument;

use crate::error_map::map_sqlx_err;

/// `Bookmark` Aggregate 의 `Postgres` 저장소.
#[derive(Debug, Clone)]
pub struct PgBookmarkRepository {
    pool: PgPool,
}

impl PgBookmarkRepository {
    /// 새 저장소를 만들어요.
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

const LISTING_COLUMNS: &str = "user_id, listing_id, note, created_at";
const EXTERNAL_COLUMNS: &str = "id, user_id, target_kind, target_id, note, created_at";

fn row_to_listing_bookmark(row: &PgRow) -> Result<BookmarkListing, RepoError> {
    let user_id_str: String = row
        .try_get("user_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let listing_id_str: String = row
        .try_get("listing_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let note: Option<String> = row
        .try_get("note")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let user_id = Id::<UserMarker>::try_from_str(user_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed user_id in DB: {e}")))?;
    let listing_id = Id::<ListingMarker>::try_from_str(listing_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed listing_id in DB: {e}")))?;

    Ok(BookmarkListing {
        user_id,
        listing_id,
        note,
        created_at,
    })
}

fn row_to_external_bookmark(row: &PgRow) -> Result<BookmarkExternal, RepoError> {
    let id_str: String = row
        .try_get("id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let user_id_str: String = row
        .try_get("user_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_kind_str: String = row
        .try_get("target_kind")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let target_id: String = row
        .try_get("target_id")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let note: Option<String> = row
        .try_get("note")
        .map_err(|e| RepoError::Database(e.to_string()))?;
    let created_at: DateTime<Utc> = row
        .try_get("created_at")
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let id = Id::<BookmarkExternalMarker>::try_from_str(id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed bookmark_external id in DB: {e}")))?;
    let user_id = Id::<UserMarker>::try_from_str(user_id_str.trim())
        .map_err(|e| RepoError::Database(format!("malformed user_id in DB: {e}")))?;
    let target_kind = match target_kind_str.as_str() {
        "parcel" => BookmarkExternalKind::Parcel,
        "court_auction" => BookmarkExternalKind::CourtAuction,
        "manufacturer" => BookmarkExternalKind::Manufacturer,
        "industrial_complex" => BookmarkExternalKind::IndustrialComplex,
        other => {
            return Err(RepoError::Database(format!(
                "unexpected bookmark_external target_kind in DB: {other}"
            )));
        }
    };

    Ok(BookmarkExternal {
        id,
        user_id,
        target_kind,
        target_id,
        note,
        created_at,
    })
}

#[async_trait]
impl BookmarkRepository for PgBookmarkRepository {
    #[instrument(skip(self), fields(user_id = %user_id.as_str()))]
    async fn find_listing_bookmarks(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<BookmarkListing>, RepoError> {
        let sql = format!(
            "select {LISTING_COLUMNS} from bookmark_listing \
             where user_id = $1 \
             order by created_at desc"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_listing_bookmark).collect()
    }

    #[instrument(skip(self), fields(user_id = %user_id.as_str()))]
    async fn find_external_bookmarks(
        &self,
        user_id: &Id<UserMarker>,
    ) -> Result<Vec<BookmarkExternal>, RepoError> {
        let sql = format!(
            "select {EXTERNAL_COLUMNS} from bookmark_external \
             where user_id = $1 \
             order by created_at desc"
        );
        let rows = sqlx::query(&sql)
            .bind(user_id.as_str())
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx_err)?;
        rows.iter().map(row_to_external_bookmark).collect()
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, bm, ctx), fields(
        user_id = %bm.user_id.as_str(),
        listing_id = %bm.listing_id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_listing_bookmark(
        &self,
        bm: &BookmarkListing,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state (composite PK).
        let before_state = crate::audit_state::read_bookmark_listing_json(
            &mut tx,
            &bm.user_id,
            &bm.listing_id,
        )
        .await?;

        sqlx::query(
            r"
            insert into bookmark_listing (user_id, listing_id, note, created_at)
            values ($1, $2, $3, $4)
            on conflict (user_id, listing_id) do update set note = excluded.note
            ",
        )
        .bind(bm.user_id.as_str())
        .bind(bm.listing_id.as_str())
        .bind(bm.note.as_deref())
        .bind(bm.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let after_state_raw = crate::audit_state::read_bookmark_listing_json(
            &mut tx,
            &bm.user_id,
            &bm.listing_id,
        )
        .await?;
        let after_state =
            crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

        write_audit_log(
            &mut tx,
            "bookmark_listing",
            bm.listing_id.as_str(),
            &ctx,
            before_state,
            after_state,
        )
        .await?;
        write_outbox_events(&mut tx, "bookmark_listing", bm.listing_id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, bm, ctx), fields(
        bookmark_id = %bm.id.as_str(),
        target_kind = %bm.target_kind.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
        events_count = ctx.events.len(),
    ))]
    async fn save_external_bookmark(
        &self,
        bm: &BookmarkExternal,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // 0. SP-Obs T4: before_state.
        let before_state =
            crate::audit_state::read_bookmark_external_json(&mut tx, &bm.id).await?;

        sqlx::query(
            r"
            insert into bookmark_external (
                id, user_id, target_kind, target_id, note, created_at
            )
            values ($1, $2, $3, $4, $5, $6)
            on conflict (id) do update set
                target_kind = excluded.target_kind,
                target_id = excluded.target_id,
                note = excluded.note
            ",
        )
        .bind(bm.id.as_str())
        .bind(bm.user_id.as_str())
        .bind(bm.target_kind.as_str())
        .bind(&bm.target_id)
        .bind(bm.note.as_deref())
        .bind(bm.created_at)
        .execute(&mut *tx)
        .await
        .map_err(map_sqlx_err)?;

        let after_state_raw =
            crate::audit_state::read_bookmark_external_json(&mut tx, &bm.id).await?;
        let after_state =
            crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

        write_audit_log(
            &mut tx,
            "bookmark_external",
            bm.id.as_str(),
            &ctx,
            before_state,
            after_state,
        )
        .await?;
        write_outbox_events(&mut tx, "bookmark_external", bm.id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        user_id = %user_id.as_str(),
        listing_id = %listing_id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn delete_listing_bookmark(
        &self,
        user_id: &Id<UserMarker>,
        listing_id: &Id<ListingMarker>,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        // SP-Obs T4: before_state — DELETE 직전 row 마지막 상태 보존.
        let before_state =
            crate::audit_state::read_bookmark_listing_json(&mut tx, user_id, listing_id).await?;

        let result =
            sqlx::query("delete from bookmark_listing where user_id = $1 and listing_id = $2")
                .bind(user_id.as_str())
                .bind(listing_id.as_str())
                .execute(&mut *tx)
                .await
                .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        let after_state = crate::audit_state::merge_metadata(None, ctx.metadata.as_ref());

        write_audit_log(
            &mut tx,
            "bookmark_listing",
            listing_id.as_str(),
            &ctx,
            before_state,
            after_state,
        )
        .await?;
        write_outbox_events(&mut tx, "bookmark_listing", listing_id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value)]
    #[instrument(skip(self, ctx), fields(
        bookmark_id = %id.as_str(),
        ctx_action = %ctx.action,
        correlation_id = %ctx.correlation_id,
    ))]
    async fn delete_external_bookmark(
        &self,
        id: &Id<BookmarkExternalMarker>,
        ctx: MutationContext,
    ) -> Result<(), RepoError> {
        let mut tx = self.pool.begin().await.map_err(map_sqlx_err)?;

        let before_state =
            crate::audit_state::read_bookmark_external_json(&mut tx, id).await?;

        let result = sqlx::query("delete from bookmark_external where id = $1")
            .bind(id.as_str())
            .execute(&mut *tx)
            .await
            .map_err(map_sqlx_err)?;
        if result.rows_affected() == 0 {
            return Err(RepoError::NotFound);
        }

        let after_state = crate::audit_state::merge_metadata(None, ctx.metadata.as_ref());

        write_audit_log(
            &mut tx,
            "bookmark_external",
            id.as_str(),
            &ctx,
            before_state,
            after_state,
        )
        .await?;
        write_outbox_events(&mut tx, "bookmark_external", id.as_str(), &ctx).await?;

        tx.commit().await.map_err(map_sqlx_err)?;
        Ok(())
    }
}

/// `audit_log` 1 row INSERT — Bookmark transactional 패턴 (SP-Obs T4 갱신).
#[allow(clippy::too_many_arguments)] // 7 = SP-Obs T4 audit chain 풀필드 (resource_kind/id 분리 + before/after).
async fn write_audit_log(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    resource_kind: &str,
    resource_id: &str,
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
        values ($1, $2, $3, $4, $5, $6, $7, $8::inet, $9, $10, $11)
        ",
    )
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(resource_kind)
    .bind(resource_id)
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

/// `outbox_event` row INSERT — Bookmark transactional 패턴.
async fn write_outbox_events(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    aggregate_kind: &str,
    aggregate_id: &str,
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
            values ($1, $2, $3, $4, $5, $6, $7, NULL)
            ",
        )
        .bind(outbox_id.as_str())
        .bind(aggregate_kind)
        .bind(aggregate_id)
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
