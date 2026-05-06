//! `/listings` 핸들러 — GET 검색 (SP6-ii) + POST/PATCH/transitions/photos (SP6-iv).

use std::str::FromStr;
use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use auth::role_guard::require_role;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::{Extension, Json};
use chrono::{DateTime, Utc};
use listing_domain::entity::{Listing, ListingUpdate};
use listing_domain::repository::{CardSearchQuery, CardSearchSort, ListingRepository};
use listing_photo_domain::entity::ListingPhoto;
use listing_photo_domain::repository::ListingPhotoRepository;
use serde::{Deserialize, Serialize};
use shared_kernel::area::AreaM2;
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::contact_visibility::ContactVisibility;
use shared_kernel::description::Description;
use shared_kernel::geometry::PointSrid;
use shared_kernel::id::{
    Id, ListingMarker as ListingIdMarker, ListingPhotoMarker, UserMarker,
};
use shared_kernel::listing_title::ListingTitle;
use shared_kernel::listing_type::ListingType;
use shared_kernel::money::MoneyKrw;
use shared_kernel::pnu::Pnu;
use shared_kernel::transaction_type::TransactionType;
use user_domain::entity::UserRole;

use crate::http::mutation_ctx::http_user_action;
use crate::http::problem::{
    from_listing_error, from_listing_repo_error, problem, ProblemResponse,
};

/// 핸들러 공유 상태.
#[derive(Clone)]
pub struct ListingsState {
    /// `Listing` 저장소.
    pub listing_repo: Arc<dyn ListingRepository>,
    /// `ListingPhoto` 저장소.
    pub photo_repo: Arc<dyn ListingPhotoRepository>,
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
        viewer_user_id: auth.user.id.clone(),
    };

    let (cards, total) = state
        .listing_repo
        .find_card_summaries_in_bbox(query)
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
            lat: c.geom.lat,
            lng: c.geom.lng,
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

// ─────────────────────────────────────────────────────────────────────────
// SP6-iv: 매물 등록 / 수정 / 상태 전이 / 사진 (Broker 전용)
// ─────────────────────────────────────────────────────────────────────────

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
    /// 좌표 — 선택. 없으면 None.
    pub geom_point: Option<GeomPointInput>,
    /// 연락처 공개 범위 (default `login_required`).
    pub contact_visibility: Option<String>,
}

/// 좌표 입력.
#[derive(Debug, Deserialize)]
pub struct GeomPointInput {
    /// 경도.
    pub lng: f64,
    /// 위도.
    pub lat: f64,
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
/// 으로 audit_log + outbox row 자동 INSERT.
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

    state
        .listing_repo
        .save(&listing, ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing save failed");
            from_listing_repo_error(&e)
        })?;

    Ok((
        StatusCode::CREATED,
        Json(CreateListingResponse {
            id: listing.id.as_str().to_owned(),
            version: listing.version,
        }),
    ))
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
    let geom_point = body
        .geom_point
        .as_ref()
        .map(|g| PointSrid::try_new_wgs84(g.lng, g.lat))
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "geom_point 가 유효하지 않아요",
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
        geom_point,
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
    /// `null` = 좌표 제거, 객체 = 변경, 미언급 = 그대로.
    #[serde(default, deserialize_with = "deserialize_optional_geom")]
    pub geom_point: Option<Option<GeomPointInput>>,
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

#[allow(clippy::option_option)]
fn deserialize_optional_geom<'de, D>(de: D) -> Result<Option<Option<GeomPointInput>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::<GeomPointInput>::deserialize(de).map(Some)
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
    state
        .listing_repo
        .save(&listing, ctx)
        .await
        .map_err(|e| {
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
    let area = body
        .area_m2
        .map(AreaM2::try_new)
        .transpose()
        .map_err(|e| {
            problem(
                "validation",
                "area_m2 가 유효하지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
    let geom_point = match body.geom_point {
        Some(Some(g)) => Some(Some(PointSrid::try_new_wgs84(g.lng, g.lat).map_err(
            |e| {
                problem(
                    "validation",
                    "geom_point 가 유효하지 않아요",
                    StatusCode::BAD_REQUEST,
                    Some(e.to_string()),
                )
            },
        )?)),
        Some(None) => Some(None),
        None => None,
    };
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
        geom_point,
        contact_visibility,
    })
}

// ─────────────────────────────────────────────────────────────────────────
// 상태 전이 (SP6-iv T5)
// ─────────────────────────────────────────────────────────────────────────

/// 상태 전이 응답 — id + new version.
#[derive(Debug, Serialize)]
pub struct TransitionResponse {
    /// 매물 ID.
    pub id: String,
    /// 전이 후 version.
    pub version: i64,
    /// 전이 후 status (`snake_case`).
    pub status: String,
}

/// `POST /listings/:id/submit-for-review` — Draft → PendingReview (Broker 전용).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn submit_for_review(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<TransitionResponse>, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let mut listing = load_listing_for_actor(&state, &auth, &id).await?;
    listing
        .submit_for_review(Utc::now())
        .map_err(|e| from_listing_error(&e))?;

    let ctx = http_user_action(&auth, "submit_for_review");
    state
        .listing_repo
        .save(&listing, ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing save (submit) failed");
            from_listing_repo_error(&e)
        })?;

    Ok(Json(TransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

/// `POST /listings/:id/revise` — Rejected → Draft (Broker 전용).
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn revise(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
) -> Result<Json<TransitionResponse>, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let mut listing = load_listing_for_actor(&state, &auth, &id).await?;
    listing
        .revise_after_rejection(Utc::now())
        .map_err(|e| from_listing_error(&e))?;

    let ctx = http_user_action(&auth, "revise_listing");
    state
        .listing_repo
        .save(&listing, ctx)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "listing save (revise) failed");
            from_listing_repo_error(&e)
        })?;

    Ok(Json(TransitionResponse {
        id: listing.id.as_str().to_owned(),
        version: listing.version,
        status: listing.status.as_str().to_owned(),
    }))
}

/// 공통 — listing id parse + find + ownership check.
async fn load_listing_for_actor(
    state: &ListingsState,
    auth: &AuthenticatedUser,
    id: &str,
) -> Result<Listing, ProblemResponse> {
    let listing_id = Id::<ListingIdMarker>::from_str(id).map_err(|e| {
        problem(
            "validation",
            "listing id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;
    let listing = state
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
            "본인 매물만 조작할 수 있어요",
            StatusCode::FORBIDDEN,
            None,
        ));
    }
    Ok(listing)
}

// ─────────────────────────────────────────────────────────────────────────
// 사진 (SP6-iv T5) — pre-signed URL pattern, 1차는 mock URL
// ─────────────────────────────────────────────────────────────────────────

/// `POST /listings/:id/photos` 요청 본문.
#[derive(Debug, Deserialize)]
pub struct RequestPhotoUploadRequest {
    /// 표시 순서 (≥0).
    pub display_order: i32,
    /// MIME content-type (`image/jpeg`/`image/png`/`image/webp`).
    pub content_type: String,
    /// 캡션 (≤200자, 선택).
    pub caption: Option<String>,
}

/// `POST /listings/:id/photos` 응답.
#[derive(Debug, Serialize)]
pub struct RequestPhotoUploadResponse {
    /// 새 사진 ID (`lph_<26 ULID>`).
    pub photo_id: String,
    /// pre-signed PUT URL — 1차 mock (SP4-iii-e R2 통합 전).
    pub presigned_put_url: String,
    /// R2 객체 키 (`listings/<lst_id>/<lph_id>.<ext>`).
    pub r2_key: String,
    /// URL 만료 시각 (mock 은 +15분).
    pub expires_at: DateTime<Utc>,
}

/// `POST /listings/:id/photos` — pre-signed URL 발급 (Broker + 소유자 전용).
///
/// 1차 mock: `presigned_put_url = "MOCK://..."`. SP4-iii-e 의 `aws-sdk-s3` 통합
/// 후 실 R2 URL 반환. `ListingPhoto` row 는 *지금* 생성됨 — frontend 가 PUT
/// 성공 시 별도 confirm endpoint 호출은 후속 (FU 49).
#[allow(clippy::too_many_lines)]
#[tracing::instrument(skip(state, auth, body), fields(actor = %auth.user.id, listing_id = %id))]
pub async fn request_photo_upload(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path(id): Path<String>,
    Json(body): Json<RequestPhotoUploadRequest>,
) -> Result<(StatusCode, Json<RequestPhotoUploadResponse>), ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    let listing = load_listing_for_actor(&state, &auth, &id).await?;

    let content_type =
        listing_photo_domain::entity::PhotoContentType::from_str(&body.content_type).map_err(
            |e| {
                problem(
                    "validation",
                    "content_type 가 유효하지 않아요 (image/jpeg, image/png, image/webp)",
                    StatusCode::BAD_REQUEST,
                    Some(e.to_string()),
                )
            },
        )?;

    let photo_id = Id::<ListingPhotoMarker>::new();
    let ext = match content_type {
        listing_photo_domain::entity::PhotoContentType::Jpeg => "jpg",
        listing_photo_domain::entity::PhotoContentType::Png => "png",
        listing_photo_domain::entity::PhotoContentType::Webp => "webp",
    };
    let r2_key = format!(
        "listings/{}/{}.{ext}",
        listing.id.as_str(),
        photo_id.as_str()
    );

    let now = Utc::now();
    let photo = ListingPhoto::try_new(
        photo_id.clone(),
        listing.id.clone(),
        &r2_key,
        None,
        body.caption.as_deref(),
        body.display_order,
        None,
        None,
        None,
        content_type,
        now,
    )
    .map_err(|e| {
        problem(
            "validation",
            "사진 메타가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let ctx = http_user_action(&auth, "request_photo_upload");
    state.photo_repo.save(&photo, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "photo save failed");
        // listing-photo 의 RepoError 와 listing 의 RepoError 는 별개 enum.
        // 매핑은 간단히 재현 (NotFound/Conflict/Database).
        match e {
            listing_photo_domain::repository::RepoError::NotFound => problem(
                "not-found",
                "리소스를 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            ),
            listing_photo_domain::repository::RepoError::Conflict => problem(
                "version-conflict",
                "충돌이 발생했어요",
                StatusCode::CONFLICT,
                None,
            ),
            listing_photo_domain::repository::RepoError::Database(_) => problem(
                "internal-error",
                "내부 서버 오류",
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
        }
    })?;

    // SECURITY/UX: SP4-iii-e R2 통합 전. presigned URL 은 mock — frontend e2e 가
    // 실 PUT 시도 안 함. tracing target=`photo.upload.mock` 으로 후속 R2 통합 시
    // 검색 가능하도록 marker 남김.
    tracing::info!(
        target: "photo.upload.mock",
        photo_id = %photo_id,
        r2_key = %r2_key,
        "issued mock presigned URL (SP4-iii-e pending)"
    );

    Ok((
        StatusCode::CREATED,
        Json(RequestPhotoUploadResponse {
            photo_id: photo_id.as_str().to_owned(),
            presigned_put_url: format!("MOCK://r2/{r2_key}"),
            r2_key,
            expires_at: now + chrono::Duration::minutes(15),
        }),
    ))
}

/// `DELETE /listings/:id/photos/:photo_id` — soft-delete.
#[tracing::instrument(skip(state, auth), fields(actor = %auth.user.id, listing_id = %listing_id, photo_id = %photo_id))]
pub async fn delete_photo(
    State(state): State<ListingsState>,
    Extension(auth): Extension<AuthenticatedUser>,
    Path((listing_id, photo_id)): Path<(String, String)>,
) -> Result<StatusCode, ProblemResponse> {
    require_role(&auth, UserRole::Broker).map_err(|_| {
        problem(
            "forbidden",
            "broker 권한이 필요해요",
            StatusCode::FORBIDDEN,
            None,
        )
    })?;

    // 매물 ownership 검증 (사진 단독 ownership 컬럼은 없음 — listing 으로 추적).
    let _listing = load_listing_for_actor(&state, &auth, &listing_id).await?;

    let pid = Id::<ListingPhotoMarker>::from_str(&photo_id).map_err(|e| {
        problem(
            "validation",
            "photo_id 가 유효하지 않아요",
            StatusCode::BAD_REQUEST,
            Some(e.to_string()),
        )
    })?;

    let ctx = http_user_action(&auth, "delete_photo");
    state.photo_repo.delete(&pid, ctx).await.map_err(|e| {
        tracing::error!(error = %e, "photo delete failed");
        match e {
            listing_photo_domain::repository::RepoError::NotFound => problem(
                "not-found",
                "사진을 찾을 수 없어요",
                StatusCode::NOT_FOUND,
                None,
            ),
            listing_photo_domain::repository::RepoError::Conflict
            | listing_photo_domain::repository::RepoError::Database(_) => problem(
                "internal-error",
                "내부 서버 오류",
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
            ),
        }
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// ─────────────────────────────────────────────────────────────────────────
// SP6-iii: GET /listings/:id 매물 상세
// ─────────────────────────────────────────────────────────────────────────

/// 매물 상세 응답.
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
    /// 좌표 (위/경도).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geom_point: Option<PointResponse>,
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

/// 좌표 응답.
#[derive(Debug, Serialize)]
pub struct PointResponse {
    /// 위도.
    pub lat: f64,
    /// 경도.
    pub lng: f64,
}

/// 사진 응답.
#[derive(Debug, Serialize)]
pub struct PhotoResponse {
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
/// 본인 매물 아니면 view_count 1 증가 (best-effort).
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
fn detail_to_response(
    detail: listing_domain::repository::ListingDetail,
) -> ListingDetailResponse {
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
        geom_point: l.geom_point.map(|p| PointResponse {
            lat: p.lat,
            lng: p.lng,
        }),
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
                r2_key: p.r2_key,
                thumbnail_r2_key: p.thumbnail_r2_key,
                caption: p.caption,
                display_order: p.display_order,
                content_type: p.content_type,
            })
            .collect(),
    }
}
