# Sub-project 6-iii: 매물 상세 + 북마크 — Spec

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Draft |
| 선행 | SP6-ii (Listing 검색), SP5-ii (PgBookmarkRepository), SP6-iv (mutation 패턴) |
| 후속 | SP6-v 알림 |
| 추정 | 8 task, 1.5-2일 |

---

## 1. 개요

검색 → 상세 → 즐겨찾기 사용자 흐름 닫음. SP6-ii 가 검색 카드 리스트까지,
SP6-iv 가 broker 등록까지 완성한 상태에서 **상세 페이지** 와 **북마크 toggle**
이 빈 자리.

**핵심 흐름**:

1. `/listings` 카드 리스트 → 카드 클릭 → `/listings/:id` 상세 페이지
2. 상세에서 "즐겨찾기" 버튼 → 즉시 toggle (mutation), bookmark_count 업데이트
3. `/me/bookmarks` 내 북마크 목록 (선택, 본 SP 1차 미포함)

**SSS 핵심 결정 — bookmark_count 정합성**:

기존 follow-up FU 21 = "outbox consumer 가 listing.bookmark_count 동기화" 였으나
근본 SSS 검토에서 *denormalization 자체를 제거* 가 더 정확함:

- Bookmark 저장 시 outbox consumer 가 별도 작동 → eventual consistency 위험
- 본 SP 결정 = **JOIN COUNT 직접 응답** (denormalized 필드 미사용)
- `find_card_summaries_in_bbox` / `find_by_id` 가 `LEFT JOIN bookmark_listing` +
  `count(*)` — 한 query 로 정확
- `Listing.bookmark_count` 필드는 *deprecated* 표기, FU 70 에서 schema 제거

이렇게 하면:
- SSOT § 6 — bookmark_listing 테이블이 진실. 사본 0
- 자동강제 § 2 — 시스템이 매번 정확. consumer 가 깨져도 응답 정합
- 추적성 § 3 — bookmark_listing INSERT/DELETE 가 audit_log + outbox 그대로 남음
  (SP5-ii 패턴)

---

## 2. 범위

### 포함

#### Backend (`services/api/src/routes/`)

신규 endpoint 4 (모두 인증 필수):

- **`GET /listings/:id`** — 매물 상세 단건. 본인 또는 *Active/Sold/Expired* 만
  공개. PendingReview/Rejected/Draft 는 `owner_id == auth.user.id` 만.
  - 응답 = `ListingDetailResponse` (Listing 21 필드 + photos[] + bookmark_count
    JOIN COUNT + `is_bookmarked: bool` (현재 사용자 기준))
  - `record_view_count` 호출 — `Listing::increment_view_count` (version bump 안
    함, 빈번한 갱신이라 OCC 와 무관). 본인 조회 시 counter 증가 안 함 (자기
    조회 노이즈).
- **`POST /listings/:id/bookmark`** — 즐겨찾기 추가 (멱등). `BookmarkListing::try_new`
  로 만든 후 `repo.save_listing_bookmark(bm, ctx)`. 이미 있으면 UPSERT (note 갱신).
- **`DELETE /listings/:id/bookmark`** — 해제. `repo.delete_listing_bookmark(...)`.
  멱등 — 이미 없으면 200 (NotFound 매핑 X).
- **`GET /me/bookmarks`** — 내 북마크 listing/external. SP6-iii 1차 = listing 만.
  external 은 FU 71.

#### Domain — `Listing` 변경 없음

`Listing::record_bookmark` / `release_bookmark` 는 *제거 안 함* — denormalized
counter 가 deprecated 일 뿐, 향후 cache 용도로 부활 가능 (FU 21 재해석).

#### Repository — `ListingRepository` 시그니처 보강

`find_card_summaries_in_bbox` 의 `ListingCardSummary` 에 `is_bookmarked: bool`
필드 추가 — 현재 사용자 ID 를 query 에 함께 넘겨 LEFT JOIN. 인증 사용자만 쓰는
endpoint 라 항상 user_id 있음.

```rust
pub struct CardSearchQuery {
    // ... 기존 필드
    pub viewer_user_id: Id<UserMarker>, // 신규 — bookmark JOIN 용
}
```

**SP6-ii 영향**: get_listings handler 가 `auth.user.id` 를 query 에 주입.
프론트엔드 `ListingCard.is_bookmarked` 표시.

#### Repository — `find_detail_by_id` 신규

```rust
pub trait ListingRepository {
    // ...
    async fn find_detail_by_id(
        &self,
        id: &Id<ListingMarker>,
        viewer_user_id: &Id<UserMarker>,
    ) -> Result<Option<ListingDetail>, RepoError>;
}

pub struct ListingDetail {
    pub listing: Listing,                    // 21 필드
    pub photos: Vec<ListingPhotoSummary>,
    pub bookmark_count: i64,                 // JOIN COUNT, NOT denormalized
    pub is_bookmarked: bool,                 // 본 viewer 기준
}

pub struct ListingPhotoSummary {
    pub r2_key: String,
    pub thumbnail_r2_key: Option<String>,
    pub caption: Option<String>,
    pub display_order: i32,
    pub content_type: PhotoContentType,
}
```

PgImpl 가 단일 query (LEFT JOIN bookmark_listing + LEFT JOIN listing_photo)
또는 2 query (listing + photos + count 따로) 트레이드오프 — 1차는 **2 query**
명확성 우선. tx isolation 으로 정합 (REPEATABLE READ).

#### Frontend (`apps/web/`)

- **`app/(authenticated)/listings/[id]/page.tsx`** — server component, fetch
  initial data + suspense boundary
- **`components/listings/listing-detail.tsx`** — client component, BookmarkButton
  과 photo gallery + 도메인 정보 panel 묶음
- **`components/listings/bookmark-button.tsx`** — 즉시 toggle (optimistic UI),
  TanStack Query mutation, sonner toast
- **`lib/listings/api.ts` 확장** — `fetchListingDetail(id)`
- **`lib/listings/mutations.ts` 확장** — `useToggleBookmark` (POST/DELETE)
- **`lib/listings/use-bookmark-state.ts`** — TanStack Query hook (cache 공유)
- **i18n `listings.ko.json` 확장** — 상세 라벨

ListingCard 의 heart 버튼 — *상세 페이지 navigation 만 트리거* (단순 link).
toggle 은 detail page 에서. 카드에서 inline toggle 은 SP6-iii-2 (FU 72).

### 미포함

- **외부 (Parcel/Mfr) 북마크 UI** — backend 는 SP5-ii 가 만들어 둠. 프론트엔드 UI
  는 FU 71. 본 SP 1차 = listing 만.
- **`/me/bookmarks` 페이지 UI** — endpoint 는 backend 추가, 프론트엔드 페이지는
  FU 73 (선택)
- **사진 carousel + lightbox** — 1차 = thumbnail grid. carousel 은 FU 74.
- **카드 inline bookmark toggle** — FU 72.
- **broker contact reveal** (`contact_visibility` 별 분기 UI) — FU 75 (auth /
  verified 사용자 분기 로직)

---

## 3. 컴포넌트

### 3.1 `find_detail_by_id` SQL

```sql
SELECT
  l.*,
  COALESCE(b.cnt, 0) as bookmark_count,
  CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END as is_bookmarked
FROM listing l
LEFT JOIN (
  SELECT listing_id, COUNT(*) as cnt
  FROM bookmark_listing
  GROUP BY listing_id
) b ON b.listing_id = l.id
LEFT JOIN bookmark_listing ub
  ON ub.listing_id = l.id AND ub.user_id = $viewer_user_id
WHERE l.id = $id
```

photos 는 별도 query (single round-trip 보장 위해 sqlx pipeline).

### 3.2 ListingPhoto 응답 매핑

`ListingPhotoRepository::find_by_listing` 가 도메인 12 필드 반환.
`ListingPhotoSummary` 는 frontend 표시 5 필드만 사출 (audit/소유 메타 제외).

### 3.3 view_count 갱신

`record_view_count` 가 `Listing::increment_view_count(now)` 호출 후
`PgListingRepository::save` — 단 *MutationContext 없이* (view 가 audit_log 일관
대상 아님 — 빈도 너무 높음). 별도 trait method 또는 조용한 UPDATE.

```rust
pub trait ListingRepository {
    /// view_count 증가 — version bump X, audit X (빈도가 높아 분리).
    async fn increment_view_count(
        &self,
        id: &Id<ListingMarker>,
    ) -> Result<(), RepoError>;
}
```

PgImpl 단순 `UPDATE listing SET view_count = view_count + 1, updated_at = now() WHERE id = $1`.

본인이 본인 매물 조회 시 skip — handler 에서 `if listing.owner_id != auth.user.id { increment_view_count }`.

### 3.4 RBAC — 비공개 상태 접근

```rust
fn can_view(listing: &Listing, viewer_id: &Id<UserMarker>) -> bool {
    use ListingStatus::*;
    match listing.status {
        Active | Sold | Expired => true,
        Draft | PendingReview | Rejected => listing.owner_id == *viewer_id,
        Archived => listing.owner_id == *viewer_id, // FU: admin 도 허용 검토
    }
}
```

비허용 → `403 forbidden` 또는 `404 not-found`. SSS 답: **404 not-found** —
존재 자체 leak 안 함 (BVQ/LRQ 동일 패턴).

### 3.5 `is_bookmarked` 카드 리스트 통합

`find_card_summaries_in_bbox` 의 SQL 에 LEFT JOIN bookmark_listing 추가:

```sql
LEFT JOIN bookmark_listing ub
  ON ub.listing_id = l.id AND ub.user_id = $viewer_user_id
```

select 절에 `CASE WHEN ub.user_id IS NOT NULL THEN true ELSE false END as is_bookmarked`.

`bookmark_count` 도 동일 GROUPED COUNT — 모든 bookmark JOIN 단일 쿼리. 시계열
필터 / sort 가 추가돼도 견고.

### 3.6 ProblemDetails 매핑 추가

| 도메인 에러 | HTTP | type ID |
|---|---|---|
| `BookmarkError::EmptyTargetId` | 400 | `validation` |
| `BookmarkError::TargetIdTooLong` | 400 | `validation` |
| `BookmarkError::NoteTooLong` | 400 | `validation` |
| Bookmark `RepoError::NotFound` | 404 | `not-found` (이미 없으면 멱등 — 삭제 시 200) |
| Bookmark `RepoError::Database` | 500 | `internal-error` |

---

## 4. 검증 기준 (DoD)

1. `find_detail_by_id` + `increment_view_count` PgImpl + 단위 테스트 3 (도메인은
   변경 없음)
2. `find_card_summaries_in_bbox` 의 `is_bookmarked` 추가 + 기존 통합 테스트 갱신
3. 4 endpoint (`GET /listings/:id` / `POST + DELETE bookmark` / `GET /me/bookmarks`)
4. 6 통합 테스트:
   - detail happy path (Active 매물 + photos + count + is_bookmarked false)
   - detail Draft owner 만 보임 (cross-user → 404)
   - detail bookmark count + is_bookmarked true round-trip
   - bookmark POST 멱등 (UPSERT)
   - bookmark DELETE 멱등 (이미 없어도 200)
   - GET /me/bookmarks 가 user 별 격리
5. Frontend `[id]/page.tsx` + `listing-detail.tsx` + `bookmark-button.tsx`
6. e2e 1: 검색 → 카드 클릭 → 상세 → bookmark toggle → 카드로 돌아오면 heart
   채워짐
7. Vitest 4: bookmark mutation hook / detail fetch hook / view count UX /
   404 redirect
8. clippy `--all-targets` + typecheck + Vitest + Playwright 그린
9. SSOT 갱신 (roadmap, project_progress, MEMORY)

---

## 5. SSS 7기둥

| 기둥 | 적용 |
|---|---|
| 1 일관성 | 모든 mutation `MutationContext::new_user_action` (북마크 patterns) |
| 2 자동강제 | bookmark_count = JOIN COUNT — denormalization 사본 0, 시스템이 매번 정확 |
| 3 추적성 | bookmark INSERT/DELETE 가 audit_log + outbox (SP5-ii 패턴) |
| 4 안전성 | 비공개 상태 = 404 (존재 leak 차단). 멱등 design (재시도 안전) |
| 5 가시성 | view_count 별도 path — audit 노이즈 분리. tracing instrument |
| 6 SSOT | bookmark_listing 테이블 = 진실. denormalized counter 제거 |
| 7 명확성 | RBAC 룰 (`can_view`) 단일 함수 — handler 가 호출. 산재 X |

---

## 6. Follow-up

- **FU 70**: `listing.bookmark_count` schema 컬럼 제거 (deprecation 후 마이그)
- **FU 71**: 외부 (Parcel/Mfr/IC/CourtAuction) 북마크 UI
- **FU 72**: ListingCard inline bookmark toggle (검색 결과 카드에서 즉시)
- **FU 73**: `/me/bookmarks` 프론트엔드 페이지
- **FU 74**: 사진 carousel + lightbox (keyboard nav + a11y)
- **FU 75**: `contact_visibility` 별 broker contact 표시 분기
- **FU 76**: bookmark 알림 — broker 가 본인 매물 북마크 받으면 notification
  (SP6-v 와 묶음)

---

## 7. Risk

- **`find_card_summaries_in_bbox` SQL 변경** = SP6-ii 통합 테스트 5+ 영향. 모두
  `viewer_user_id` 추가로 갱신
- **bookmark JOIN COUNT 비용** — 1만 매물 / 사용자당 평균 5 북마크 = 무시 가능.
  100만 매물 도달 시 materialized view 또는 dedicated counter cache (Redis FU 28)
  로 evolve
- **Listing.record_bookmark / release_bookmark** = dead code 화 — `#[deprecated]`
  주석 + clippy allow. 향후 cache layer 시 부활 시점에 maintainers 가 재심의
