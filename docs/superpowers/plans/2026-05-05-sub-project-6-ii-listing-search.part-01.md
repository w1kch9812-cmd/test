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

