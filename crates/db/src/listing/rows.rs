use bigdecimal::{BigDecimal, ToPrimitive};
use chrono::{DateTime, Utc};
use listing_domain::entity::Listing;
use listing_domain::repository::RepoError;
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::listing_status::ListingStatus;
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use sqlx::postgres::PgRow;
use sqlx::Row;

/// All `listing` aggregate columns.
pub(super) const LISTING_FULL_COLUMNS: &str =
    "id, owner_id, parcel_pnu, listing_type, transaction_type, \
    price_krw, deposit_krw, monthly_rent_krw, area_m2, \
    title, description, status, contact_visibility, \
    view_count, bookmark_count, \
    created_at, updated_at, expires_at, version";

pub(super) const LISTING_FULL_COLUMNS_WITH_L_ALIAS: &str =
    "l.id, l.owner_id, l.parcel_pnu, l.listing_type, \
    l.transaction_type, l.price_krw, l.deposit_krw, l.monthly_rent_krw, l.area_m2, \
    l.title, l.description, l.status, l.contact_visibility, \
    l.view_count, l.bookmark_count, \
    l.created_at, l.updated_at, l.expires_at, l.version";

pub(super) fn row_to_listing(row: &PgRow) -> Result<Listing, RepoError> {
    ListingDbRow::from_row(row)?.into_listing()
}

struct ListingDbRow {
    id_str: String,
    owner_id_str: String,
    parcel_pnu_str: String,
    listing_type_str: String,
    transaction_type_str: String,
    price_krw: i64,
    deposit_krw: Option<i64>,
    monthly_rent_krw: Option<i64>,
    area_decimal: BigDecimal,
    title_str: String,
    description_str: String,
    status_str: String,
    contact_vis_str: String,
    view_count_i64: i64,
    bookmark_count_i64: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
    version: i64,
}

impl ListingDbRow {
    fn from_row(row: &PgRow) -> Result<Self, RepoError> {
        Ok(Self {
            id_str: get(row, "id")?,
            owner_id_str: get(row, "owner_id")?,
            parcel_pnu_str: get(row, "parcel_pnu")?,
            listing_type_str: get(row, "listing_type")?,
            transaction_type_str: get(row, "transaction_type")?,
            price_krw: get(row, "price_krw")?,
            deposit_krw: get(row, "deposit_krw")?,
            monthly_rent_krw: get(row, "monthly_rent_krw")?,
            area_decimal: get(row, "area_m2")?,
            title_str: get(row, "title")?,
            description_str: get(row, "description")?,
            status_str: get(row, "status")?,
            contact_vis_str: get(row, "contact_visibility")?,
            view_count_i64: get(row, "view_count")?,
            bookmark_count_i64: get(row, "bookmark_count")?,
            created_at: get(row, "created_at")?,
            updated_at: get(row, "updated_at")?,
            expires_at: get(row, "expires_at")?,
            version: get(row, "version")?,
        })
    }

    fn into_listing(self) -> Result<Listing, RepoError> {
        let id = Id::<ListingIdMarker>::try_from_str(&self.id_str)
            .map_err(|e| RepoError::Database(format!("malformed listing id in DB: {e}")))?;
        let owner_id = Id::<UserMarker>::try_from_str(&self.owner_id_str)
            .map_err(|e| RepoError::Database(format!("malformed owner_id in DB: {e}")))?;
        let parcel_pnu = Pnu::try_new(&self.parcel_pnu_str)
            .map_err(|e| RepoError::Database(format!("malformed pnu in DB: {e}")))?;
        let listing_type = parse_listing_type(&self.listing_type_str)?;
        let transaction_type = parse_transaction_type(&self.transaction_type_str)?;
        let price = money(self.price_krw, "price_krw")?;
        let deposit = self
            .deposit_krw
            .map(|v| money(v, "deposit_krw"))
            .transpose()?;
        let monthly_rent = self
            .monthly_rent_krw
            .map(|v| money(v, "monthly_rent_krw"))
            .transpose()?;
        let area = area(&self.area_decimal)?;
        let title = ListingTitle::try_new(&self.title_str)
            .map_err(|e| RepoError::Database(format!("invalid title in DB: {e}")))?;
        let description = Description::try_new(&self.description_str)
            .map_err(|e| RepoError::Database(format!("invalid description in DB: {e}")))?;
        let status = parse_listing_status(&self.status_str)?;
        let contact_visibility = parse_contact_visibility(&self.contact_vis_str)?;

        Ok(Listing {
            id,
            owner_id,
            parcel_pnu,
            listing_type,
            transaction_type,
            price,
            deposit,
            monthly_rent,
            area,
            title,
            description,
            status,
            contact_visibility,
            view_count: u64::try_from(self.view_count_i64).unwrap_or(0),
            bookmark_count: u64::try_from(self.bookmark_count_i64).unwrap_or(0),
            created_at: self.created_at,
            updated_at: self.updated_at,
            expires_at: self.expires_at,
            version: self.version,
        })
    }
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

fn area(area_decimal: &BigDecimal) -> Result<AreaM2, RepoError> {
    let area_f64 = area_decimal
        .to_f64()
        .ok_or_else(|| RepoError::Database("area_m2 BigDecimal -> f64 conversion failed".into()))?;
    AreaM2::try_new(area_f64)
        .map_err(|e| RepoError::Database(format!("invalid area_m2 in DB: {e}")))
}

pub(super) fn parse_listing_type(s: &str) -> Result<ListingType, RepoError> {
    match s {
        "factory" => Ok(ListingType::Factory),
        "warehouse" => Ok(ListingType::Warehouse),
        "office" => Ok(ListingType::Office),
        "knowledge_industry_center" => Ok(ListingType::KnowledgeIndustryCenter),
        "industrial_land" => Ok(ListingType::IndustrialLand),
        "logistics_center" => Ok(ListingType::LogisticsCenter),
        other => Err(RepoError::Database(format!(
            "unexpected listing_type in DB: {other}"
        ))),
    }
}

pub(super) fn parse_transaction_type(s: &str) -> Result<TransactionType, RepoError> {
    match s {
        "sale" => Ok(TransactionType::Sale),
        "monthly_rent" => Ok(TransactionType::MonthlyRent),
        "jeonse" => Ok(TransactionType::Jeonse),
        other => Err(RepoError::Database(format!(
            "unexpected transaction_type in DB: {other}"
        ))),
    }
}

fn parse_listing_status(s: &str) -> Result<ListingStatus, RepoError> {
    match s {
        "draft" => Ok(ListingStatus::Draft),
        "pending_review" => Ok(ListingStatus::PendingReview),
        "active" => Ok(ListingStatus::Active),
        "sold" => Ok(ListingStatus::Sold),
        "expired" => Ok(ListingStatus::Expired),
        "rejected" => Ok(ListingStatus::Rejected),
        other => Err(RepoError::Database(format!(
            "unexpected status in DB: {other}"
        ))),
    }
}

fn parse_contact_visibility(s: &str) -> Result<ContactVisibility, RepoError> {
    match s {
        "public" => Ok(ContactVisibility::Public),
        "login_required" => Ok(ContactVisibility::LoginRequired),
        "verified_only" => Ok(ContactVisibility::VerifiedOnly),
        other => Err(RepoError::Database(format!(
            "unexpected contact_visibility in DB: {other}"
        ))),
    }
}
