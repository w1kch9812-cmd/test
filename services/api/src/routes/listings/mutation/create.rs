use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_role;
use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use listing_domain::entity::Listing;
use listing_domain::repository::ListingParcelDenormalize;
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{from_listing_error, from_listing_repo_error, problem, ProblemResponse};

use super::super::state::ListingsState;

/// `POST /listings` 요청 본문.
#[derive(Debug, Deserialize)]
pub struct CreateListingRequest {
    /// 필지 PNU (19자리).
    pub parcel_pnu: String,
    /// 매물 유형 (`factory`/`warehouse`/...).
    pub listing_type: String,
    /// 거래 유형 (`sale`/`monthly_rent`/`jeonse`).
    pub transaction_type: String,
    /// 가격 (KRW).
    pub price_krw: i64,
    /// 보증금 (KRW). `MonthlyRent` / `Jeonse` 만 Some.
    pub deposit_krw: Option<i64>,
    /// 월세 (KRW). `MonthlyRent` 만 Some.
    pub monthly_rent_krw: Option<i64>,
    /// 면적 (m²).
    pub area_m2: f64,
    /// 제목 (≤200자).
    pub title: String,
    /// 설명 (≤5000자).
    pub description: String,
    /// 연락처 공개 범위 (default `login_required`).
    pub contact_visibility: Option<String>,
}

/// `POST /listings` 응답.
#[derive(Debug, Serialize)]
pub struct CreateListingResponse {
    /// 새 매물 ID (`lst_<26 ULID>`).
    pub id: String,
    /// 초기 version (= 1).
    pub version: i64,
}

/// `POST /listings` — 매물 등록 (Broker 전용).
///
/// 도메인 invariant 검증 → `Listing::try_new_draft` → `repo.save(&listing, ctx)`.
/// `actor_id == owner_id` (인증 사용자 = 소유자). `MutationContext::new_user_action`
/// 으로 `audit_log` + outbox row 자동 INSERT.
#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip(state, auth, body), fields(actor = %auth.user.id))]
pub async fn create_listing(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Json(body): Json<CreateListingRequest>,
) -> Result<(StatusCode, Json<CreateListingResponse>), ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let listing = build_draft_from_request(&body, auth.user.id.clone(), Utc::now())?;
    let ctx = http_user_action(&auth, "create_listing");

    state.listing_repo.save(&listing, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "listing save failed");
        from_listing_repo_error(&e)
    })?;

    // SP9 T4 / ADR 0018: PNU lookup → denormalize 컬럼 채움. best-effort —
    // 실패 시 listing 은 생성됨 (denormalize NULL). 월간 재매핑 cron 이 backfill.
    populate_parcel_denormalize(&state, &listing).await;

    Ok((
        StatusCode::CREATED,
        Json(CreateListingResponse {
            id: listing.id.as_str().to_owned(),
            version: listing.version,
        }),
    ))
}

/// PNU lookup → `admin_code/land_use_type/zoning` denormalize 갱신.
///
/// best-effort — 모든 단계 실패는 warn 로그 후 swallow. listing row 자체는
/// `create_listing` 에서 이미 commit 됐고, denormalize 가 NULL 이어도 검색이
/// 동작 안 할 뿐 (PNU 기준 검색은 가능). 월간 ETL cron 이 stale row 재매핑.
#[allow(clippy::cognitive_complexity)] // 3 분기 + 2 logging — 분해 시 가독성 떨어짐
async fn populate_parcel_denormalize(state: &ListingsState, listing: &Listing) {
    let info = match state.parcel_lookup.lookup_by_pnu(&listing.parcel_pnu).await {
        Ok(Some(info)) => info,
        Ok(None) => {
            tracing::warn!(
                listing_id = %listing.id,
                pnu = %listing.parcel_pnu,
                "parcel_lookup returned None — denormalize skipped"
            );
            return;
        }
        Err(e) => {
            tracing::warn!(
                listing_id = %listing.id,
                pnu = %listing.parcel_pnu,
                error = %e,
                "parcel_lookup failed — denormalize skipped"
            );
            return;
        }
    };

    let denormalize = ListingParcelDenormalize {
        admin_code: info.admin.eupmyeondong.clone(),
        land_use_type: info.land_use_type,
        zoning: info.zoning,
    };

    if let Err(e) = state
        .listing_repo
        .update_parcel_denormalize(&listing.id, &denormalize)
        .await
    {
        tracing::warn!(
            listing_id = %listing.id,
            error = %e,
            "update_parcel_denormalize failed — denormalize NULL until cron backfills"
        );
    }
}

/// 입력 → 도메인 값 객체 변환 + `Listing::try_new_draft`. 도메인 invariant 위반은
/// `from_listing_error` 로 RFC 7807 매핑.
#[allow(clippy::too_many_lines)] // 14 필드 변환 each with own ProblemDetails — 분해 시 중복 boilerplate.
fn build_draft_from_request(
    body: &CreateListingRequest,
    owner_id: Id<UserMarker>,
    now: DateTime<Utc>,
) -> Result<Listing, ProblemResponse> {
    let pnu = Pnu::try_new(&body.parcel_pnu).map_err(|e| {
        problem(
            "validation",
            "parcel_pnu 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let listing_type = ListingType::from_str(&body.listing_type).map_err(|e| {
        problem(
            "validation",
            "listing_type 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let transaction_type = TransactionType::from_str(&body.transaction_type).map_err(|e| {
        problem(
            "validation",
            "transaction_type 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let price = MoneyKrw::try_new(body.price_krw).map_err(|e| {
        problem(
            "validation",
            "price_krw 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let deposit = body
        .deposit_krw
        .map(MoneyKrw::try_new)
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "deposit_krw 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let monthly_rent = body
        .monthly_rent_krw
        .map(MoneyKrw::try_new)
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "monthly_rent_krw 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let area = AreaM2::try_new(body.area_m2).map_err(|e| {
        problem(
            "validation",
            "area_m2 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let title = ListingTitle::try_new(&body.title).map_err(|e| {
        problem(
            "validation",
            "title 이 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let description = Description::try_new(&body.description).map_err(|e| {
        problem(
            "validation",
            "description 이 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let mut listing = Listing::try_new_draft(
        Id::<ListingIdMarker>::new(),
        owner_id,
        pnu,
        listing_type,
        transaction_type,
        price,
        deposit,
        monthly_rent,
        area,
        title,
        description,
        now,
    )
    .map_err(|e| from_listing_error(&e))?;

    if let Some(cv_str) = body.contact_visibility.as_deref() {
        let cv = ContactVisibility::from_str(cv_str).map_err(|e| {
            problem(
                "validation",
                "contact_visibility 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
        listing.contact_visibility = cv;
    }

    Ok(listing)
}
