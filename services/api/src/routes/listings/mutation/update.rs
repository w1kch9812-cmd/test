use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_role;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use chrono::Utc;
use listing_domain::entity::ListingUpdate;
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::id::{Id, ListingMarker as ListingIdMarker};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::money::MoneyKrw;
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{from_listing_error, from_listing_repo_error, problem, ProblemResponse};

use super::super::state::ListingsState;

/// `PATCH /listings/:id` 요청 본문 — 모든 필드 optional. partial update 패턴.
///
/// `Option<Option<T>>` 은 의도된 partial-update 패턴: 외부 `None` = 미언급
/// (값 보존), 외부 `Some(None)` = `null` 명시 (clear), 외부 `Some(Some(v))` =
/// 새 값. JSON `{}` vs `{"deposit_krw":null}` 구분이 본질이라 clippy 권고
/// `Option<T>` 대체 불가능.
#[derive(Debug, Deserialize, Default)]
#[allow(clippy::option_option)]
pub struct UpdateListingRequest {
    /// 새 제목.
    pub title: Option<String>,
    /// 새 설명.
    pub description: Option<String>,
    /// 새 가격.
    pub price_krw: Option<i64>,
    /// `null` = clear, `Some(n)` = 새 값, 미언급 = 그대로.
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub deposit_krw: Option<Option<i64>>,
    /// 동일 패턴.
    #[serde(default, deserialize_with = "deserialize_optional_field")]
    pub monthly_rent_krw: Option<Option<i64>>,
    /// 새 면적.
    pub area_m2: Option<f64>,
    /// 새 `contact_visibility`.
    pub contact_visibility: Option<String>,
}

/// `null` 과 "필드 미언급" 구분 — `Option<Option<T>>` 패턴.
#[allow(clippy::option_option)]
fn deserialize_optional_field<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Option::<T>::deserialize(de).map(Some)
}

/// `PATCH /listings/:id` 응답.
#[derive(Debug, Serialize)]
pub struct UpdateListingResponse {
    /// 매물 ID.
    pub id: String,
    /// 새 version.
    pub version: i64,
}

/// `PATCH /listings/:id` — 매물 수정 (Broker + 소유자 전용, OCC `if-match`).
///
/// `if-match` header 가 stale 이면 409 — 도메인 메서드 가 version bump 후
/// `repo.save` 가 DB OCC 로 다시 검증 (이중 가드).
#[tracing::instrument(skip(state, auth, headers, body), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn patch_listing(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<UpdateListingRequest>,
) -> Result<Json<UpdateListingResponse>, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let listing_id = Id::<ListingIdMarker>::from_str(&id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let if_match = parse_if_match_header(&headers)?;

    let mut listing = state
        .listing_repo
        .find(&listing_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing find failed");
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

    if listing.owner_id != auth.user.id {
        return Err(problem(
            "forbidden",
            "본인 매물만 수정할 수 있어요",
            StatusCode::FORBIDDEN,
            None,
        ));
    }
    if listing.version != if_match {
        return Err(problem(
            "version-conflict",
            "동시 수정 충돌 — 다시 불러오세요",
            StatusCode::CONFLICT,
            Some(format!(
                "if-match={if_match}, current_version={}",
                listing.version
            )),
        ));
    }

    let update = build_update_from_request(body)?;
    listing
        .update_editable_fields(update, Utc::now())
        .map_err(|e| from_listing_error(&e))?;

    let ctx = http_user_action(&auth, "update_listing");
    state.listing_repo.save(&listing, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "listing save (update) failed");
        from_listing_repo_error(&e)
    })?;

    Ok(Json(UpdateListingResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
    }))
}

fn parse_if_match_header(headers: &HeaderMap) -> Result<i64, ProblemResponse> {
    let header_value = headers.get("if-match").ok_or_else(|| {
        problem(
            "validation",
            "if-match header 가 필요해요 (OCC)",
            StatusCode::BAD_REQUEST,
            Some("미일치 시 stale 한 응답 위 덮어쓰기 위험".to_owned()),
        )
    })?;
    let s = header_value.to_str().map_err(|_| {
        problem(
            "validation",
            "if-match header 가 ASCII 가 아니에요",
            StatusCode::BAD_REQUEST,
            None,
        )
    })?;
    // ETag-style 따옴표 strip.
    let trimmed = s.trim().trim_matches('"');
    trimmed.parse::<i64>().map_err(|e| {
        problem(
            "validation",
            "if-match header 는 정수 version 이어야 해요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })
}

#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
fn build_update_from_request(body: UpdateListingRequest) -> Result<ListingUpdate, ProblemResponse> {
    let title = body
        .title
        .map(|t| ListingTitle::try_new(&t))
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "title 이 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let description = body
        .description
        .map(|d| Description::try_new(&d))
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "description 이 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let price = body
        .price_krw
        .map(MoneyKrw::try_new)
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "price_krw 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let deposit = match body.deposit_krw {
        Some(Some(v)) => Some(Some(MoneyKrw::try_new(v).map_err(|e| {
            problem(
                "validation",
                "deposit_krw 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?)),
        Some(None) => Some(None),
        None => None,
    };
    let monthly_rent = match body.monthly_rent_krw {
        Some(Some(v)) => Some(Some(MoneyKrw::try_new(v).map_err(|e| {
            problem(
                "validation",
                "monthly_rent_krw 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?)),
        Some(None) => Some(None),
        None => None,
    };
    let area = body.area_m2.map(AreaM2::try_new).transpose().map_err(|e| {
        problem(
            "validation",
            "area_m2 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let contact_visibility = body
        .contact_visibility
        .map(|s| ContactVisibility::from_str(&s))
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "contact_visibility 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;

    Ok(ListingUpdate {
        title,
        description,
        price,
        deposit,
        monthly_rent,
        area,
        contact_visibility,
    })
}
