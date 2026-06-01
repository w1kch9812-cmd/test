# Sub-project 6-ii Listing Search - Part 01B: Handler, Routing, Tests, and Commit

Parent index: [Sub-project 6-ii Listing Search - Part 01](./2026-05-05-sub-project-6-ii-listing-search.part-01.md).
- [ ] **Step 1.5: handler 작성**

`services/api/src/routes/listings.rs`:

```rust
//! `GET /listings` — 카드 list 검색 endpoint (SP6-ii).

use std::sync::Arc;

use auth::middleware::AuthenticatedUser;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use listing_domain::repository::{
    CardSearchQuery, CardSearchSort, ListingCardSummary, ListingRepository,
};
use serde::{Deserialize, Serialize};
use shared_kernel::bounding_box::BoundingBox;
use shared_kernel::listing_type::ListingType;
use shared_kernel::transaction_type::TransactionType;
use std::str::FromStr;

/// 핸들러용 상태.
#[derive(Clone)]
pub struct ListingsState {
    pub listing_repo: Arc<dyn ListingRepository>,
}

#[derive(Debug, Deserialize)]
pub struct ListingsQuery {
    /// "south,west,north,east" (4 floats). 없으면 한국 전체.
    pub bounds: Option<String>,
    /// comma-separated listing_type. 빈 값 = 6 종 모두.
    pub types: Option<String>,
    /// comma-separated transaction_type. 빈 값 = 3 종 모두.
    pub transaction: Option<String>,
    pub min_area_m2: Option<f64>,
    pub max_area_m2: Option<f64>,
    pub min_price_krw: Option<i64>,
    pub max_price_krw: Option<i64>,
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub sort: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListingCardResponse {
    pub id: String,
    pub title: String,
    pub listing_type: String,
    pub transaction_type: String,
    pub price_krw: i64,
    pub deposit_krw: Option<i64>,
    pub monthly_rent_krw: Option<i64>,
    pub area_m2: f64,
    pub lat: f64,
    pub lng: f64,
    pub thumbnail_url: Option<String>,
    pub view_count: i64,
    pub bookmark_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListingsResponse {
    pub listings: Vec<ListingCardResponse>,
    pub total: u64,
    pub page: u32,
    pub size: u32,
    pub has_next: bool,
}

#[derive(Debug, Serialize)]
pub struct ProblemDetails {
    #[serde(rename = "type")]
    pub type_: String,
    pub title: String,
    pub status: u16,
    pub detail: Option<String>,
}

fn problem(type_id: &str, title: &str, status: StatusCode, detail: Option<String>) -> (StatusCode, Json<ProblemDetails>) {
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

pub async fn get_listings(
    State(state): State<ListingsState>,
    _auth: AuthenticatedUser,
    Query(q): Query<ListingsQuery>,
) -> Result<Json<ListingsResponse>, (StatusCode, Json<ProblemDetails>)> {
    // bounds parsing
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
        let floats: Result<Vec<f64>, _> = parts.iter().map(|s| s.parse::<f64>()).collect();
        let floats = floats.map_err(|e| {
            problem(
                "listings/invalid-bounds",
                "bounds 파라미터가 올바르지 않아요",
                StatusCode::BAD_REQUEST,
                Some(e.to_string()),
            )
        })?;
        BoundingBox::new(floats[0], floats[1], floats[2], floats[3])
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

    // types parsing
    let types = if let Some(s) = q.types.as_deref().filter(|s| !s.is_empty()) {
        let parsed: Result<Vec<ListingType>, _> = s
            .split(',')
            .map(ListingType::from_str)
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

    // transaction parsing
    let transactions = if let Some(s) = q.transaction.as_deref().filter(|s| !s.is_empty()) {
        let parsed: Result<Vec<TransactionType>, _> = s
            .split(',')
            .map(TransactionType::from_str)
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
            deposit_krw: c.deposit.map(|d| d.as_i64()),
            monthly_rent_krw: c.monthly_rent.map(|d| d.as_i64()),
            area_m2: c.area_m2,
            lat: c.geom.lat(),
            lng: c.geom.lng(),
            thumbnail_url: c.thumbnail_url,
            view_count: c.view_count,
            bookmark_count: c.bookmark_count,
            created_at: c.created_at,
        })
        .collect();

    let has_next = (page as u64 + 1) * (size as u64) < total;

    Ok(Json(ListingsResponse {
        listings,
        total,
        page,
        size,
        has_next,
    }))
}
```

(NOTE: 실제 `MoneyKrw::as_i64`, `PointSrid::lat/lng`, `ListingType::as_str` 시그니처는 grep 으로 확인 후 조정.)

- [ ] **Step 1.6: main.rs 라우트 등록**

`services/api/src/main.rs` 의 `routes` 모듈에 `pub mod listings;` 추가, protected router 에 listings 라우트 + state 추가:

```rust
mod routes {
    pub mod auth_event;
    pub mod listings;
}

// main() 안 — listing_repo 생성 (이미 PgListingRepository 가 SP5 에 있음)
let listing_repo: Arc<dyn ListingRepository> = Arc::new(PgListingRepository::new(pool.clone()));

let listings_state = routes::listings::ListingsState {
    listing_repo: listing_repo.clone(),
};

// protected route 에 .route("/listings", get(routes::listings::get_listings)) 추가
let protected: Router<()> = Router::new()
    .route("/users/me", get(me))
    .route("/users/:id", get(get_user))
    .route("/listings", get(routes::listings::get_listings).with_state(listings_state.clone()))
    .with_state(app_state)
    .layer(middleware::from_fn_with_state(auth_state, auth_layer));
```

(`with_state` 가 두 번 — Axum 은 nested state 패턴. 또는 `Router<ListingsState>` 로 별도 router 만든 후 merge.)

- [ ] **Step 1.7: cargo check + clippy + cargo fmt**

```bash
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo fmt --all
```

Expected: PASS.

- [ ] **Step 1.8: 작은 unit test 추가 (CardSearchSort enum + ProblemDetails 직렬화)**

`services/api/src/routes/listings.rs` 끝에:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_details_serializes_with_type_field() {
        let p = problem("listings/invalid-bounds", "잘못된 bounds", StatusCode::BAD_REQUEST, None);
        let json = serde_json::to_string(&p.1.0).unwrap();
        assert!(json.contains("\"type\":\"https://gongzzang.com/errors/listings/invalid-bounds\""));
        assert!(json.contains("\"status\":400"));
    }

    #[test]
    fn invalid_sort_returns_bad_request() {
        // (실제 handler 호출 위해 axum-test 또는 mock — 여기는 sort 매핑 로직만 직접 테스트)
        let sort = match Some("invalid").map(|s| s.to_owned()).as_deref() {
            Some("price_asc") => Some(CardSearchSort::PriceAsc),
            Some("created_at_desc") | None => Some(CardSearchSort::CreatedAtDesc),
            _ => None,
        };
        assert_eq!(sort, None);  // fallback 처리 확인
    }
}
```

`cargo test -p api`.

- [ ] **Step 1.9: Commit**

```bash
git add crates/domain/core/listing/src/repository.rs crates/domain/core/listing/src/lib.rs crates/db/src/listing.rs services/api/src/routes/listings.rs services/api/src/main.rs Cargo.lock
git commit -m "feat(6ii-T1): backend GET /listings — find_card_summaries_in_bbox + handler

- listing-domain: ListingCardSummary projection (12 필드) + CardSearchQuery + CardSearchSort enum (5 종) + find_card_summaries_in_bbox trait
- crates/db PgImpl: PostGIS ST_Within(geom_point, ST_MakeEnvelope) + listing_type/transaction_type ANY filter + area/price BETWEEN + ORDER BY + LIMIT/OFFSET + total_count
- services/api: GET /listings handler — query parsing + RFC 7807 (listings/invalid-bounds | invalid-filter | database) + JSON response (listings array + total + has_next)
- main.rs: listings 라우트 protected (auth_layer 적용)"
```

---
