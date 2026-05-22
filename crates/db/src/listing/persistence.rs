#![allow(clippy::needless_pass_by_value)]

use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::Utc;
use listing_domain::entity::Listing;
use listing_domain::repository::RepoError;
use serde_json::Value;
use shared_kernel::id::{AuditLogMarker, Id, OutboxEventMarker};
use shared_kernel::money::MoneyKrw;
use shared_kernel::mutation::MutationContext;
use sqlx::{PgPool, Postgres, Transaction};

use crate::error_map::map_sqlx_err;

pub(super) async fn save(
    pool: &PgPool,
    listing: &Listing,
    ctx: MutationContext,
) -> Result<(), RepoError> {
    let area_decimal = listing_area_decimal(listing)?;
    let view_count_i64 = i64::try_from(listing.view_count).unwrap_or(i64::MAX);
    let bookmark_count_i64 = i64::try_from(listing.bookmark_count).unwrap_or(i64::MAX);

    let mut tx = pool.begin().await.map_err(map_sqlx_err)?;
    let before_state = crate::audit_state::read_listing_json(&mut tx, &listing.id).await?;

    upsert_listing(
        &mut tx,
        listing,
        &area_decimal,
        view_count_i64,
        bookmark_count_i64,
    )
    .await?;

    let after_state_raw = crate::audit_state::read_listing_json(&mut tx, &listing.id).await?;
    let after_state = crate::audit_state::merge_metadata(after_state_raw, ctx.metadata.as_ref());

    insert_audit_log(
        &mut tx,
        listing,
        &ctx,
        before_state.as_ref(),
        after_state.as_ref(),
    )
    .await?;
    insert_outbox_events(&mut tx, listing, &ctx).await?;

    tx.commit().await.map_err(map_sqlx_err)?;
    Ok(())
}

fn listing_area_decimal(listing: &Listing) -> Result<BigDecimal, RepoError> {
    let area_str = format!("{:.2}", listing.area.as_f64());
    BigDecimal::from_str(&area_str)
        .map_err(|e| RepoError::Database(format!("invalid area_m2 conversion: {e}")))
}

async fn upsert_listing(
    tx: &mut Transaction<'_, Postgres>,
    listing: &Listing,
    area_decimal: &BigDecimal,
    view_count_i64: i64,
    bookmark_count_i64: i64,
) -> Result<(), RepoError> {
    let result = sqlx::query(
        r"
        insert into listing (
            id, owner_id, parcel_pnu, listing_type, transaction_type,
            price_krw, deposit_krw, monthly_rent_krw, area_m2,
            title, description, status, contact_visibility,
            view_count, bookmark_count, created_at, updated_at, expires_at, version
        )
        values (
            $1, $2, $3, $4, $5,
            $6, $7, $8, $9,
            $10, $11, $12, $13,
            $14, $15,
            $16, $17, $18, $19
        )
        on conflict (id) do update set
            listing_type = excluded.listing_type,
            transaction_type = excluded.transaction_type,
            price_krw = excluded.price_krw,
            deposit_krw = excluded.deposit_krw,
            monthly_rent_krw = excluded.monthly_rent_krw,
            area_m2 = excluded.area_m2,
            title = excluded.title,
            description = excluded.description,
            status = excluded.status,
            contact_visibility = excluded.contact_visibility,
            view_count = excluded.view_count,
            bookmark_count = excluded.bookmark_count,
            updated_at = excluded.updated_at,
            expires_at = excluded.expires_at,
            version = excluded.version
        where listing.version = $19 - 1
        ",
    )
    .bind(listing.id.as_str())
    .bind(listing.owner_id.as_str())
    .bind(listing.parcel_pnu.as_str())
    .bind(listing.listing_type.as_str())
    .bind(listing.transaction_type.as_str())
    .bind(listing.price.as_i64())
    .bind(listing.deposit.map(MoneyKrw::as_i64))
    .bind(listing.monthly_rent.map(MoneyKrw::as_i64))
    .bind(area_decimal)
    .bind(listing.title.as_str())
    .bind(listing.description.as_str())
    .bind(listing.status.as_str())
    .bind(listing.contact_visibility.as_str())
    .bind(view_count_i64)
    .bind(bookmark_count_i64)
    .bind(listing.created_at)
    .bind(listing.updated_at)
    .bind(listing.expires_at)
    .bind(listing.version)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;

    if result.rows_affected() == 0 {
        return Err(RepoError::Conflict);
    }
    Ok(())
}

async fn insert_audit_log(
    tx: &mut Transaction<'_, Postgres>,
    listing: &Listing,
    ctx: &MutationContext,
    before_state: Option<&Value>,
    after_state: Option<&Value>,
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
        values ($1, $2, $3, 'listing', $4, $5, $6, $7::inet, $8, $9, $10)
        ",
    )
    .bind(audit_id.as_str())
    .bind(ctx.actor_id.as_ref().map(Id::as_str))
    .bind(&ctx.action)
    .bind(listing.id.as_str())
    .bind(before_state)
    .bind(after_state)
    .bind(ctx.client_ip.as_deref())
    .bind(ctx.user_agent.as_deref())
    .bind(&ctx.correlation_id)
    .bind(occurred_at)
    .execute(&mut **tx)
    .await
    .map_err(map_sqlx_err)?;
    Ok(())
}

async fn insert_outbox_events(
    tx: &mut Transaction<'_, Postgres>,
    listing: &Listing,
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
            values ($1, 'listing', $2, $3, $4, $5, $6, NULL)
            ",
        )
        .bind(outbox_id.as_str())
        .bind(listing.id.as_str())
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
