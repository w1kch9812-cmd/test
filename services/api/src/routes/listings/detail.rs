use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use listing_domain::entity::Listing;
use serde::Serialize;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker, UserMarker};
use shared_kernel::money::MoneyKrw;

use crate::http::problem::{from_listing_repo_error, problem, ProblemResponse};

use super::state::ListingsState;

#[derive(Debug, Serialize)]
pub struct ListingDetailResponse {
    /// 매물 ID.
    pub id: String,
    /// 소유자 ID.
    pub owner_id: String,
    /// PNU 19자리.
    pub parcel_pnu: String,
    /// 매물 유형.
    pub listing_type: String,
    /// 거래 유형.
    pub transaction_type: String,
    /// 가격 (원).
    pub price_krw: i64,
    /// 보증금 (원, 해당 시).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_krw: Option<i64>,
    /// 월세 (원, 해당 시).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_rent_krw: Option<i64>,
    /// 면적 (m²).
    pub area_m2: f64,
    /// 제목.
    pub title: String,
    /// 설명.
    pub description: String,
    /// 상태 (`snake_case`).
    pub status: String,
    /// 연락처 공개 범위.
    pub contact_visibility: String,
    /// 조회수.
    pub view_count: i64,
    /// 즐겨찾기 수 (JOIN COUNT).
    pub bookmark_count: i64,
    /// 본 viewer 가 즐겨찾기 한 매물인지.
    pub is_bookmarked: bool,
    /// version.
    pub version: i64,
    /// 등록일.
    pub created_at: DateTime<Utc>,
    /// 마지막 갱신일.
    pub updated_at: DateTime<Utc>,
    /// 만료일 (선택).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// 사진 (`display_order ASC`).
    pub photos: Vec<PhotoResponse>,
}

/// 사진 응답.
#[derive(Debug, Serialize)]
pub struct PhotoResponse {
    /// Photo ID.
    pub photo_id: String,
    /// R2 객체 키.
    pub r2_key: String,
    /// 썸네일 키 (선택).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_r2_key: Option<String>,
    /// 캡션 (선택).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    /// 표시 순서.
    pub display_order: i32,
    /// MIME content-type.
    pub content_type: String,
}

/// `GET /listings/:id` — 매물 상세 (인증 사용자 + RBAC).
///
/// RBAC: `Active` / `Sold` / `Expired` 만 공개. `Draft` / `PendingReview` /
/// `Rejected` / `Archived` 는 owner only — 비owner 접근은 *404* (존재 자체 leak
/// 차단).
///
/// 본인 매물 아니면 `view_count` 1 증가 (best-effort).
#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn get_listing_detail(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<ListingDetailResponse>, ProblemResponse> {
    let listing_id = Id::<ListingIdMarker>::from_str(&id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let detail = state
        .listing_repo
        .find_detail_by_id(&listing_id, &auth.user.id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing detail find failed");
            from_listing_repo_error(&e)
        })?
        .ok_or_else(|| {
            problem(
                "not-found",
                "매물을 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            )
        })?;

    // RBAC: 비공개 상태는 owner only — 비owner = 404 (존재 leak X).
    if !can_view(&detail.listing, &auth.user.id) {
        return Err(problem(
            "not-found",
            "매물을 찾을 수 없어요",
            StatusCode::NOT_FOUND,
            None,
        ));
    }

    // view_count: 본인 본인 매물 X (자기 조회 노이즈 차단). best-effort.
    if detail.listing.owner_id != auth.user.id {
        if let Err(e) = state.listing_repo.increment_view_count(&listing_id).await {
            tracing::warn!(error = %e, "view_count update failed — proceeding");
        }
    }

    Ok(Json(detail_to_response(detail)))
}

/// 비공개 상태 RBAC. `Draft` / `PendingReview` / `Rejected` / `Archived` 는
/// owner only — 비owner 진입은 404.
fn can_view(listing: &Listing, viewer_id: &Id<UserMarker>) -> bool {
    use shared_kernel::listing_status::ListingStatus;
    matches!(
        listing.status,
        ListingStatus::Active | ListingStatus::Sold | ListingStatus::Expired
    ) || listing.owner_id == *viewer_id
}

#[allow(clippy::needless_pass_by_value)]
fn detail_to_response(detail: listing_domain::repository::ListingDetail) -> ListingDetailResponse {
    let l = detail.listing;
    ListingDetailResponse {
        id: l.id.as_str().to_owned(),
        owner_id: l.owner_id.as_str().to_owned(),
        parcel_pnu: l.parcel_pnu.as_str().to_owned(),
        listing_type: l.listing_type.as_str().to_owned(),
        transaction_type: l.transaction_type.as_str().to_owned(),
        price_krw: l.price.as_i64(),
        deposit_krw: l.deposit.map(MoneyKrw::as_i64),
        monthly_rent_krw: l.monthly_rent.map(MoneyKrw::as_i64),
        area_m2: l.area.as_f64(),
        title: l.title.as_str().to_owned(),
        description: l.description.as_str().to_owned(),
        status: l.status.as_str().to_owned(),
        contact_visibility: l.contact_visibility.as_str().to_owned(),
        view_count: i64::try_from(l.view_count).unwrap_or(i64::MAX),
        bookmark_count: detail.bookmark_count,
        is_bookmarked: detail.is_bookmarked,
        version: l.version,
        created_at: l.created_at,
        updated_at: l.updated_at,
        expires_at: l.expires_at,
        photos: detail
            .photos
            .into_iter()
            .map(|p| PhotoResponse {
                photo_id: p.photo_id,
                r2_key: p.r2_key,
                thumbnail_r2_key: p.thumbnail_r2_key,
                caption: p.caption,
                display_order: p.display_order,
                content_type: p.content_type,
            })
            .collect(),
    }
}
