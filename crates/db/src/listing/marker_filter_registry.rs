use listing_domain::repository::{
    ListingMarkerFilterSpec, ListingMarkerRegisteredFilter, NormalizedListingMarkerFilterSpec,
    RepoError,
};
use serde_json::{json, Value};
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;
use sqlx::{PgPool, Row};

use crate::error_map::map_sqlx_err;

pub(super) async fn register_listing_marker_filter(
    pool: &PgPool,
    filter: NormalizedListingMarkerFilterSpec,
) -> Result<ListingMarkerRegisteredFilter, RepoError> {
    let filter_hash = filter.filter_hash();
    let spec = filter_to_json(&filter);

    sqlx::query(
        r"
        insert into listing_marker_filter_registry (
            filter_hash,
            spec,
            created_at,
            last_used_at,
            request_count
        )
        values ($1, $2, now(), now(), 1)
        on conflict (filter_hash) do update set
            spec = excluded.spec,
            last_used_at = now(),
            request_count = listing_marker_filter_registry.request_count + 1
        ",
    )
    .bind(&filter_hash)
    .bind(spec)
    .execute(pool)
    .await
    .map_err(map_sqlx_err)?;

    Ok(ListingMarkerRegisteredFilter { filter_hash })
}

pub(super) async fn resolve_listing_marker_filter(
    pool: &PgPool,
    filter_hash: &str,
) -> Result<Option<NormalizedListingMarkerFilterSpec>, RepoError> {
    let row = sqlx::query(
        r"
        update listing_marker_filter_registry
        set last_used_at = now(),
            request_count = request_count + 1
        where filter_hash = $1
        returning spec
        ",
    )
    .bind(filter_hash)
    .fetch_optional(pool)
    .await
    .map_err(map_sqlx_err)?;

    row.map(|row| {
        let spec: Value = row.try_get("spec").map_err(map_sqlx_err)?;
        json_to_filter(spec)
    })
    .transpose()
}

fn filter_to_json(filter: &NormalizedListingMarkerFilterSpec) -> Value {
    json!({
        "types": filter.types.iter().map(|value| value.as_str()).collect::<Vec<_>>(),
        "transactions": filter.transactions.iter().map(|value| value.as_str()).collect::<Vec<_>>(),
        "min_area_m2": filter.min_area_m2,
        "max_area_m2": filter.max_area_m2,
        "min_price_krw": filter.min_price_krw,
        "max_price_krw": filter.max_price_krw,
    })
}

fn json_to_filter(value: Value) -> Result<NormalizedListingMarkerFilterSpec, RepoError> {
    let types = read_string_array(&value, "types")?
        .into_iter()
        .map(|raw| {
            raw.parse::<ListingType>()
                .map_err(|e| RepoError::Database(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let transactions = read_string_array(&value, "transactions")?
        .into_iter()
        .map(|raw| {
            raw.parse::<TransactionType>()
                .map_err(|e| RepoError::Database(e.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    ListingMarkerFilterSpec {
        types,
        transactions,
        min_area_m2: read_i64_or_null(&value, "min_area_m2")?,
        max_area_m2: read_i64_or_null(&value, "max_area_m2")?,
        min_price_krw: read_i64_or_null(&value, "min_price_krw")?,
        max_price_krw: read_i64_or_null(&value, "max_price_krw")?,
    }
    .try_normalized()
    .map_err(|e| RepoError::Database(e.to_string()))
}

fn read_string_array(value: &Value, key: &'static str) -> Result<Vec<String>, RepoError> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| RepoError::Database(format!("invalid marker filter registry field: {key}")))?
        .iter()
        .map(|item| {
            item.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                RepoError::Database(format!("invalid marker filter registry string: {key}"))
            })
        })
        .collect()
}

fn read_i64_or_null(value: &Value, key: &'static str) -> Result<Option<i64>, RepoError> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(raw) => raw.as_i64().map(Some).ok_or_else(|| {
            RepoError::Database(format!("invalid marker filter registry number: {key}"))
        }),
        None => Err(RepoError::Database(format!(
            "missing marker filter registry field: {key}"
        ))),
    }
}
