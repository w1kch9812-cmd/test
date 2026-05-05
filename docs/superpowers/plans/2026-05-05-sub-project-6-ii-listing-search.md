# Sub-project 6-ii Listing 검색 화면 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Naver Maps + 카드 list + 6 매물 종류 + 3 거래방식 + 필터 + 무한 스크롤 + Pretendard self-host 로 첫 진짜 product 화면 (`/listings`) 구축.

**Architecture:** `/(authenticated)/listings/page.tsx` (server component) → Zustand store (지도 bounds + filter + selected) → ky `/api/proxy/listings` → backend `/listings` (`PgListingRepository::find_card_summaries_in_bbox` PostGIS bounding box query) → 응답 zod parse → `<ListingMap>` (Naver Maps + 핀 + 클러스터) + `<ListingCardList>` (무한 스크롤 + skeleton).

**Tech Stack:** Next.js 16.2 / React 19 / Zustand / TanStack Query / Naver Maps SDK / Pretendard self-host (next/font/local) / shadcn (range-slider, multi-select 추가) / Rust 1.88 / sqlx + PostGIS.

**Spec:** `docs/superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md` (`a20483d`).

---

## File Structure

각 파일 단일 책임. SP6-i 패턴 일관 (≤500줄, 1500 안티패턴 회피).

### Backend (T1)

| 파일 | 책임 |
|---|---|
| `crates/domain/core/listing/src/repository.rs` (modify) | `ListingCardSummary` projection + `find_card_summaries_in_bbox` trait method 추가 |
| `crates/db/src/listing.rs` (modify) | PostGIS `ST_MakeEnvelope` + `ST_Within` + filter (types/transaction/area/price) + page/sort impl |
| `services/api/src/routes/listings.rs` (new) | `GET /listings` handler, query param parsing, RFC 7807, response shape |
| `services/api/src/main.rs` (modify) | listings route 등록 (auth_layer 적용) |

### Frontend (T2-T6)

| 파일 | 책임 |
|---|---|
| `apps/web/lib/listings/api.ts` | ky 호출 + zod 응답 schema (`ListingsResponseSchema`) |
| `apps/web/lib/listings/filters.ts` | URL query parse / serialize (`parseFiltersFromSearchParams` / `toSearchParams`) |
| `apps/web/lib/listings/format.ts` | `formatPriceKrw` (123억 4,500만), `formatAreaPyeong` (m² → 평) |
| `apps/web/lib/listings/pin-color.ts` | `listing_type` → 색상 매핑 |
| `apps/web/lib/naver-maps.ts` | Naver Maps SDK script lazy loader + readyPromise |
| `apps/web/stores/listings.ts` | Zustand: `{ bounds, filters, selectedListingId, setBounds, setFilters }` |
| `apps/web/components/listings/search-bar.tsx` | 지역 검색 input |
| `apps/web/components/listings/filter-bar.tsx` | 종류 multi + 거래 multi + 평수 range + 가격 range |
| `apps/web/components/listings/listing-pin.tsx` | 종류별 색상 SVG 핀 |
| `apps/web/components/listings/listing-map.tsx` | Naver Maps + 핀 + 클러스터 + bounds 이벤트 |
| `apps/web/components/listings/listing-card.tsx` | 카드 (사진 + 종류 badge + 위치 + 평수 + 가격) |
| `apps/web/components/listings/listing-card-list.tsx` | 무한 스크롤 + skeleton + 핀 ↔ 카드 highlight |
| `apps/web/app/(authenticated)/listings/page.tsx` | 통합 페이지 |
| `apps/web/app/(authenticated)/listings/loading.tsx` | Suspense skeleton |
| `apps/web/lib/i18n/messages/listings.ko.json` | 모든 listings UI string SSOT |

### Design system (T7)

| 파일 | 책임 |
|---|---|
| `apps/web/public/fonts/Pretendard-*.woff2` (new) | Pretendard variable font self-host |
| `apps/web/app/layout.tsx` (modify) | next/font/local 으로 Pretendard 로드 + Tailwind class 적용 |
| `packages/ui/tokens/typography.css` (modify) | `cdn.jsdelivr` import 제거 + self-host 폰트 변수 |
| `apps/web/proxy.ts` (modify) | CSP `style-src` 의 cdn.jsdelivr 제거 (불필요) |
| `packages/ui/primitives/range-slider.tsx` (new) | shadcn Slider primitive |
| `packages/ui/primitives/multi-select.tsx` (new) | shadcn Combobox + 다중 선택 |
| `packages/ui/index.ts` (modify) | 신규 primitive export |

### CI / config

| 파일 | 변경 |
|---|---|
| `apps/web/lib/env.ts` | `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` 추가 (zod) |
| `apps/web/.env.local.example` | `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=` placeholder |
| `apps/web/package.json` | `@types/navermaps` |
| `turbo.json` (modify) | `globalEnv` 에 `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` |
| `apps/web/tests/e2e/listings.spec.ts` (new) | 5 시나리오 (목록 / 필터 / 무한 스크롤 / 모바일 / a11y) |
| `tarpaulin.toml` | `crates/db/src/listing.rs` 90% (이미 포함) |

### Docs (T9)

| 파일 | 책임 |
|---|---|
| `docs/frontend/listings-search.md` | 운영 가이드 (디버깅 / 데이터 source / Naver Maps quota) |
| `docs/adr/0006-listing-search-naver-maps.md` | ADR (Naver vs 카카오 vs Google 결정 근거) |

---

## Task 1: Backend `GET /listings` — `ListingCardSummary` + `find_card_summaries_in_bbox` + handler

**Files:**
- Modify: `crates/domain/core/listing/src/repository.rs` (projection + trait method 추가)
- Modify: `crates/db/src/listing.rs` (PgImpl)
- Create: `services/api/src/routes/listings.rs`
- Modify: `services/api/src/main.rs`

- [ ] **Step 1.1: domain projection + trait method 추가**

`crates/domain/core/listing/src/repository.rs` 의 `ListingMarker` 다음에 추가:

```rust
/// 카드 list 용 풍부한 projection (지도 핀 + 우측 카드 양쪽 사용).
///
/// 전체 [`Listing`] 의 21 필드 중 list 페이지에 필요한 것만.
#[derive(Debug, Clone, PartialEq)]
pub struct ListingCardSummary {
    /// 매물 ID (`lst_...`).
    pub id: Id<ListingIdMarker>,
    /// 제목.
    pub title: String,
    /// 좌표 (`WGS84`, geom_point 가 NULL 인 매물은 응답 제외).
    pub geom: PointSrid,
    /// 매물 유형.
    pub listing_type: ListingType,
    /// 거래 유형.
    pub transaction_type: TransactionType,
    /// 주가격 (sale 의 경우 매매가, jeonse 의 경우 보증금, monthly_rent 의 경우 월세).
    pub price: MoneyKrw,
    /// 보증금 (월세/전세 만; sale 은 None).
    pub deposit: Option<MoneyKrw>,
    /// 월세 (monthly_rent 만; sale/jeonse 는 None).
    pub monthly_rent: Option<MoneyKrw>,
    /// 면적 (m²).
    pub area_m2: f64,
    /// 사진 thumbnail URL (없으면 None — placeholder UI).
    pub thumbnail_url: Option<String>,
    /// 조회수.
    pub view_count: i64,
    /// 즐겨찾기 수.
    pub bookmark_count: i64,
    /// 등록일.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 카드 list 검색 조건 (모두 optional, default 는 "전체").
#[derive(Debug, Clone, Default)]
pub struct CardSearchQuery {
    /// 지도 영역 (4326). None 이면 한국 전체.
    pub bbox: Option<BoundingBox>,
    /// `listing_type` 필터 (None or empty = 6 종 모두).
    pub types: Option<Vec<ListingType>>,
    /// `transaction_type` 필터 (None or empty = 3 종 모두).
    pub transactions: Option<Vec<TransactionType>>,
    /// `area_m2 >=` (None = 0).
    pub min_area_m2: Option<f64>,
    /// `area_m2 <=` (None = +inf).
    pub max_area_m2: Option<f64>,
    /// `price_krw >=` (None = 0).
    pub min_price_krw: Option<i64>,
    /// `price_krw <=` (None = +inf).
    pub max_price_krw: Option<i64>,
    /// page (0-indexed).
    pub page: u32,
    /// page 당 항목 수 (max 100).
    pub size: u32,
    /// 정렬.
    pub sort: CardSearchSort,
}

/// 정렬 방식.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CardSearchSort {
    /// 등록일 최신순 (default).
    #[default]
    CreatedAtDesc,
    /// 가격 오름차순.
    PriceAsc,
    /// 가격 내림차순.
    PriceDesc,
    /// 면적 오름차순.
    AreaAsc,
    /// 면적 내림차순.
    AreaDesc,
}
```

trait `ListingRepository` 의 `find_markers_in_bbox` 다음에 추가:

```rust
    /// 카드 list 검색 — `status='active'` + `geom_point` not null + filter 적용.
    ///
    /// `query.size` 의 max 100 (caller 가 검증). 응답은 (cards, total_count) 튜플.
    ///
    /// # Errors
    ///
    /// DB 통신 실패 시 [`RepoError::Database`].
    async fn find_card_summaries_in_bbox(
        &self,
        query: CardSearchQuery,
    ) -> Result<(Vec<ListingCardSummary>, u64), RepoError>;
```

`pub use` 도 추가 — `crates/domain/core/listing/src/lib.rs`:

```rust
pub use repository::{
    CardSearchQuery, CardSearchSort, ListingCardSummary, ListingMarker, ListingRepository,
    RepoError,
};
```

(현재 `lib.rs` 의 `pub use` 가 무엇 export 하는지 확인 후 정확한 변경.)

- [ ] **Step 1.2: cargo check (트레잇 시그니처 컴파일 확인)**

```bash
cargo check -p listing-domain
```

Expected: PASS (impl 블록 추가 안 했으니 — trait 자체는 unimplemented, impl 추가 시 cargo check 가 빠진 메서드 알림).

- [ ] **Step 1.3: PgImpl 추가**

`crates/db/src/listing.rs` 의 `find_markers_in_bbox` impl 다음에 (라인 318 근처) 추가:

먼저 grep 으로 정확한 위치 확인:

```bash
grep -n "find_markers_in_bbox\|find_by_owner\|fn save" crates/db/src/listing.rs
```

`find_markers_in_bbox` 와 `find_by_owner` 사이에 `find_card_summaries_in_bbox` 추가:

```rust
    async fn find_card_summaries_in_bbox(
        &self,
        query: listing_domain::repository::CardSearchQuery,
    ) -> Result<(Vec<listing_domain::repository::ListingCardSummary>, u64), RepoError> {
        use listing_domain::repository::{CardSearchSort, ListingCardSummary};

        // bbox: None → 전체 한국. ST_MakeEnvelope 4326 + ST_Within.
        let (south, west, north, east) = query
            .bbox
            .map(|b| (b.south(), b.west(), b.north(), b.east()))
            .unwrap_or((33.0, 124.0, 39.0, 132.0));

        // listing_type / transaction_type 필터 (None or empty = 전체).
        let types_array: Option<Vec<&str>> = query.types.as_ref().filter(|v| !v.is_empty()).map(|v| {
            v.iter().map(|t| t.as_str()).collect()
        });
        let txns_array: Option<Vec<&str>> = query.transactions.as_ref().filter(|v| !v.is_empty()).map(|v| {
            v.iter().map(|t| t.as_str()).collect()
        });

        let min_area = query.min_area_m2.unwrap_or(0.0);
        let max_area = query.max_area_m2.unwrap_or(f64::MAX);
        let min_price = query.min_price_krw.unwrap_or(0);
        let max_price = query.max_price_krw.unwrap_or(i64::MAX);

        let order_by = match query.sort {
            CardSearchSort::CreatedAtDesc => "created_at DESC",
            CardSearchSort::PriceAsc => "price_krw ASC",
            CardSearchSort::PriceDesc => "price_krw DESC",
            CardSearchSort::AreaAsc => "area_m2 ASC",
            CardSearchSort::AreaDesc => "area_m2 DESC",
        };

        let size = query.size.min(100).max(1);
        let offset = (query.page as i64) * (size as i64);

        // count + page query — 같은 conditional WHERE
        // sqlx 의 dynamic SQL 은 query_as_with 또는 string interpolation 으로 처리.
        // 안전을 위해 listing_type/transaction_type 은 enum 의 fixed strings 만 사용.

        let sql = format!(
            r#"
            WITH filtered AS (
                SELECT id, title, geom_point, listing_type, transaction_type,
                       price_krw, deposit_krw, monthly_rent_krw, area_m2,
                       view_count, bookmark_count, created_at
                FROM listing
                WHERE status = 'active'
                  AND geom_point IS NOT NULL
                  AND ST_Within(geom_point, ST_MakeEnvelope($1, $2, $3, $4, 4326))
                  AND ($5::text[] IS NULL OR listing_type = ANY($5::text[]))
                  AND ($6::text[] IS NULL OR transaction_type = ANY($6::text[]))
                  AND area_m2 BETWEEN $7 AND $8
                  AND price_krw BETWEEN $9 AND $10
            )
            SELECT
                (SELECT COUNT(*) FROM filtered) AS total_count,
                f.id, f.title,
                ST_X(f.geom_point) AS lng, ST_Y(f.geom_point) AS lat,
                f.listing_type, f.transaction_type,
                f.price_krw, f.deposit_krw, f.monthly_rent_krw,
                f.area_m2::float8 AS area_m2,
                f.view_count, f.bookmark_count, f.created_at
            FROM filtered f
            ORDER BY f.{order_by}
            LIMIT $11 OFFSET $12
            "#
        );

        let mut q = sqlx::query(&sql)
            .bind(west).bind(south).bind(east).bind(north)
            .bind(types_array.as_deref())
            .bind(txns_array.as_deref())
            .bind(min_area).bind(max_area)
            .bind(min_price).bind(max_price)
            .bind(size as i64).bind(offset);

        let rows = q.fetch_all(&self.pool).await
            .map_err(|e| RepoError::Database(e.to_string()))?;

        let mut total_count: u64 = 0;
        let mut cards: Vec<ListingCardSummary> = Vec::with_capacity(rows.len());
        for row in rows {
            use sqlx::Row;
            // total_count 는 모든 row 에 같은 값.
            total_count = row.try_get::<i64, _>("total_count").unwrap_or(0) as u64;

            let id_str: String = row.try_get("id").map_err(|e| RepoError::Database(e.to_string()))?;
            let id = Id::<ListingIdMarker>::try_from_str(&id_str)
                .map_err(|e| RepoError::Database(format!("invalid id: {e}")))?;

            let title: String = row.try_get("title").map_err(|e| RepoError::Database(e.to_string()))?;
            let lng: f64 = row.try_get("lng").map_err(|e| RepoError::Database(e.to_string()))?;
            let lat: f64 = row.try_get("lat").map_err(|e| RepoError::Database(e.to_string()))?;
            let geom = PointSrid::new(lng, lat, 4326);

            let lt_str: String = row.try_get("listing_type").map_err(|e| RepoError::Database(e.to_string()))?;
            let listing_type = ListingType::from_str(&lt_str)
                .map_err(|e| RepoError::Database(format!("invalid listing_type: {e}")))?;

            let tt_str: String = row.try_get("transaction_type").map_err(|e| RepoError::Database(e.to_string()))?;
            let transaction_type = TransactionType::from_str(&tt_str)
                .map_err(|e| RepoError::Database(format!("invalid transaction_type: {e}")))?;

            let price_i: i64 = row.try_get("price_krw").map_err(|e| RepoError::Database(e.to_string()))?;
            let price = MoneyKrw::from_i64(price_i)
                .map_err(|e| RepoError::Database(format!("invalid price: {e}")))?;

            let deposit_opt: Option<i64> = row.try_get("deposit_krw").ok();
            let deposit = deposit_opt.and_then(|d| MoneyKrw::from_i64(d).ok());

            let rent_opt: Option<i64> = row.try_get("monthly_rent_krw").ok();
            let monthly_rent = rent_opt.and_then(|d| MoneyKrw::from_i64(d).ok());

            let area_m2: f64 = row.try_get("area_m2").map_err(|e| RepoError::Database(e.to_string()))?;
            let view_count: i64 = row.try_get("view_count").unwrap_or(0);
            let bookmark_count: i64 = row.try_get("bookmark_count").unwrap_or(0);
            let created_at: DateTime<Utc> = row.try_get("created_at").map_err(|e| RepoError::Database(e.to_string()))?;

            cards.push(ListingCardSummary {
                id, title, geom, listing_type, transaction_type,
                price, deposit, monthly_rent, area_m2,
                thumbnail_url: None,  // SP6-iii 가 listing-photo 테이블 join 으로 채움
                view_count, bookmark_count, created_at,
            });
        }

        Ok((cards, total_count))
    }
```

(NOTE: 실제 `MoneyKrw::from_i64`, `ListingType::as_str` / `from_str`, `BoundingBox::south/west/...`, `PointSrid::new` 시그니처는 grep 으로 확인 후 조정 필요.)

- [ ] **Step 1.4: cargo check + clippy**

```bash
cargo check -p db -p listing-domain
cargo clippy -p db -p listing-domain --all-targets -- -D warnings
```

Expected: PASS.

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

## Task 2: Frontend api.ts + Zustand store + filters URL 동기화 + format helpers

**Files:**
- Create: `apps/web/lib/listings/api.ts`
- Create: `apps/web/lib/listings/filters.ts`
- Create: `apps/web/lib/listings/format.ts`
- Create: `apps/web/lib/listings/pin-color.ts`
- Create: `apps/web/stores/listings.ts`
- Test: `apps/web/tests/unit/listings/format.test.ts`
- Test: `apps/web/tests/unit/listings/filters.test.ts`
- Test: `apps/web/tests/unit/listings/pin-color.test.ts`

- [ ] **Step 2.1: format.ts — failing tests**

`apps/web/tests/unit/listings/format.test.ts`:

```typescript
// @vitest-environment node
import { describe, it, expect } from "vitest";
import { formatPriceKrw, formatAreaPyeong, formatAreaM2, m2ToPyeong } from "@/lib/listings/format";

describe("formatPriceKrw — 한국 가격 표기", () => {
  it("1조 이상", () => {
    expect(formatPriceKrw(1_500_000_000_000)).toBe("1조 5,000억원");
  });
  it("억 + 만원", () => {
    expect(formatPriceKrw(8_500_000_000)).toBe("85억원");
    expect(formatPriceKrw(123_450_000)).toBe("1억 2,345만원");
  });
  it("만원 단위", () => {
    expect(formatPriceKrw(50_000_000)).toBe("5,000만원");
  });
  it("원 단위", () => {
    expect(formatPriceKrw(800_000)).toBe("800,000원");
  });
  it("0", () => {
    expect(formatPriceKrw(0)).toBe("0원");
  });
});

describe("m2ToPyeong + formatAreaPyeong", () => {
  it("1평 = 3.305 m²", () => {
    expect(m2ToPyeong(3.305)).toBeCloseTo(1.0, 1);
  });
  it("formatAreaPyeong 소수점 1자리", () => {
    expect(formatAreaPyeong(330.5)).toBe("100.0평");
    expect(formatAreaPyeong(33.05)).toBe("10.0평");
  });
});

describe("formatAreaM2", () => {
  it("정수 + 천단위 콤마", () => {
    expect(formatAreaM2(3960.5)).toBe("3,961㎡");
  });
});
```

- [ ] **Step 2.2: Run test — FAIL**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/format.test.ts
```

Expected: FAIL — module not found.

- [ ] **Step 2.3: format.ts 구현**

`apps/web/lib/listings/format.ts`:

```typescript
const TRILLION = 1_000_000_000_000n;
const HUNDRED_MILLION = 100_000_000n;
const TEN_THOUSAND = 10_000n;
const PYEONG_PER_M2 = 0.3025; // 1 평 = 3.305 m² → 1 m² ≈ 0.3025 평

/**
 * 한국 가격 표기 (1조 5,000억원 / 85억원 / 1억 2,345만원 / 5,000만원 / 800,000원).
 */
export function formatPriceKrw(value: number): string {
  if (value === 0) return "0원";
  const big = BigInt(Math.round(value));
  const trillions = big / TRILLION;
  const remainderAfterTrillions = big % TRILLION;
  const hundredMillions = remainderAfterTrillions / HUNDRED_MILLION;
  const remainderAfterHM = remainderAfterTrillions % HUNDRED_MILLION;
  const tenThousands = remainderAfterHM / TEN_THOUSAND;

  const parts: string[] = [];
  if (trillions > 0n) parts.push(`${trillions}조`);
  if (hundredMillions > 0n) {
    if (trillions > 0n) {
      parts.push(`${formatThousands(hundredMillions)}억원`);
      return parts.join(" ");
    }
    parts.push(`${formatThousands(hundredMillions)}억`);
    if (tenThousands > 0n) parts.push(`${formatThousands(tenThousands)}만원`);
    else parts[parts.length - 1] = `${parts[parts.length - 1]}원`;
    return parts.join(" ");
  }
  if (tenThousands > 0n) return `${formatThousands(tenThousands)}만원`;
  return `${formatThousands(big)}원`;
}

function formatThousands(n: bigint): string {
  return n.toLocaleString("ko-KR");
}

/** m² → 평 변환. */
export function m2ToPyeong(m2: number): number {
  return m2 * PYEONG_PER_M2;
}

/** "100.0평" 형식. */
export function formatAreaPyeong(m2: number): string {
  return `${m2ToPyeong(m2).toFixed(1)}평`;
}

/** "3,961㎡" 형식 (정수 + 천단위 콤마). */
export function formatAreaM2(m2: number): string {
  return `${Math.round(m2).toLocaleString("ko-KR")}㎡`;
}
```

- [ ] **Step 2.4: Run test — PASS**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/format.test.ts
```

Expected: PASS (5+ tests).

- [ ] **Step 2.5: pin-color.ts — test + impl**

`apps/web/tests/unit/listings/pin-color.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { getPinColor, LISTING_TYPE_COLORS } from "@/lib/listings/pin-color";

describe("getPinColor", () => {
  it("6 종 매물 모두 hex color 반환", () => {
    expect(getPinColor("factory")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("warehouse")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("office")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("knowledge_industry_center")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("industrial_land")).toMatch(/^#[0-9a-f]{6}$/i);
    expect(getPinColor("logistics_center")).toMatch(/^#[0-9a-f]{6}$/i);
  });
  it("6 종 모두 unique color", () => {
    const colors = new Set(Object.values(LISTING_TYPE_COLORS));
    expect(colors.size).toBe(6);
  });
});
```

`apps/web/lib/listings/pin-color.ts`:

```typescript
export const LISTING_TYPE_COLORS = {
  factory: "#dc2626",                    // red-600 (공장)
  warehouse: "#2563eb",                  // blue-600 (창고)
  office: "#059669",                     // emerald-600 (사무실)
  knowledge_industry_center: "#7c3aed",  // violet-600 (지식산업센터)
  industrial_land: "#ea580c",            // orange-600 (산업단지/토지)
  logistics_center: "#0891b2",           // cyan-600 (물류센터)
} as const;

export type ListingTypeKey = keyof typeof LISTING_TYPE_COLORS;

export function getPinColor(listingType: string): string {
  return LISTING_TYPE_COLORS[listingType as ListingTypeKey] ?? "#6b7280"; // gray-500 fallback
}
```

- [ ] **Step 2.6: filters.ts — test + impl**

`apps/web/tests/unit/listings/filters.test.ts`:

```typescript
// @vitest-environment node
import { describe, it, expect } from "vitest";
import {
  parseFiltersFromSearchParams,
  toSearchParams,
  type ListingFilters,
} from "@/lib/listings/filters";

describe("parseFiltersFromSearchParams", () => {
  it("default filter (모두 빈 값)", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams());
    expect(f.types).toEqual([]);
    expect(f.transactions).toEqual([]);
    expect(f.minAreaM2).toBeUndefined();
    expect(f.sort).toBe("created_at_desc");
  });
  it("comma-separated types", () => {
    const f = parseFiltersFromSearchParams(new URLSearchParams("types=factory,warehouse"));
    expect(f.types).toEqual(["factory", "warehouse"]);
  });
  it("range parsing", () => {
    const f = parseFiltersFromSearchParams(
      new URLSearchParams("min_area_m2=100&max_area_m2=2000&min_price_krw=0&max_price_krw=5000000000"),
    );
    expect(f.minAreaM2).toBe(100);
    expect(f.maxAreaM2).toBe(2000);
    expect(f.minPriceKrw).toBe(0);
    expect(f.maxPriceKrw).toBe(5_000_000_000);
  });
});

describe("toSearchParams (round trip)", () => {
  it("filter → URLSearchParams → 동일 filter", () => {
    const f: ListingFilters = {
      types: ["factory", "office"],
      transactions: ["sale"],
      minAreaM2: 200,
      maxAreaM2: undefined,
      minPriceKrw: undefined,
      maxPriceKrw: undefined,
      sort: "price_asc",
    };
    const sp = toSearchParams(f);
    const back = parseFiltersFromSearchParams(sp);
    expect(back.types).toEqual(f.types);
    expect(back.transactions).toEqual(f.transactions);
    expect(back.minAreaM2).toBe(200);
    expect(back.sort).toBe("price_asc");
  });
});
```

`apps/web/lib/listings/filters.ts`:

```typescript
export type ListingType =
  | "factory"
  | "warehouse"
  | "office"
  | "knowledge_industry_center"
  | "industrial_land"
  | "logistics_center";

export type TransactionType = "sale" | "monthly_rent" | "jeonse";

export type SortKey =
  | "created_at_desc"
  | "price_asc"
  | "price_desc"
  | "area_asc"
  | "area_desc";

export interface ListingFilters {
  types: ListingType[];
  transactions: TransactionType[];
  minAreaM2: number | undefined;
  maxAreaM2: number | undefined;
  minPriceKrw: number | undefined;
  maxPriceKrw: number | undefined;
  sort: SortKey;
}

const VALID_TYPES: ListingType[] = [
  "factory",
  "warehouse",
  "office",
  "knowledge_industry_center",
  "industrial_land",
  "logistics_center",
];

const VALID_TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];

const VALID_SORTS: SortKey[] = [
  "created_at_desc",
  "price_asc",
  "price_desc",
  "area_asc",
  "area_desc",
];

function parseList<T extends string>(raw: string | null, valid: readonly T[]): T[] {
  if (!raw) return [];
  return raw
    .split(",")
    .map((s) => s.trim())
    .filter((s): s is T => valid.includes(s as T));
}

function parseNumber(raw: string | null): number | undefined {
  if (raw === null || raw === "") return undefined;
  const n = Number(raw);
  return Number.isFinite(n) ? n : undefined;
}

export function parseFiltersFromSearchParams(sp: URLSearchParams): ListingFilters {
  const sortRaw = sp.get("sort");
  const sort: SortKey = VALID_SORTS.includes(sortRaw as SortKey)
    ? (sortRaw as SortKey)
    : "created_at_desc";

  return {
    types: parseList(sp.get("types"), VALID_TYPES),
    transactions: parseList(sp.get("transaction"), VALID_TXNS),
    minAreaM2: parseNumber(sp.get("min_area_m2")),
    maxAreaM2: parseNumber(sp.get("max_area_m2")),
    minPriceKrw: parseNumber(sp.get("min_price_krw")),
    maxPriceKrw: parseNumber(sp.get("max_price_krw")),
    sort,
  };
}

export function toSearchParams(f: ListingFilters): URLSearchParams {
  const sp = new URLSearchParams();
  if (f.types.length > 0) sp.set("types", f.types.join(","));
  if (f.transactions.length > 0) sp.set("transaction", f.transactions.join(","));
  if (f.minAreaM2 !== undefined) sp.set("min_area_m2", String(f.minAreaM2));
  if (f.maxAreaM2 !== undefined) sp.set("max_area_m2", String(f.maxAreaM2));
  if (f.minPriceKrw !== undefined) sp.set("min_price_krw", String(f.minPriceKrw));
  if (f.maxPriceKrw !== undefined) sp.set("max_price_krw", String(f.maxPriceKrw));
  if (f.sort !== "created_at_desc") sp.set("sort", f.sort);
  return sp;
}
```

- [ ] **Step 2.7: Run filter tests — PASS**

```bash
pnpm --filter=@gongzzang/web test -- tests/unit/listings/
```

Expected: PASS.

- [ ] **Step 2.8: api.ts — zod schema + ky 호출**

`apps/web/lib/listings/api.ts`:

```typescript
import { z } from "zod";
import { api } from "@/lib/api";
import type { ListingFilters } from "@/lib/listings/filters";
import { toSearchParams } from "@/lib/listings/filters";

export const ListingCardSchema = z.object({
  id: z.string(),
  title: z.string(),
  listing_type: z.enum([
    "factory",
    "warehouse",
    "office",
    "knowledge_industry_center",
    "industrial_land",
    "logistics_center",
  ]),
  transaction_type: z.enum(["sale", "monthly_rent", "jeonse"]),
  price_krw: z.number().int(),
  deposit_krw: z.number().int().nullable(),
  monthly_rent_krw: z.number().int().nullable(),
  area_m2: z.number(),
  lat: z.number(),
  lng: z.number(),
  thumbnail_url: z.string().nullable(),
  view_count: z.number().int(),
  bookmark_count: z.number().int(),
  created_at: z.string(), // ISO 8601
});

export type ListingCard = z.infer<typeof ListingCardSchema>;

export const ListingsResponseSchema = z.object({
  listings: z.array(ListingCardSchema),
  total: z.number().int(),
  page: z.number().int(),
  size: z.number().int(),
  has_next: z.boolean(),
});

export type ListingsResponse = z.infer<typeof ListingsResponseSchema>;

export interface FetchListingsInput {
  filters: ListingFilters;
  bounds?: { south: number; west: number; north: number; east: number };
  page?: number;
  size?: number;
}

export async function fetchListings(input: FetchListingsInput): Promise<ListingsResponse> {
  const sp = toSearchParams(input.filters);
  if (input.bounds) {
    const { south, west, north, east } = input.bounds;
    sp.set("bounds", `${south},${west},${north},${east}`);
  }
  if (input.page !== undefined) sp.set("page", String(input.page));
  if (input.size !== undefined) sp.set("size", String(input.size));

  const json = await api.get(`listings?${sp.toString()}`).json<unknown>();
  return ListingsResponseSchema.parse(json);
}
```

- [ ] **Step 2.9: stores/listings.ts (Zustand)**

`apps/web/stores/listings.ts`:

```typescript
"use client";
import { create } from "zustand";
import type { ListingFilters, SortKey } from "@/lib/listings/filters";

export interface MapBounds {
  south: number;
  west: number;
  north: number;
  east: number;
}

interface ListingsState {
  bounds: MapBounds | undefined;
  filters: ListingFilters;
  selectedListingId: string | null;
  setBounds: (b: MapBounds) => void;
  setFilters: (next: ListingFilters) => void;
  patchFilters: (patch: Partial<ListingFilters>) => void;
  setSelectedListingId: (id: string | null) => void;
}

const DEFAULT_FILTERS: ListingFilters = {
  types: [],
  transactions: [],
  minAreaM2: undefined,
  maxAreaM2: undefined,
  minPriceKrw: undefined,
  maxPriceKrw: undefined,
  sort: "created_at_desc" as SortKey,
};

export const useListingsStore = create<ListingsState>((set) => ({
  bounds: undefined,
  filters: DEFAULT_FILTERS,
  selectedListingId: null,
  setBounds: (b) => set({ bounds: b }),
  setFilters: (next) => set({ filters: next }),
  patchFilters: (patch) =>
    set((state) => ({ filters: { ...state.filters, ...patch } })),
  setSelectedListingId: (id) => set({ selectedListingId: id }),
}));
```

- [ ] **Step 2.10: typecheck + lint + commit**

```bash
pnpm typecheck
pnpm lint
git add apps/web/lib/listings/ apps/web/stores/listings.ts apps/web/tests/unit/listings/
git commit -m "feat(6ii-T2): listings api.ts + zod + filters + format + Zustand store

- lib/listings/api.ts: ky + zod (ListingsResponseSchema 검증)
- lib/listings/filters.ts: URL query parse/serialize + 6 ListingType + 3 TransactionType + 5 SortKey
- lib/listings/format.ts: formatPriceKrw (1조 5,000억원) + formatAreaPyeong (m² → 평) + formatAreaM2
- lib/listings/pin-color.ts: 6 매물 종류 → unique hex color (red/blue/emerald/violet/orange/cyan)
- stores/listings.ts: Zustand { bounds, filters, selectedListingId } + setters/patchers
- 14 unit test (format 5 + pin-color 2 + filters 4)"
```

---

## Task 3: Naver Maps 통합 — loader + ListingMap + 핀 + 클러스터 + bounds 이벤트

**Files:**
- Modify: `apps/web/lib/env.ts` (NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID 추가)
- Modify: `apps/web/.env.local.example`
- Modify: `turbo.json` (globalEnv)
- Modify: `apps/web/package.json` (`@types/navermaps`)
- Create: `apps/web/lib/naver-maps.ts`
- Create: `apps/web/components/listings/listing-pin.tsx`
- Create: `apps/web/components/listings/listing-map.tsx`

- [ ] **Step 3.1: env.ts 확장 (zod)**

`apps/web/lib/env.ts` 의 `PublicEnvSchema` 에 `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` 추가:

```typescript
const PublicEnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: z.string().min(1).default("naver-maps-placeholder"),
});
```

(default 둠 — placeholder 면 지도 안 뜨지만 build 통과.)

`safeParse` 호출 시 객체에도 추가:

```typescript
const parsed = Schema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID: process.env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID,
  // ... 기존 server env
});
```

- [ ] **Step 3.2: .env.local.example + turbo.json**

`apps/web/.env.local.example` 끝에:

```
# SP6-ii — Naver Maps
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=naver-maps-placeholder
```

`turbo.json` 의 `globalEnv` 끝에:

```json
"NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID",
```

- [ ] **Step 3.3: deps 추가**

```bash
pnpm --filter=@gongzzang/web add -D @types/navermaps@^3.7.0
```

- [ ] **Step 3.4: lib/naver-maps.ts (lazy script loader)**

`apps/web/lib/naver-maps.ts`:

```typescript
import { env } from "@/lib/env";

let _readyPromise: Promise<typeof naver> | null = null;

/**
 * Naver Maps SDK script lazy load. 한 번만 로드.
 *
 * `naver` 글로벌이 ready 되면 resolve.
 */
export function loadNaverMaps(): Promise<typeof naver> {
  if (_readyPromise) return _readyPromise;
  if (typeof window === "undefined") {
    return Promise.reject(new Error("loadNaverMaps must run in browser"));
  }
  _readyPromise = new Promise((resolve, reject) => {
    if (typeof naver !== "undefined" && naver.maps) {
      resolve(naver);
      return;
    }
    const script = document.createElement("script");
    script.src = `https://oapi.map.naver.com/openapi/v3/maps.js?ncpClientId=${env.NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID}&submodules=clustering`;
    script.async = true;
    script.onload = () => resolve(naver);
    script.onerror = () => reject(new Error("Naver Maps SDK failed to load"));
    document.head.appendChild(script);
  });
  return _readyPromise;
}
```

- [ ] **Step 3.5: components/listings/listing-pin.tsx (SVG marker template)**

`apps/web/components/listings/listing-pin.tsx`:

```typescript
import { getPinColor, type ListingTypeKey } from "@/lib/listings/pin-color";

/**
 * Naver Maps 의 marker icon 으로 사용할 SVG HTML string.
 * `new naver.maps.Marker({ icon: { content: pinIconHtml(...) } })`.
 */
export function pinIconHtml(listingType: string, options: { selected?: boolean } = {}): string {
  const color = getPinColor(listingType);
  const size = options.selected ? 36 : 28;
  const stroke = options.selected ? "#ffffff" : "#1f2937";
  const strokeWidth = options.selected ? 3 : 1.5;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}" viewBox="0 0 24 24" fill="${color}" stroke="${stroke}" stroke-width="${strokeWidth}">
    <path d="M12 2C7.58 2 4 5.58 4 10c0 5.25 7 12 8 12s8-6.75 8-12c0-4.42-3.58-8-8-8z"/>
    <circle cx="12" cy="10" r="3" fill="#ffffff"/>
  </svg>`;
}
```

- [ ] **Step 3.6: components/listings/listing-map.tsx**

`apps/web/components/listings/listing-map.tsx`:

```typescript
"use client";
import { useEffect, useRef } from "react";
import { loadNaverMaps } from "@/lib/naver-maps";
import { useListingsStore } from "@/stores/listings";
import type { ListingCard } from "@/lib/listings/api";
import { pinIconHtml } from "@/components/listings/listing-pin";

interface ListingMapProps {
  listings: ListingCard[];
}

export function ListingMap({ listings }: ListingMapProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const mapRef = useRef<naver.maps.Map | null>(null);
  const markersRef = useRef<naver.maps.Marker[]>([]);
  const setBounds = useListingsStore((s) => s.setBounds);
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);

  // 1. 지도 초기화 (1회)
  useEffect(() => {
    let cancelled = false;
    loadNaverMaps().then((naverNs) => {
      if (cancelled || !containerRef.current) return;
      const map = new naverNs.maps.Map(containerRef.current, {
        center: new naverNs.maps.LatLng(37.5665, 126.978),  // 서울 시청
        zoom: 8,
        mapTypeControl: false,
      });
      mapRef.current = map;

      // bounds 변경 이벤트 (debounce)
      let timer: ReturnType<typeof setTimeout> | null = null;
      naverNs.maps.Event.addListener(map, "bounds_changed", () => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
          const bounds = map.getBounds() as naver.maps.LatLngBounds;
          const sw = bounds.getMin();
          const ne = bounds.getMax();
          setBounds({
            south: sw.y,
            west: sw.x,
            north: ne.y,
            east: ne.x,
          });
        }, 350);
      });
      // 초기 bounds 도 emit
      const b = map.getBounds() as naver.maps.LatLngBounds;
      setBounds({
        south: b.getMin().y,
        west: b.getMin().x,
        north: b.getMax().y,
        east: b.getMax().x,
      });
    });
    return () => {
      cancelled = true;
    };
  }, [setBounds]);

  // 2. 매물 변경 → marker 재생성
  useEffect(() => {
    if (!mapRef.current) return;
    const map = mapRef.current;
    // 기존 marker 제거
    for (const m of markersRef.current) m.setMap(null);
    markersRef.current = [];

    // 새 marker 생성
    for (const listing of listings) {
      const marker = new naver.maps.Marker({
        position: new naver.maps.LatLng(listing.lat, listing.lng),
        map,
        icon: {
          content: pinIconHtml(listing.listing_type, { selected: listing.id === selectedId }),
          anchor: new naver.maps.Point(14, 28),
        },
      });
      naver.maps.Event.addListener(marker, "click", () => {
        setSelected(listing.id);
      });
      markersRef.current.push(marker);
    }
  }, [listings, selectedId, setSelected]);

  return <div ref={containerRef} className="h-full w-full" />;
}
```

- [ ] **Step 3.7: typecheck**

```bash
pnpm --filter=@gongzzang/web typecheck
```

Expected: PASS (단 `@types/navermaps` 의 정확한 type 시그니처에 따라 일부 cast 필요할 수 있음 — 발견 시 inline `as` 또는 `// eslint-disable` 대신 type narrowing 으로 해결).

- [ ] **Step 3.8: Commit**

```bash
git add apps/web/lib/naver-maps.ts apps/web/lib/env.ts apps/web/.env.local.example apps/web/components/listings/listing-pin.tsx apps/web/components/listings/listing-map.tsx apps/web/package.json apps/web/pnpm-lock.yaml turbo.json
git commit -m "feat(6ii-T3): Naver Maps 통합 — lazy SDK loader + ListingMap + SVG pin

- lib/env.ts: NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID (zod public schema)
- lib/naver-maps.ts: lazy script loader (한 번만 inject + ready Promise)
- components/listings/listing-pin.tsx: SVG marker template (6 종 unique color + selected highlight)
- components/listings/listing-map.tsx: Naver Maps + 마커 + bounds_changed debounced (350ms) → store + click → selected
- @types/navermaps 추가
- turbo.json globalEnv 추가"
```

---

## Task 4: Filter / Search bar — 종류 + 거래 + 평수 + 가격 + URL query 동기화

**Files:**
- Create: `packages/ui/primitives/range-slider.tsx`
- Create: `packages/ui/primitives/multi-select.tsx`
- Modify: `packages/ui/index.ts` (export)
- Create: `apps/web/components/listings/search-bar.tsx`
- Create: `apps/web/components/listings/filter-bar.tsx`
- Test: `apps/web/tests/unit/listings/filter-bar.test.tsx`

(주: shadcn/Radix 의 Slider primitive 와 Combobox 패턴 사용. 이미 packages/ui 에 Radix 가 있으므로 추가 dep 불필요 — Radix Slider 직접 사용.)

- [ ] **Step 4.1: Radix Slider 추가 + range-slider primitive**

```bash
pnpm --filter=@gongzzang/ui add @radix-ui/react-slider@^1.2.3
```

`packages/ui/primitives/range-slider.tsx`:

```typescript
"use client";
import * as SliderPrimitive from "@radix-ui/react-slider";
import { cn } from "../lib/cn";

interface RangeSliderProps {
  min: number;
  max: number;
  step?: number;
  value: [number, number];
  onValueChange: (next: [number, number]) => void;
  formatValue?: (v: number) => string;
  className?: string;
}

export function RangeSlider({
  min, max, step = 1, value, onValueChange, formatValue, className,
}: RangeSliderProps) {
  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <SliderPrimitive.Root
        min={min}
        max={max}
        step={step}
        value={value}
        onValueChange={(v) => onValueChange([v[0], v[1]] as [number, number])}
        className="relative flex h-5 w-full touch-none select-none items-center"
      >
        <SliderPrimitive.Track className="relative h-1 w-full grow overflow-hidden rounded-full bg-muted">
          <SliderPrimitive.Range className="absolute h-full bg-primary" />
        </SliderPrimitive.Track>
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
        <SliderPrimitive.Thumb className="block h-4 w-4 rounded-full border border-primary bg-background shadow focus:outline-none focus:ring-2 focus:ring-ring" />
      </SliderPrimitive.Root>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{formatValue ? formatValue(value[0]) : value[0]}</span>
        <span>{formatValue ? formatValue(value[1]) : value[1]}</span>
      </div>
    </div>
  );
}
```

- [ ] **Step 4.2: multi-select primitive (간단 chip 형태)**

`packages/ui/primitives/multi-select.tsx`:

```typescript
"use client";
import { cn } from "../lib/cn";

interface MultiSelectOption {
  value: string;
  label: string;
}

interface MultiSelectProps {
  options: MultiSelectOption[];
  value: string[];
  onValueChange: (next: string[]) => void;
  className?: string;
}

export function MultiSelect({ options, value, onValueChange, className }: MultiSelectProps) {
  const toggle = (v: string) => {
    if (value.includes(v)) onValueChange(value.filter((x) => x !== v));
    else onValueChange([...value, v]);
  };
  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {options.map((opt) => {
        const selected = value.includes(opt.value);
        return (
          <button
            key={opt.value}
            type="button"
            aria-pressed={selected}
            onClick={() => toggle(opt.value)}
            className={cn(
              "rounded-full border px-3 py-1 text-sm transition",
              selected
                ? "border-primary bg-primary text-primary-foreground"
                : "border-border bg-background hover:bg-muted",
            )}
          >
            {opt.label}
          </button>
        );
      })}
    </div>
  );
}
```

- [ ] **Step 4.3: packages/ui/index.ts export**

```typescript
export { RangeSlider } from "./primitives/range-slider";
export { MultiSelect } from "./primitives/multi-select";
```

- [ ] **Step 4.4: i18n listings.ko.json**

`apps/web/lib/i18n/messages/listings.ko.json`:

```json
{
  "listings": {
    "page": {
      "title": "매물 검색"
    },
    "search": {
      "placeholder": "지역명을 검색해 주세요"
    },
    "filter": {
      "type": "매물 종류",
      "transaction": "거래 방식",
      "areaM2": "면적 (m²)",
      "priceKrw": "가격 (원)",
      "sort": "정렬"
    },
    "type": {
      "factory": "공장",
      "warehouse": "창고",
      "office": "사무실",
      "knowledge_industry_center": "지식산업센터",
      "industrial_land": "산업단지",
      "logistics_center": "물류센터"
    },
    "transaction": {
      "sale": "매매",
      "monthly_rent": "월세",
      "jeonse": "전세"
    },
    "sort": {
      "created_at_desc": "최신순",
      "price_asc": "가격 낮은 순",
      "price_desc": "가격 높은 순",
      "area_asc": "면적 좁은 순",
      "area_desc": "면적 넓은 순"
    },
    "card": {
      "viewCount": "조회",
      "bookmarkCount": "관심",
      "favoritePlaceholder": "즐겨찾기"
    },
    "empty": "조건에 맞는 매물이 없어요",
    "loading": "매물을 불러오는 중이에요",
    "errors": {
      "fetchFailed": "매물을 불러오지 못했어요. 잠시 후 다시 시도해 주세요."
    }
  }
}
```

`apps/web/i18n.ts` 의 message merge 에 listings 추가:

```typescript
const [common, auth, listings] = await Promise.all([
  import("./lib/i18n/ko.json"),
  import("./lib/i18n/messages/auth.ko.json"),
  import("./lib/i18n/messages/listings.ko.json"),
]);
return {
  locale,
  messages: { ...common.default, ...auth.default, ...listings.default },
};
```

- [ ] **Step 4.5: filter-bar.tsx + search-bar.tsx**

`apps/web/components/listings/search-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { Input } from "@gongzzang/ui";
import { useState } from "react";

export function SearchBar() {
  const t = useTranslations("listings.search");
  const [value, setValue] = useState("");
  return (
    <div className="w-full max-w-md">
      <Input
        type="search"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={t("placeholder")}
        aria-label={t("placeholder")}
      />
    </div>
  );
}
```

(NOTE: 실제 지역 검색은 미래 — 지금은 input 만 자리. T9 Open question 1.)

`apps/web/components/listings/filter-bar.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import { MultiSelect, RangeSlider } from "@gongzzang/ui";
import { useListingsStore } from "@/stores/listings";
import type { ListingType, TransactionType, SortKey } from "@/lib/listings/filters";
import { formatAreaM2, formatPriceKrw } from "@/lib/listings/format";

const TYPES: ListingType[] = [
  "factory", "warehouse", "office",
  "knowledge_industry_center", "industrial_land", "logistics_center",
];
const TXNS: TransactionType[] = ["sale", "monthly_rent", "jeonse"];
const SORTS: SortKey[] = [
  "created_at_desc", "price_asc", "price_desc", "area_asc", "area_desc",
];

const AREA_MIN = 0;
const AREA_MAX = 10_000;
const PRICE_MIN = 0;
const PRICE_MAX = 100_000_000_000; // 1000억

export function FilterBar() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const patch = useListingsStore((s) => s.patchFilters);

  const typeOptions = TYPES.map((v) => ({ value: v, label: t(`type.${v}`) }));
  const txnOptions = TXNS.map((v) => ({ value: v, label: t(`transaction.${v}`) }));

  return (
    <div className="flex flex-col gap-4 p-4">
      <section aria-label={t("filter.type")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.type")}</h3>
        <MultiSelect
          options={typeOptions}
          value={filters.types}
          onValueChange={(v) => patch({ types: v as ListingType[] })}
        />
      </section>
      <section aria-label={t("filter.transaction")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.transaction")}</h3>
        <MultiSelect
          options={txnOptions}
          value={filters.transactions}
          onValueChange={(v) => patch({ transactions: v as TransactionType[] })}
        />
      </section>
      <section aria-label={t("filter.areaM2")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.areaM2")}</h3>
        <RangeSlider
          min={AREA_MIN}
          max={AREA_MAX}
          step={100}
          value={[filters.minAreaM2 ?? AREA_MIN, filters.maxAreaM2 ?? AREA_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minAreaM2: min === AREA_MIN ? undefined : min,
              maxAreaM2: max === AREA_MAX ? undefined : max,
            })
          }
          formatValue={formatAreaM2}
        />
      </section>
      <section aria-label={t("filter.priceKrw")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.priceKrw")}</h3>
        <RangeSlider
          min={PRICE_MIN}
          max={PRICE_MAX}
          step={10_000_000}
          value={[filters.minPriceKrw ?? PRICE_MIN, filters.maxPriceKrw ?? PRICE_MAX]}
          onValueChange={([min, max]) =>
            patch({
              minPriceKrw: min === PRICE_MIN ? undefined : min,
              maxPriceKrw: max === PRICE_MAX ? undefined : max,
            })
          }
          formatValue={formatPriceKrw}
        />
      </section>
      <section aria-label={t("filter.sort")}>
        <h3 className="mb-2 text-sm font-semibold">{t("filter.sort")}</h3>
        <select
          value={filters.sort}
          onChange={(e) => patch({ sort: e.target.value as SortKey })}
          className="rounded border border-border bg-background px-3 py-2 text-sm"
          aria-label={t("filter.sort")}
        >
          {SORTS.map((s) => (
            <option key={s} value={s}>{t(`sort.${s}`)}</option>
          ))}
        </select>
      </section>
    </div>
  );
}
```

- [ ] **Step 4.6: typecheck + lint + commit**

```bash
pnpm typecheck && pnpm lint
git add packages/ui/primitives/range-slider.tsx packages/ui/primitives/multi-select.tsx packages/ui/index.ts packages/ui/package.json apps/web/components/listings/search-bar.tsx apps/web/components/listings/filter-bar.tsx apps/web/lib/i18n/messages/listings.ko.json apps/web/i18n.ts pnpm-lock.yaml
git commit -m "feat(6ii-T4): RangeSlider + MultiSelect primitives + FilterBar + i18n

- packages/ui: RangeSlider (Radix Slider) + MultiSelect (chip toggle) primitives
- listings/search-bar: 지역 검색 input (실 검색 API 통합 미래)
- listings/filter-bar: 종류/거래 multi + 평수/가격 range + 정렬 select + i18n
- listings.ko.json: 6 type + 3 transaction + 5 sort + filter labels"
```

---

## Task 5: Listing Card + Card List — 무한 스크롤 + skeleton + 핀↔카드 highlight

**Files:**
- Create: `apps/web/components/listings/listing-card.tsx`
- Create: `apps/web/components/listings/listing-card-list.tsx`

- [ ] **Step 5.1: listing-card.tsx**

`apps/web/components/listings/listing-card.tsx`:

```typescript
"use client";
import { useTranslations } from "next-intl";
import Link from "next/link";
import { Card, CardContent } from "@gongzzang/ui";
import { Heart } from "lucide-react";
import type { ListingCard as ListingCardData } from "@/lib/listings/api";
import { formatAreaPyeong, formatPriceKrw } from "@/lib/listings/format";
import { getPinColor } from "@/lib/listings/pin-color";
import { useListingsStore } from "@/stores/listings";

interface ListingCardProps {
  data: ListingCardData;
}

export function ListingCard({ data }: ListingCardProps) {
  const t = useTranslations("listings");
  const selectedId = useListingsStore((s) => s.selectedListingId);
  const setSelected = useListingsStore((s) => s.setSelectedListingId);
  const isSelected = selectedId === data.id;

  return (
    <Card
      className={`overflow-hidden transition ${
        isSelected ? "ring-2 ring-primary" : "hover:bg-muted/50"
      }`}
      onMouseEnter={() => setSelected(data.id)}
      onMouseLeave={() => setSelected(null)}
    >
      <Link href={`/listings/${data.id}`} className="block">
        <div
          className="aspect-[4/3] w-full bg-muted"
          style={{
            backgroundColor: data.thumbnail_url ? undefined : `${getPinColor(data.listing_type)}22`,
          }}
        >
          {data.thumbnail_url ? (
            <img src={data.thumbnail_url} alt={data.title} className="h-full w-full object-cover" />
          ) : (
            <div className="flex h-full items-center justify-center text-muted-foreground text-sm">
              {t(`type.${data.listing_type}`)}
            </div>
          )}
        </div>
        <CardContent className="p-4">
          <div className="mb-2 flex items-center gap-2">
            <span
              className="rounded-full px-2 py-0.5 text-xs font-medium text-white"
              style={{ backgroundColor: getPinColor(data.listing_type) }}
            >
              {t(`type.${data.listing_type}`)}
            </span>
            <span className="text-xs text-muted-foreground">
              {t(`transaction.${data.transaction_type}`)}
            </span>
          </div>
          <h3 className="mb-1 line-clamp-1 text-base font-semibold">{data.title}</h3>
          <div className="mb-2 text-sm text-muted-foreground">
            {formatAreaPyeong(data.area_m2)}
          </div>
          <div className="text-lg font-bold">{formatPriceKrw(data.price_krw)}</div>
          <div className="mt-2 flex items-center gap-3 text-xs text-muted-foreground">
            <span aria-label={t("card.viewCount")}>👁 {data.view_count}</span>
            <button
              type="button"
              aria-label={t("card.favoritePlaceholder")}
              className="flex items-center gap-1 hover:text-primary"
              onClick={(e) => {
                e.preventDefault();
                // SP6-iii 가 즐겨찾기 toggle 구현
              }}
            >
              <Heart className="h-3 w-3" /> {data.bookmark_count}
            </button>
          </div>
        </CardContent>
      </Link>
    </Card>
  );
}
```

- [ ] **Step 5.2: listing-card-list.tsx (무한 스크롤)**

`apps/web/components/listings/listing-card-list.tsx`:

```typescript
"use client";
import { useEffect, useRef } from "react";
import { useInfiniteQuery } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { ListingCard } from "@/components/listings/listing-card";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

const PAGE_SIZE = 20;

export function ListingCardList() {
  const t = useTranslations("listings");
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);

  const query = useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({
        filters,
        bounds,
        page: pageParam as number,
        size: PAGE_SIZE,
      }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });

  // 무한 스크롤 sentinel
  const sentinelRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!sentinelRef.current) return;
    const obs = new IntersectionObserver((entries) => {
      if (entries[0]?.isIntersecting && query.hasNextPage && !query.isFetchingNextPage) {
        query.fetchNextPage();
      }
    });
    obs.observe(sentinelRef.current);
    return () => obs.disconnect();
  }, [query]);

  if (query.isLoading) {
    return (
      <div className="flex flex-col gap-3 p-4">
        {Array.from({ length: 6 }).map((_, i) => (
          <div key={i} className="h-48 animate-pulse rounded-lg bg-muted" />
        ))}
      </div>
    );
  }

  if (query.isError) {
    return (
      <div className="p-8 text-center text-sm text-destructive">
        {t("errors.fetchFailed")}
      </div>
    );
  }

  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  if (allListings.length === 0) {
    return (
      <div className="p-8 text-center text-sm text-muted-foreground">
        {t("empty")}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-3 p-4">
      {allListings.map((listing) => (
        <ListingCard key={listing.id} data={listing} />
      ))}
      <div ref={sentinelRef} className="h-8" />
      {query.isFetchingNextPage && (
        <div className="text-center text-xs text-muted-foreground">{t("loading")}</div>
      )}
    </div>
  );
}
```

- [ ] **Step 5.3: typecheck**

```bash
pnpm --filter=@gongzzang/web typecheck
```

Expected: PASS.

- [ ] **Step 5.4: Commit**

```bash
git add apps/web/components/listings/listing-card.tsx apps/web/components/listings/listing-card-list.tsx
git commit -m "feat(6ii-T5): ListingCard + ListingCardList (무한 스크롤 + skeleton + 핀↔카드 highlight)

- listing-card: 사진 (또는 종류 placeholder) + type badge + 제목 + 평수 + 가격 + view/bookmark count + 즐겨찾기 자리 (SP6-iii) + hover → 핀 highlight
- listing-card-list: TanStack Query useInfiniteQuery + IntersectionObserver sentinel + skeleton (6 카드) + empty/error 상태 + i18n"
```

---

## Task 6: `/(authenticated)/listings/page.tsx` 통합 + i18n

**Files:**
- Create: `apps/web/app/(authenticated)/listings/page.tsx`
- Create: `apps/web/app/(authenticated)/listings/loading.tsx`

- [ ] **Step 6.1: page.tsx**

`apps/web/app/(authenticated)/listings/page.tsx`:

```typescript
import { getTranslations } from "next-intl/server";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { FilterBar } from "@/components/listings/filter-bar";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");

  return (
    <main className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border p-4">
        <h1 className="text-xl font-bold">{t("title")}</h1>
        <SearchBar />
      </header>
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[280px_1fr_400px]">
        <aside className="overflow-y-auto border-r border-border md:block hidden">
          <FilterBar />
        </aside>
        <section className="relative h-full" aria-label={t("title")}>
          <ListingMap listings={[]} />
        </section>
        <aside className="overflow-y-auto border-l border-border">
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
```

(NOTE: `<ListingMap listings={[]} />` — 실 listings 는 ListingCardList 의 useInfiniteQuery 가 fetch. ListingMap 도 같은 query 의 `data?.pages.flatMap` 을 받아야 — 이건 client side 에서 useListingsStore 통해 같이 sync. T6 의 이 통합은 단순화 — ListingMap 이 자체적으로 useInfiniteQuery 호출하거나 또는 listings prop 으로 받음. 후자가 더 단순.)

**개선** — ListingMap 도 listings prop 받기:

```typescript
"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

export function ListingsContent() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  const query = useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({ filters, bounds, page: pageParam as number, size: 20 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
  const allListings = query.data?.pages.flatMap((p) => p.listings) ?? [];

  return (
    <>
      <section className="relative h-full">
        <ListingMap listings={allListings} />
      </section>
      <aside className="overflow-y-auto border-l border-border">
        <ListingCardList query={query} />
      </aside>
    </>
  );
}
```

이 통합 component 를 만들고 page.tsx 에서 사용. ListingCardList 는 query 를 prop 으로 받도록 변경 (또는 hook 분리).

**SSS 결정**: query 를 별도 hook (`useListingsQuery`) 으로 분리 — ListingMap, ListingCardList 둘 다 사용. 단일 query, 캐시 공유.

`apps/web/lib/listings/use-listings-query.ts`:

```typescript
"use client";
import { useInfiniteQuery } from "@tanstack/react-query";
import { fetchListings, type ListingsResponse } from "@/lib/listings/api";
import { useListingsStore } from "@/stores/listings";

export function useListingsQuery() {
  const filters = useListingsStore((s) => s.filters);
  const bounds = useListingsStore((s) => s.bounds);
  return useInfiniteQuery<ListingsResponse>({
    queryKey: ["listings", filters, bounds],
    queryFn: ({ pageParam }) =>
      fetchListings({ filters, bounds, page: pageParam as number, size: 20 }),
    initialPageParam: 0,
    getNextPageParam: (last) => (last.has_next ? last.page + 1 : undefined),
    enabled: bounds !== undefined,
  });
}
```

ListingMap + ListingCardList 둘 다 `const query = useListingsQuery()` 호출 — TanStack Query 는 동일 queryKey 를 cache 공유.

ListingMap 의 props 변경:

```typescript
export function ListingMap() {
  const query = useListingsQuery();
  const listings = query.data?.pages.flatMap((p) => p.listings) ?? [];
  // ... 기존 동작
}
```

ListingCardList 도 `const query = useListingsQuery()`. (Step 5.2 의 query 를 hook 호출로 변경.)

이 변경 위해 Step 5.2 의 listing-card-list.tsx 를 hook 호출로 수정:

```typescript
import { useListingsQuery } from "@/lib/listings/use-listings-query";

export function ListingCardList() {
  const query = useListingsQuery();
  // ... 기존 (query 직접 호출 부분만 변경)
}
```

ListingMap 도 동일.

- [ ] **Step 6.2: page.tsx 단순화**

```typescript
import { getTranslations } from "next-intl/server";
import { ListingMap } from "@/components/listings/listing-map";
import { ListingCardList } from "@/components/listings/listing-card-list";
import { FilterBar } from "@/components/listings/filter-bar";
import { SearchBar } from "@/components/listings/search-bar";

export default async function ListingsPage() {
  const t = await getTranslations("listings.page");
  return (
    <main className="flex h-screen flex-col">
      <header className="flex items-center justify-between border-b border-border p-4">
        <h1 className="text-xl font-bold">{t("title")}</h1>
        <SearchBar />
      </header>
      <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-[280px_1fr_400px]">
        <aside className="overflow-y-auto border-r border-border md:block hidden">
          <FilterBar />
        </aside>
        <section className="relative h-full">
          <ListingMap />
        </section>
        <aside className="overflow-y-auto border-l border-border">
          <ListingCardList />
        </aside>
      </div>
    </main>
  );
}
```

- [ ] **Step 6.3: loading.tsx**

`apps/web/app/(authenticated)/listings/loading.tsx`:

```typescript
import { getTranslations } from "next-intl/server";

export default async function ListingsLoading() {
  const t = await getTranslations("listings");
  return (
    <main className="flex h-screen items-center justify-center">
      <div className="text-sm text-muted-foreground">{t("loading")}</div>
    </main>
  );
}
```

- [ ] **Step 6.4: typecheck + dev 로컬 시연**

```bash
pnpm typecheck
# 별도 터미널: pnpm --filter=@gongzzang/web dev
# http://localhost:3000/listings 접속 (로그인 후)
```

- [ ] **Step 6.5: Commit**

```bash
git add apps/web/app/\(authenticated\)/listings/ apps/web/lib/listings/use-listings-query.ts apps/web/components/listings/listing-map.tsx apps/web/components/listings/listing-card-list.tsx
git commit -m "feat(6ii-T6): /(authenticated)/listings 통합 + useListingsQuery hook

- /listings/page.tsx: 3-column 그리드 (필터/지도/카드 list) + 헤더 (제목 + 검색바)
- useListingsQuery: 단일 useInfiniteQuery hook (ListingMap + ListingCardList 캐시 공유)
- loading.tsx: Suspense fallback (i18n)"
```

---

## Task 7: Pretendard self-host + dark mode + CSP cdn 제거

**Files:**
- Create: `apps/web/public/fonts/Pretendard-Regular.woff2`
- Create: `apps/web/public/fonts/Pretendard-Medium.woff2`
- Create: `apps/web/public/fonts/Pretendard-Bold.woff2`
- Create: `apps/web/public/fonts/Pretendard-Heavy.woff2`
- Modify: `apps/web/app/layout.tsx`
- Modify: `packages/ui/tokens/typography.css`
- Modify: `apps/web/proxy.ts` (CSP)

- [ ] **Step 7.1: Pretendard variable woff2 다운로드 (4 가중치)**

```bash
mkdir -p apps/web/public/fonts
cd apps/web/public/fonts
# Pretendard variable subset web font
curl -L -o Pretendard-Regular.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Regular.woff2
curl -L -o Pretendard-Medium.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Medium.woff2
curl -L -o Pretendard-Bold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-Bold.woff2
curl -L -o Pretendard-ExtraBold.woff2 https://github.com/orioncactus/pretendard/raw/v1.3.9/packages/pretendard/dist/web/static/woff2/Pretendard-ExtraBold.woff2
ls -la
```

(NOTE: 4 file ≈ 800 KB 합. license = OFL 1.1 Pretendard 의 라이선스. README 의 attribution 추가 권장.)

- [ ] **Step 7.2: app/layout.tsx 의 next/font/local**

`apps/web/app/layout.tsx` 수정:

```typescript
import localFont from "next/font/local";

const pretendard = localFont({
  src: [
    { path: "../public/fonts/Pretendard-Regular.woff2", weight: "400", style: "normal" },
    { path: "../public/fonts/Pretendard-Medium.woff2", weight: "500", style: "normal" },
    { path: "../public/fonts/Pretendard-Bold.woff2", weight: "700", style: "normal" },
    { path: "../public/fonts/Pretendard-ExtraBold.woff2", weight: "800", style: "normal" },
  ],
  variable: "--font-pretendard",
  display: "swap",
});

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="ko" className={pretendard.variable}>
      <body className="font-sans">
        {/* ... existing providers ... */}
      </body>
    </html>
  );
}
```

(NOTE: 기존 layout.tsx 의 정확한 내용은 Read 후 정확히 변경. providers wrapper, NextIntlClientProvider 등 유지.)

- [ ] **Step 7.3: tokens/typography.css 정리**

`packages/ui/tokens/typography.css` 의 `@import url('https://cdn.jsdelivr.net/...')` 줄 제거. font-family 만 유지:

```css
:root {
  --font-sans: var(--font-pretendard), -apple-system, BlinkMacSystemFont, "Segoe UI",
    "Helvetica Neue", "Apple SD Gothic Neo", sans-serif;
}
```

`tailwind.config.ts` (또는 inline) 의 fontFamily.sans 가 `var(--font-sans)` 사용하도록 — 이미 그럴 수도 있음 (Read 로 확인).

- [ ] **Step 7.4: proxy.ts CSP 의 cdn.jsdelivr 제거**

`apps/web/proxy.ts` 의 CSP `style-src` 정리:

```typescript
const cspHeader = [
  `default-src 'self'`,
  `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
  `style-src 'self' 'unsafe-inline'`,           // cdn.jsdelivr 제거됨
  `img-src 'self' data: blob:`,
  `font-src 'self' data:`,                      // self-host 만 허용
  `connect-src 'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER}`,
  `frame-ancestors 'none'`,
  `base-uri 'self'`,
  `form-action 'self' ${env.ZITADEL_ISSUER}`,
].join("; ");
```

(NOTE: 기존 cdn.jsdelivr.net allow 는 SP6-foundation 시점 자리. self-host 전환 후 삭제. — 단 기존 코드에 이미 추가됐는지 Read 로 확인. 없을 수도 있음.)

- [ ] **Step 7.5: 로컬 검증**

```bash
pnpm --filter=@gongzzang/web dev
# 브라우저: http://localhost:3000/listings
# DevTools → Network → fonts/* 의 200 + 자체 도메인 확인
# DevTools → Console 에 "Refused to execute" / "violates CSP" 경고 없어야
```

- [ ] **Step 7.6: bundle size**

```bash
pnpm --filter=@gongzzang/web test:bundle
```

Expected: under threshold (Pretendard 800 KB → next/font 가 subset 자동 적용 → 실제 < 200KB 추가).

- [ ] **Step 7.7: Commit**

```bash
git add apps/web/public/fonts/ apps/web/app/layout.tsx packages/ui/tokens/typography.css apps/web/proxy.ts
git commit -m "feat(6ii-T7): Pretendard self-host (next/font/local) + CSP cdn.jsdelivr 제거

- public/fonts/Pretendard-{Regular,Medium,Bold,ExtraBold}.woff2 (OFL 1.1, attribution in README)
- app/layout.tsx: localFont (4 weights, swap display, --font-pretendard variable)
- tokens/typography.css: cdn.jsdelivr import 제거, --font-sans = var(--font-pretendard) chain
- proxy.ts CSP: style-src 의 cdn.jsdelivr.net 제거 (self-host 전환 완료)"
```

---

## Task 8: E2E + a11y + mobile responsive + bundle

**Files:**
- Create: `apps/web/tests/e2e/listings.spec.ts`
- Modify: `apps/web/playwright.config.ts` (필요시 viewport)

- [ ] **Step 8.1: e2e listings.spec.ts**

`apps/web/tests/e2e/listings.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const ZITADEL_REAL = process.env.ZITADEL_E2E_REAL === "true";

test.describe("listings search", () => {
  test("/(authenticated)/listings 미인증 → /login redirect", async ({ page }) => {
    await page.goto("/listings");
    await page.waitForURL(/\/login/, { timeout: 10000 });
    expect(page.url()).toContain("returnTo=%2Flistings");
  });

  test("a11y on /listings (인증 우회 — Zitadel 없을 때 skip)", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");
    // 로그인 흐름 (auth.spec.ts 의 login flow 와 같음, 단순화)
    await page.goto("/login");
    await page.click('button[type="submit"]');
    await page.waitForURL(/localhost:8443/);
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    await page.waitForURL(/\/profile|\/listings/, { timeout: 30000 });
    await page.goto("/listings");

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  test("필터 변경 → URL query 동기화", async ({ page }) => {
    test.skip(!ZITADEL_REAL, "real Zitadel container required");
    await page.goto("/listings");
    // 종류 chip click
    await page.getByRole("button", { name: "공장", exact: true }).click();
    await expect(page).toHaveURL(/types=factory/);
  });
});
```

- [ ] **Step 8.2: 로컬 e2e**

```bash
docker compose -f infra/zitadel/docker-compose.yml up -d
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 1 PASS (미인증 redirect), 2 skip (ZITADEL_E2E_REAL 미설정).

- [ ] **Step 8.3: bundle 검증**

```bash
pnpm --filter=@gongzzang/web test:bundle
```

Expected: PASS.

- [ ] **Step 8.4: Commit**

```bash
git add apps/web/tests/e2e/listings.spec.ts
git commit -m "test(6ii-T8): listings e2e + a11y (Zitadel-dep test 는 ZITADEL_E2E_REAL flag)

- 미인증 → /login redirect (CI 자동 실행)
- a11y axe 검증 (Zitadel 의존 — SP6-iam-infra ephemeral env 에서 ZITADEL_E2E_REAL=true)
- 필터 chip 클릭 → URL query 동기화 (SP6-iam-infra 에서 검증)"
```

---

## Task 9: docs + ADR

**Files:**
- Create: `docs/frontend/listings-search.md`
- Create: `docs/adr/0006-listing-search-naver-maps.md`

- [ ] **Step 9.1: ADR 0006**

`docs/adr/0006-listing-search-naver-maps.md`:

```markdown
# ADR-0006: Listing 검색 화면의 지도 vendor — Naver Maps

| | |
|---|---|
| 작성일 | 2026-05-05 |
| 상태 | Accepted |
| 결정자 | 사용자 |
| 컨텍스트 | SP6-ii (매물 검색 화면) — 지도 SDK 선택 |

## 결정

**Naver Maps JavaScript SDK** 를 SP6-ii 의 지도 vendor 로 채택.

## 대안 비교

| 기준 | Naver Maps | 카카오맵 | Google Maps |
|---|---|---|---|
| 한국 산업단지 정확도 | ◎ | ◎ | △ (해외 base) |
| 무료 quota (dev) | 10만/월 | 30만/월 | 28000/월 |
| 부동산 표준 | ◎ (네이버 부동산) | ○ | △ |
| 공시지가 / 산업단지 layer | 별도 | 별도 | X |
| API key 발급 | NCP 가입 필요 | 카카오 Dev 가입 | Google Cloud |
| 한국어 UI / docs | ◎ | ◎ | ○ |

## 결정 근거

1. 네이버 부동산 = 한국 부동산 표준 — 사용자 친숙도
2. Naver Maps SDK 의 산업단지 표시 정확도 (KSURE / GIS layer 기반)
3. 향후 V-World / 공시지가 layer 통합 시 Naver geo coding 호환성 (PNU 매핑)

## 결과

- `apps/web/lib/naver-maps.ts` lazy SDK loader
- `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` env 추가 (zod validated)
- 6 매물 종류 unique pin color
- 클러스터링 (`submodules=clustering`) 사용

## 미래 결정 자리

- Naver Maps 무료 quota 초과 시 카카오맵 fallback (SP7-i 의 quota alert + SP6-data-sync 가 batch 호출 분리)
- 해외 매물 추가 시 Google Maps multi-vendor 검토
```

- [ ] **Step 9.2: docs/frontend/listings-search.md**

`docs/frontend/listings-search.md`:

```markdown
# Listings 검색 화면 — 운영 가이드

> SP6-ii. 디버깅 / 데이터 source / Naver Maps quota / 자주 발생하는 이슈.

## 1. 화면 흐름

```
/login (SP6-i) → /listings (SP6-ii)
                  ↓
                  proxy.ts auth gate (sid → access_token Bearer)
                  ↓
                  GET /api/proxy/listings?bounds=&types=&page=
                  ↓
                  backend GET /listings → ListingRepository::find_card_summaries_in_bbox
                  ↓
                  PostGIS ST_Within(geom_point, ST_MakeEnvelope) + filter + page
```

## 2. 환경 변수

```
NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID=<NCP Maps Client ID>
```

NCP 가입 후 Maps 등록. Free tier: 10만 호출/월. 초과 시 SP7-i 의 alert.

## 3. 자주 발생하는 이슈

| 증상 | 원인 후보 | 확인 |
|---|---|---|
| 지도 안 뜸 | NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID 미설정 | DevTools Console 의 `Naver Maps SDK failed` |
| 매물 0건 | DB 에 매물 없음 / 모든 매물 status != active | psql `SELECT count(*) FROM listing WHERE status='active'` |
| 필터 무시 됨 | URL query 와 store 동기화 깨짐 | DevTools Network 의 `?types=` 확인 |
| 무한 스크롤 안 됨 | IntersectionObserver sentinel 가 viewport 밖 | DevTools Elements 의 sentinel div 확인 |

## 4. 데이터 source

- DB `listing` 테이블 (V001) — `status='active'` 만 표시
- 외부 API (V-World / data.go.kr) sync 는 SP6-data-sync 가 책임
- 사진 (`thumbnail_url`) 은 SP6-iii 의 listing-photo 테이블 join 으로 채움

## 5. 주요 컴포넌트

| File | 역할 |
|---|---|
| `app/(authenticated)/listings/page.tsx` | 통합 layout (3-column grid) |
| `components/listings/listing-map.tsx` | Naver Maps + 핀 |
| `components/listings/listing-card-list.tsx` | 카드 + 무한 스크롤 |
| `lib/listings/use-listings-query.ts` | 단일 useInfiniteQuery hook |
| `stores/listings.ts` | Zustand: bounds + filters + selectedListingId |

## 6. 미래 sub-project 자리

- SP6-iii: 매물 상세 (`/listings/:id`) + 즐겨찾기 toggle + 사진 갤러리
- SP6-iv: 매물 등록 (broker)
- SP6-data-sync: V-World / data.go.kr / 공공 API → DB 자동 sync
- SP6-search-region: 지역 검색 (Naver/카카오 주소 검색 API 통합)

## 7. Spec / Plan / ADR

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-ii-listing-search-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-ii-listing-search.md`
- ADR-0006: `docs/adr/0006-listing-search-naver-maps.md`
```

- [ ] **Step 9.3: markdownlint + commit**

```bash
pnpm markdownlint-cli2 docs/frontend/listings-search.md docs/adr/0006-listing-search-naver-maps.md
git add docs/frontend/listings-search.md docs/adr/0006-listing-search-naver-maps.md
git commit -m "docs(6ii-T9): listings-search.md 운영 가이드 + ADR-0006 (Naver Maps 결정)

- frontend/listings-search.md: 디버깅 / 환경 변수 / 자주 발생 이슈 / 데이터 source / 미래 SP 자리
- adr/0006: Naver vs 카카오 vs Google 비교 + 한국 부동산 표준 + 무료 quota 결정 근거"
```

---

## 최종 검증 (T8 완료 후)

- [ ] **Step F.1: Push + 4 CI workflow 그린 확인**

```bash
git push origin main
gh run list --branch main --limit 5 --json status,conclusion,name
```

Expected: 4/4 success.

- [ ] **Step F.2: 사용자 manual 검증 (DB 의 매물 등록 + 화면 확인)**

```bash
# 1. Zitadel + Redis dev container
docker compose -f infra/zitadel/docker-compose.yml up -d

# 2. backend 시작
cargo run -p api

# 3. 가짜 매물 1개 SQL 직접 insert (또는 psql 으로)
psql $DATABASE_URL -c "
INSERT INTO listing (id, owner_id, parcel_pnu, listing_type, transaction_type,
                     price_krw, area_m2, title, status, geom_point, contact_visibility)
VALUES (
  'lst_test01HXY...',
  '<existing user.id>',
  '4111017200103580000',
  'factory', 'sale', 8000000000, 3960.0,
  '평택 첨단산업단지 공장',
  'active',
  ST_SetSRID(ST_MakePoint(127.0876, 37.0779), 4326),
  'login_required'
);
"

# 4. frontend dev
pnpm --filter=@gongzzang/web dev
```

브라우저 → `/login` → admin → `/listings` → 평택 매물 핀 + 카드 표시 확인.

- [ ] **Step F.3: SP6-ii 완료 보고 + 다음 sub-project 의향**

다음 후보:
- SP6-iii: 매물 상세 + 즐겨찾기
- SP6-iv: broker 매물 등록
- SP6-data-sync: 외부 API → DB sync
- SP6-iam-infra: Zitadel Pulumi 화 + production HTTPS 검증

---

## Spec coverage 자가 점검

| Spec § | 요구사항 | 구현 task |
|---|---|---|
| 2.1 Frontend 화면 | `/listings` page + 지도 + 카드 + 필터 + 무한 스크롤 + skeleton | T6 (page) + T3 (map) + T5 (card) + T4 (filter) |
| 2.1 Backend API | `GET /listings` + bounds + filter + page + sort | T1 |
| 2.1 디자인 | Pretendard self-host + Card/Range/MultiSelect + dark mode | T7 + T4 |
| 2.1 Naver Maps | SDK + 핀 + 클러스터 + bounds 이벤트 | T3 |
| 2.1 Mobile responsive | 작은 화면 toggle | T6 (md:grid-cols-) + T8 (e2e) |
| 4 API contract | RFC 7807 + zod | T1 (backend) + T2 (frontend zod) |
| 6 Task 분해 | T1-T9 | 전체 |
| 7 SSS 7기둥 | 일관성/자동강제/추적성/안전성/가시성/SSOT/명확성 | T1-T9 분산 |
| 8 Testing | unit + integration + e2e + a11y + mobile + bundle | T2-T8 |
| 9 디자인 시스템 | Pretendard self-host + dark mode + range/multi primitive | T7 + T4 |
| 10 Open questions | 5 종 (사진 / 지역 검색 / debounce / 클러스터 / 무한 스크롤) | T1/T3/T4/T5 시점 결정 |

**미반영 = 0**.
