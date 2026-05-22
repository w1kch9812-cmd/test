use std::str::FromStr;

use auth::middleware::AuthenticatedUser;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use listing_domain::repository::{CardSearchQuery, CardSearchSort};
use serde::{Deserialize, Serialize};
use shared_kernel::land_use_type::LandUseType;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;

use crate::http::problem::{problem, ProblemResponse};

use super::state::ListingsState;

/// `GET /listings` 쿼리 파라미터.
#[derive(Debug, Deserialize)]
pub struct ListingsQuery {
    /// 필지 PNU 19자리 — 폴리곤 클릭 시 해당 필지 매물만 (ADR 0018, SP9 T4).
    pub pnu: Option<String>,
    /// 행정구역 코드 (시도 2 / 시군구 5 / 읍면동 8자리). prefix 매치로 처리해 시도/
    /// 시군구/읍면동 어느 단계든 사용 가능.
    pub admin_code: Option<String>,
    /// 지목 필터 (예: `factory_site`, `warehouse_site`).
    pub land_use_type: Option<String>,
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
    /// 썸네일 URL (SP6-iii 이후 채워짐).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    /// 조회수.
    pub view_count: i64,
    /// 즐겨찾기 수.
    pub bookmark_count: i64,
    /// 본 viewer 가 즐겨찾기 한 매물인지 (SP6-iii).
    pub is_bookmarked: bool,
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

/// `GET /listings` — 카드 list 검색 (인증 필수).
#[allow(clippy::too_many_lines)]
#[tracing::instrument(
    skip(state, auth),
    fields(
        page = q.page,
        size = q.size,
        sort = ?q.sort,
    ),
)]
pub async fn get_listings(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Query(q): Query<ListingsQuery>,
) -> Result<Json<ListingsResponse>, ProblemResponse> {
    // size 검증: 0 은 has_next 무한 루프를 유발, 100 초과는 서버 부하 방지.
    let size = q.size.unwrap_or(20);
    if size == 0 || size > 100 {
        return Err(problem(
            "listings/invalid-filter",
            "size 파라미터는 1~100 사이여야 해요",
            StatusCode::BAD_REQUEST,
            Some(format!("got size={size}, allowed 1..=100")),
        ));
    }

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

    // ADR 0018 SP9 T4: pnu/admin_code/land_use_type 검증.
    let pnu_filter = q
        .pnu
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(Pnu::try_new)
        .transpose()
        .map_err(|e| {
            problem(
                "listings/invalid-filter",
                "pnu 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;

    // admin_code prefix — 2/5/8 자리만 허용 (시도/시군구/읍면동). 그 외는 잘못된
    // 입력 — DB LIKE 가 받기는 하나 의미 없는 prefix 차단.
    let admin_prefix_filter = match q.admin_code.as_deref().filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => {
            if !matches!(s.len(), 2 | 5 | 8) || !s.chars().all(|c| c.is_ascii_digit()) {
                return Err(problem(
                    "listings/invalid-filter",
                    "admin_code 는 2 / 5 / 8 자리 숫자여야 해요",
                    StatusCode::BAD_REQUEST,
                    Some(format!("got '{s}'")),
                ));
            }
            Some(s.to_owned())
        }
    };

    let land_use_filter = q
        .land_use_type
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(LandUseType::from_str)
        .transpose()
        .map_err(|e| {
            problem(
                "listings/invalid-filter",
                "land_use_type 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;

    let query = CardSearchQuery {
        pnu: pnu_filter,
        admin_code_prefix: admin_prefix_filter,
        land_use_type: land_use_filter,
        types,
        transactions,
        min_area_m2: q.min_area_m2,
        max_area_m2: q.max_area_m2,
        min_price_krw: q.min_price_krw,
        max_price_krw: q.max_price_krw,
        page,
        size,
        sort,
        viewer_user_id: auth.user.id.clone(),
    };

    let (cards, total) = state
        .listing_repo
        .find_card_summaries(query)
        .await
        .map_err(|e| {
            // C1: DB 내부 정보(쿼리 구조, 테이블명 등)를 client 에 노출하지 않음.
            // 서버 log 에만 기록, 응답은 generic message.
            tracing::error!(error = %e, "listing DB query failed");
            problem(
                "listings/database",
                "매물 검색 중 오류가 발생했어요",
                StatusCode::INTERNAL_SERVER_ERROR,
                None, // production 보안 — DB internal 노출 금지
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
            thumbnail_url: c.thumbnail_url,
            view_count: c.view_count,
            bookmark_count: c.bookmark_count,
            is_bookmarked: c.is_bookmarked,
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
