//! `GET /listings` — 카드 list 검색 endpoint (SP6-ii).

use std::str::FromStr;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::{DateTime, Utc};
use listing_domain::repository::{CardSearchQuery, CardSearchSort, ListingRepository};
use serde::{Deserialize, Serialize};
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::transaction_type::TransactionType;

/// 핸들러 공유 상태.
#[derive(Clone)]
pub struct ListingsState {
    /// `Listing` 저장소.
    pub listing_repo: Arc<dyn ListingRepository>,
}

/// `GET /listings` 쿼리 파라미터.
#[derive(Debug, Deserialize)]
pub struct ListingsQuery {
    /// 지도 영역: `"south,west,north,east"` (float 4개, WGS84).
    pub bounds: Option<String>,
    /// 매물 유형 필터: comma-separated (예: `"factory,warehouse"`).
    pub types: Option<String>,
    /// 거래 유형 필터: comma-separated (예: `"sale,jeonse"`).
    pub transaction: Option<String>,
    /// 최소 면적 (m²).
    pub min_area_m2: Option<f64>,
    /// 최대 면적 (m²).
    pub max_area_m2: Option<f64>,
    /// 최소 가격 (원).
    pub min_price_krw: Option<i64>,
    /// 최대 가격 (원).
    pub max_price_krw: Option<i64>,
    /// 페이지 번호 (0-indexed, default 0).
    pub page: Option<u32>,
    /// 페이지 당 항목 수 (default 20, max 100).
    pub size: Option<u32>,
    /// 정렬: `created_at_desc`(기본) | `price_asc` | `price_desc` | `area_asc` | `area_desc`.
    pub sort: Option<String>,
}

/// 카드 응답 단건.
#[derive(Debug, Serialize)]
pub struct ListingCardResponse {
    /// 매물 ID.
    pub id: String,
    /// 제목.
    pub title: String,
    /// 매물 유형.
    pub listing_type: String,
    /// 거래 유형.
    pub transaction_type: String,
    /// 주가격 (원).
    pub price_krw: i64,
    /// 보증금 (원, 해당 시).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deposit_krw: Option<i64>,
    /// 월세 (원, 해당 시).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_rent_krw: Option<i64>,
    /// 면적 (m²).
    pub area_m2: f64,
    /// 위도.
    pub lat: f64,
    /// 경도.
    pub lng: f64,
    /// 썸네일 URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// 조회수.
    pub view_count: i64,
    /// 즐겨찾기 수.
    pub bookmark_count: i64,
    /// 등록일 (RFC 3339).
    pub created_at: DateTime<Utc>,
}

/// 페이지네이션 포함 응답.
#[derive(Debug, Serialize)]
pub struct ListingsResponse {
    /// 카드 list.
    pub listings: Vec<ListingCardResponse>,
    /// 전체 매물 수 (필터 적용 후).
    pub total: u64,
    /// 현재 페이지 (0-indexed).
    pub page: u32,
    /// 페이지 크기.
    pub size: u32,
    /// 다음 페이지 존재 여부.
    pub has_next: bool,
}

/// RFC 7807 Problem Details.
#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    /// URI 식별자 (`https://gongzzang.com/errors/<id>`).
    #[serde(rename = "type")]
    pub type_: String,
    /// 사람이 읽는 요약.
    pub title: String,
    /// HTTP 상태 코드.
    pub status: u16,
    /// 상세 설명 (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// RFC 7807 응답 생성 헬퍼.
fn problem(
    type_id: &str,
    title: &str,
    status: StatusCode,
    detail: Option<String>,
) -> (StatusCode, Json<ProblemDetails>) {
    (
        status,
        Json(ProblemDetails {
            type_: format!("https://gongzzang.com/errors/{type_id}"),
            title: title.to_owned(),
            status: status.as_u16(),
            detail,
        }),
    )
}

/// `GET /listings` — 카드 list 검색 (인증 필수).
#[allow(clippy::too_many_lines)]
pub async fn get_listings(
    State(state): State<ListingsState>,
    _auth: AuthenticatedUser,
    Query(q): Query<ListingsQuery>,
) -> Result<Json<ListingsResponse>, (StatusCode, Json<ProblemDetails>)> {
    // bounds 파싱: "south,west,north,east" → BoundingBox(min_lng=west, min_lat=south, max_lng=east, max_lat=north).
    let bbox = if let Some(b) = q.bounds.as_deref() {
        let parts: Vec<&str> = b.split(',').collect();
        if parts.len() != 4 {
            return Err(problem(
                "listings/invalid-bounds",
                "bounds 파라미터가 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some("expected 'south,west,north,east' (4 floats)".into()),
            ));
        }
        let floats: Result<Vec<f64>, _> = parts.iter().map(|s| s.trim().parse::<f64>()).collect();
        let floats = floats.map_err(|e| {
            problem(
                "listings/invalid-bounds",
                "bounds 파라미터가 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
        // south=floats[0], west=floats[1], north=floats[2], east=floats[3]
        // BoundingBox: min_lng=west, min_lat=south, max_lng=east, max_lat=north
        BoundingBox::try_new_wgs84(floats[1], floats[0], floats[3], floats[2])
            .map(Some)
            .map_err(|e| {
                problem(
                    "listings/invalid-bounds",
                    "bounds 파라미터가 올바르지 않아요",
                    StatusCode::BAD_REQUEST,
                    Some(e.to_string()),
                )
            })?
    } else {
        None
    };

    let types = if let Some(s) = q.types.as_deref().filter(|s| !s.is_empty()) {
        let parsed: Result<Vec<ListingType>, _> = s
            .split(',')
            .map(|t| ListingType::from_str(t.trim()))
            .collect();
        Some(parsed.map_err(|e| {
            problem(
                "listings/invalid-filter",
                "types 필터 값이 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?)
    } else {
        None
    };

    let transactions = if let Some(s) = q.transaction.as_deref().filter(|s| !s.is_empty()) {
        let parsed: Result<Vec<TransactionType>, _> = s
            .split(',')
            .map(|t| TransactionType::from_str(t.trim()))
            .collect();
        Some(parsed.map_err(|e| {
            problem(
                "listings/invalid-filter",
                "transaction 필터 값이 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?)
    } else {
        None
    };

    let sort = match q.sort.as_deref() {
        Some("price_asc") => CardSearchSort::PriceAsc,
        Some("price_desc") => CardSearchSort::PriceDesc,
        Some("area_asc") => CardSearchSort::AreaAsc,
        Some("area_desc") => CardSearchSort::AreaDesc,
        Some("created_at_desc") | None => CardSearchSort::CreatedAtDesc,
        Some(other) => {
            return Err(problem(
                "listings/invalid-filter",
                "sort 값이 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some(format!("unknown sort: {other}")),
            ));
        }
    };

    let page = q.page.unwrap_or(0);
    let size = q.size.unwrap_or(20).min(100);

    let query = CardSearchQuery {
        bbox,
        types,
        transactions,
        min_area_m2: q.min_area_m2,
        max_area_m2: q.max_area_m2,
        min_price_krw: q.min_price_krw,
        max_price_krw: q.max_price_krw,
        page,
        size,
        sort,
    };

    let (cards, total) = state
        .listing_repo
        .find_card_summaries_in_bbox(query)
        .await
        .map_err(|e| {
            problem(
                "listings/database",
                "매물 검색 중 오류가 발생했어요",
                StatusCode::INTERNAL_SERVER_ERROR,
                Some(e.to_string()),
            )
        })?;

    let listings: Vec<ListingCardResponse> = cards
        .into_iter()
        .map(|c| ListingCardResponse {
            id: c.id.as_str().to_owned(),
            title: c.title,
            listing_type: c.listing_type.as_str().to_owned(),
            transaction_type: c.transaction_type.as_str().to_owned(),
            price_krw: c.price.as_i64(),
            deposit_krw: c.deposit.map(MoneyKrw::as_i64),
            monthly_rent_krw: c.monthly_rent.map(MoneyKrw::as_i64),
            area_m2: c.area_m2,
            lat: c.geom.lat,
            lng: c.geom.lng,
            thumbnail_url: c.thumbnail_url,
            view_count: c.view_count,
            bookmark_count: c.bookmark_count,
            created_at: c.created_at,
        })
        .collect();

    let has_next = (u64::from(page) + 1) * u64::from(size) < total;

    Ok(Json(ListingsResponse {
        listings,
        total,
        page,
        size,
        has_next,
    }))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn problem_details_serializes_with_type_field() {
        let p = problem(
            "listings/invalid-bounds",
            "잘못된 bounds",
            StatusCode::BAD_REQUEST,
            None,
        );
        let json = serde_json::to_string(&p.1 .0).unwrap();
        assert!(
            json.contains("\"type\":\"https://gongzzang.com/errors/listings/invalid-bounds\""),
            "type field missing: {json}"
        );
        assert!(
            json.contains("\"status\":400"),
            "status field missing: {json}"
        );
    }

    #[test]
    fn problem_details_omits_detail_when_none() {
        let p = problem("listings/test", "t", StatusCode::BAD_REQUEST, None);
        let json = serde_json::to_string(&p.1 .0).unwrap();
        assert!(
            !json.contains("\"detail\""),
            "detail should be omitted when None: {json}"
        );
        assert!(
            json.contains("\"status\":400"),
            "status field missing: {json}"
        );
    }

    #[test]
    fn problem_details_includes_detail_when_some() {
        let p = problem(
            "listings/test",
            "t",
            StatusCode::INTERNAL_SERVER_ERROR,
            Some("DB connection failed".into()),
        );
        let json = serde_json::to_string(&p.1 .0).unwrap();
        assert!(
            json.contains("\"detail\":\"DB connection failed\""),
            "detail missing: {json}"
        );
        assert!(
            json.contains("\"status\":500"),
            "status field missing: {json}"
        );
    }
}
