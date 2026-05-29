use listing_domain::repository::{CardSearchQuery, CardSearchSort, ListingCardSummary, RepoError};
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};

use super::rows::{parse_listing_type, parse_transaction_type};

pub(super) async fn find_card_summaries(
    pool: &PgPool,
    query: CardSearchQuery,
) -> Result<(Vec<ListingCardSummary>, u64), RepoError> {
    let types_array: Option<Vec<&str>> = query
        .types
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().map(|t| t.as_str()).collect());
    let txns_array: Option<Vec<&str>> = query
        .transactions
        .as_ref()
        .filter(|v| !v.is_empty())
        .map(|v| v.iter().map(|t| t.as_str()).collect());

    let min_area = query.min_area_m2.unwrap_or(0.0_f64);
    let max_area = query.max_area_m2.unwrap_or(1e9_f64);
    let min_price = query.min_price_krw.unwrap_or(0_i64);
    let max_price = query.max_price_krw.unwrap_or(i64::MAX);

    let order_by = match query.sort {
        CardSearchSort::CreatedAtDesc => "created_at DESC",
        CardSearchSort::PriceAsc => "price_krw ASC",
        CardSearchSort::PriceDesc => "price_krw DESC",
        CardSearchSort::AreaAsc => "area_m2 ASC",
        CardSearchSort::AreaDesc => "area_m2 DESC",
    };

    let size = query.size;
    let offset = i64::from(query.page) * i64::from(size);

    let pnu_filter: Option<&str> = query.pnu.as_ref().map(Pnu::as_str);
    let admin_prefix_filter: Option<&str> = query.admin_code_prefix.as_deref();
    let land_use_filter: Option<&str> = query.land_use_type.map(LandUseType::as_str);

    let sql = format!(
        r"
        WITH filtered AS (
            SELECT id, title, listing_type, transaction_type,
                   price_krw, deposit_krw, monthly_rent_krw, area_m2,
                   view_count, created_at, owner_id
            FROM listing
            WHERE status = 'active'
              AND ($1::text[] IS NULL OR listing_type = ANY($1::text[]))
              AND ($2::text[] IS NULL OR transaction_type = ANY($2::text[]))
              AND area_m2::float8 BETWEEN $3 AND $4
              AND price_krw BETWEEN $5 AND $6
              AND ($10::text IS NULL OR parcel_pnu = $10)
              AND ($11::text IS NULL OR admin_code LIKE $11 || '%')
              AND ($12::text IS NULL OR parcel_land_use_type = $12)
        ),
        bm_count AS (
            SELECT listing_id, COUNT(*)::int8 AS cnt
            FROM bookmark_listing
            WHERE listing_id IN (SELECT id FROM filtered)
            GROUP BY listing_id
        )
        SELECT
            (SELECT COUNT(*) FROM filtered) AS total_count,
            f.id, f.title,
            f.listing_type, f.transaction_type,
            f.price_krw, f.deposit_krw, f.monthly_rent_krw,
            f.area_m2::float8 AS area_m2,
            f.view_count,
            COALESCE(bc.cnt, 0)::int8 AS bookmark_count,
            CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END AS is_bookmarked,
            f.created_at
        FROM filtered f
        LEFT JOIN bm_count bc ON bc.listing_id = f.id
        LEFT JOIN bookmark_listing ub
          ON ub.listing_id = f.id AND ub.user_id = $9
        ORDER BY f.{order_by}
        LIMIT $7 OFFSET $8
        "
    );

    let rows = sqlx::query(&sql)
        .bind(types_array.as_deref())
        .bind(txns_array.as_deref())
        .bind(min_area)
        .bind(max_area)
        .bind(min_price)
        .bind(max_price)
        .bind(i64::from(size))
        .bind(offset)
        .bind(query.viewer_user_id.as_str())
        .bind(pnu_filter)
        .bind(admin_prefix_filter)
        .bind(land_use_filter)
        .fetch_all(pool)
        .await
        .map_err(|e| RepoError::Database(e.to_string()))?;

    let mut total_count: u64 = 0;
    let mut cards: Vec<ListingCardSummary> = Vec::with_capacity(rows.len());
    for row in &rows {
        let (card, row_total) = row_to_card_summary(row)?;
        total_count = row_total;
        cards.push(card);
    }

    Ok((cards, total_count))
}

fn row_to_card_summary(row: &PgRow) -> Result<(ListingCardSummary, u64), RepoError> {
    let id_str: String = get(row, "id")?;
    let id = Id::<ListingIdMarker>::try_from_str(&id_str)
        .map_err(|e| RepoError::Database(format!("invalid listing id: {e}")))?;

    let lt_str: String = get(row, "listing_type")?;
    let tt_str: String = get(row, "transaction_type")?;
    let deposit_opt: Option<i64> = get(row, "deposit_krw")?;
    let rent_opt: Option<i64> = get(row, "monthly_rent_krw")?;
    let total_count_i64: i64 = row.try_get("total_count").unwrap_or(0_i64);

    let card = ListingCardSummary {
        id,
        title: get(row, "title")?,
        listing_type: parse_listing_type(&lt_str)?,
        transaction_type: parse_transaction_type(&tt_str)?,
        price: money(get(row, "price_krw")?, "price_krw")?,
        deposit: deposit_opt.map(|v| money(v, "deposit_krw")).transpose()?,
        monthly_rent: rent_opt.map(|v| money(v, "monthly_rent_krw")).transpose()?,
        area_m2: get(row, "area_m2")?,
        thumbnail_url: None,
        view_count: row.try_get("view_count").unwrap_or(0_i64),
        bookmark_count: row.try_get("bookmark_count").unwrap_or(0_i64),
        is_bookmarked: row.try_get("is_bookmarked").unwrap_or(false),
        created_at: get(row, "created_at")?,
    };

    Ok((card, u64::try_from(total_count_i64.max(0)).unwrap_or(0)))
}

fn get<'r, T>(row: &'r PgRow, column: &str) -> Result<T, RepoError>
where
    T: sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
    row.try_get(column)
        .map_err(|e| RepoError::Database(e.to_string()))
}

fn money(value: i64, column: &str) -> Result<MoneyKrw, RepoError> {
    MoneyKrw::try_new(value)
        .map_err(|e| RepoError::Database(format!("invalid {column} in DB: {e}")))
}
