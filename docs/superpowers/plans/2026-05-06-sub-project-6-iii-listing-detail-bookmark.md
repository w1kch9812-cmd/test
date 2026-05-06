# Sub-project 6-iii: 매물 상세 + 북마크 — 계획

| | |
|---|---|
| 작성일 | 2026-05-06 |
| 상태 | Approved |
| 선행 spec | [`2026-05-06-sub-project-6-iii-listing-detail-bookmark-design.md`](../specs/2026-05-06-sub-project-6-iii-listing-detail-bookmark-design.md) |
| 추정 | 8 task, 1.5-2일 |

---

## T1 — spec + plan 커밋

이 commit. `docs(sp6-iii): spec + plan -- 매물 상세 + 북마크 (bookmark_count JOIN COUNT, denormalized 제거)`

---

## T2 — `ListingRepository` 시그니처 확장

**대상**: `crates/domain/core/listing/src/repository.rs`

- `ListingDetail` struct + `ListingPhotoSummary` 추가
- `find_detail_by_id(id, viewer_user_id) -> Result<Option<ListingDetail>, RepoError>`
- `increment_view_count(id) -> Result<(), RepoError>` (별도 — audit 미적용)
- `CardSearchQuery` 에 `viewer_user_id: Id<UserMarker>` 필드 추가
- `ListingCardSummary` 에 `is_bookmarked: bool` 필드 추가
- `Listing::record_bookmark / release_bookmark` 에 `#[deprecated(note = "FU 70 schema 제거 예정 — JOIN COUNT 사용")]`

domain crate trait + entity 만 수정. PgImpl 은 T3.

**commit**: `feat(sp6-iii-t2): ListingRepository -- find_detail_by_id + increment_view_count + is_bookmarked`

---

## T3 — `PgListingRepository` 구현 + tests

**대상**: `crates/db/src/listing.rs`

- `find_detail_by_id`:
  - 단일 query (LEFT JOIN bookmark_listing GROUP COUNT + LEFT JOIN viewer ub)
  - photos 별도 query (`PgListingPhotoRepository::find_by_listing` 사용)
  - SET REPEATABLE READ 또는 단일 connection 보장으로 정합
- `increment_view_count`: `UPDATE listing SET view_count = view_count + 1, updated_at = now() WHERE id = $1`
- `find_card_summaries_in_bbox` SQL 갱신:
  - LEFT JOIN bookmark_listing 으로 GROUP COUNT + viewer ub
  - select 절 `is_bookmarked` 추가
- 기존 카드 통합 테스트 (`crates/db/tests/listing_integration.rs`) `viewer_user_id` 인자 추가

**commit**: `feat(sp6-iii-t3): PgListingRepository find_detail_by_id + increment_view_count + JOIN bookmark`

---

## T4 — `services/api` GET `/listings/:id` 핸들러

**대상**: `services/api/src/routes/listings.rs`

- `ListingDetailResponse` (struct)
- `get_listing_detail` handler:
  - `auth.user.id` 추출
  - `repo.find_detail_by_id(id, &auth.user.id)` → 404 if None or RBAC fail
  - `can_view(listing, viewer_id)` 가드 (Active/Sold/Expired or owner only) →
    falls back to 404
  - 본인 아니면 `repo.increment_view_count(id)` (best-effort, 실패 시 warn)
  - 응답 200
- 라우터 등록 (`main.rs::listings_router`)

**commit**: `feat(sp6-iii-t4): GET /listings/:id 상세 endpoint (RBAC + view count)`

---

## T5 — `BookmarkRepository` 라우트 + `me/bookmarks`

**대상**: `services/api/src/routes/bookmarks.rs` (신규) + `main.rs`

- `BookmarksState { bookmark_repo, listing_repo (? — counter sync) }`
- `POST /listings/:id/bookmark` — `BookmarkListing::try_new` + `repo.save_listing_bookmark(bm, http_user_action(&auth, "bookmark_listing"))`
- `DELETE /listings/:id/bookmark` — `repo.delete_listing_bookmark(...)`. NotFound 무시 (멱등 200).
- `GET /me/bookmarks` — `repo.find_listing_bookmarks(&auth.user.id)` → response array
- `BookmarksResponse` shape + ProblemDetails 매핑 (`from_bookmark_repo_error`)

**commit**: `feat(sp6-iii-t5): /listings/:id/bookmark POST/DELETE + GET /me/bookmarks`

---

## T6 — backend 통합 테스트 6 시나리오

**대상**: `crates/db/tests/listing_integration.rs` 보강 + `crates/db/tests/bookmark_integration.rs` 보강 (또는 신규 `listing_detail_integration.rs`)

- `find_detail_active_returns_full_data` — Active + photos + bookmark_count
- `find_detail_draft_cross_user_returns_none` — RBAC 확인
- `find_detail_with_viewer_bookmark_sets_is_bookmarked_true` — 북마크 한 사람 = true
- `bookmark_save_idempotent` — 같은 listing 두 번 저장 → 한 row, note 갱신
- `bookmark_delete_idempotent` — 두 번 삭제 → 두 번째 NotFound 무시
- `find_listing_bookmarks_user_isolation` — A user / B user bookmark 격리

**commit**: `feat(sp6-iii-t6): 6 integration tests (detail + bookmark idempotency + RBAC)`

---

## T7 — Frontend 상세 + 북마크

**대상**: `apps/web/`

- `app/(authenticated)/listings/[id]/page.tsx` — server component, suspense
  boundary, error boundary (404 → not-found.tsx)
- `components/listings/listing-detail.tsx` — gallery (thumbnail grid) +
  도메인 panel (가격/면적/주소) + BookmarkButton + view count display
- `components/listings/bookmark-button.tsx` — optimistic toggle, sonner toast,
  TanStack Query mutation invalidate
- `lib/listings/api.ts` 확장 — `fetchListingDetail(id)` + zod schema
- `lib/listings/mutations.ts` 확장 — `useToggleBookmark(listingId)` (POST or
  DELETE based on current state)
- `lib/listings/use-listing-detail.ts` (선택) — TanStack Query hook
- i18n `listings.ko.json` 추가 — 상세/북마크 라벨
- `app/(authenticated)/listings/[id]/not-found.tsx` — 404 fallback
- Vitest 4: bookmark mutation hook (mock fetch / optimistic / rollback) +
  detail fetch hook + view count display + 404 case
- Playwright e2e 1: 검색 → 카드 클릭 → 상세 → bookmark toggle → 검색으로 돌아와
  is_bookmarked=true 카드

**commit**: `feat(sp6-iii-t7): /listings/[id] 상세 페이지 + bookmark toggle + e2e`

---

## T8 — workspace 검증 + push + SSOT

- 로컬 `cargo clippy --workspace --all-features --all-targets -- -D warnings` 그린
- 로컬 `pnpm -F web run typecheck test` 그린
- push → 5 CI workflow 그린
- SSOT 갱신 (roadmap SP6-iii ✅, memory/project_progress 본문, FU 70-76 추가,
  MEMORY.md index)

**commit**: `docs(sp6-iii-t8): SP6-iii 종료 -- 매물 상세 + 북마크 (denormalized counter 제거)`

---

## 변경 파일 요약

| 분류 | 파일 | 변경 |
|---|---|---|
| 도메인 | `crates/domain/core/listing/src/{repository,entity}.rs` | trait + struct 추가 / `record_bookmark` deprecated |
| PgImpl | `crates/db/src/listing.rs` | `find_detail_by_id` + `increment_view_count` + `find_card_summaries_in_bbox` JOIN |
| Routes | `services/api/src/routes/{listings,bookmarks}.rs` | GET detail + bookmark CRUD |
| Routes wiring | `services/api/src/main.rs` | 라우터 갱신 |
| 통합 테스트 | `crates/db/tests/{listing,bookmark}_integration.rs` (또는 신규 detail_integration) | 신규 6 |
| Frontend pages | `apps/web/app/(authenticated)/listings/[id]/{page,not-found}.tsx` | 신규 |
| Frontend components | `apps/web/components/listings/{listing-detail,bookmark-button}.tsx` | 신규 |
| Frontend lib | `apps/web/lib/listings/{api,mutations,use-listing-detail}.ts` | 확장/신규 |
| Frontend i18n | `apps/web/lib/i18n/messages/listings.ko.json` | 확장 |
| Frontend tests | Vitest 4 + Playwright 1 | 신규 |
| docs | spec + plan + roadmap + project_progress + MEMORY | 신규/갱신 |

총 ~25-30 파일.

---

## 위험 요소

- **`find_card_summaries_in_bbox` 시그니처 변경**: SP6-ii 의 카드 응답 shape 변경
  (`is_bookmarked` 추가). 기존 frontend 코드 호환 — `is_bookmarked` 가 optional
  이라면 OK. 1차 = required + frontend 즉시 사용. SP6-ii 의 ListingCardSchema
  zod 갱신 필수
- **viewer_user_id 항상 필요**: bookmark JOIN 이 sentinel user_id (anonymous)
  를 어떻게? — 모든 listings endpoint 가 인증 필수라 항상 user 있음. anonymous
  접근 = SP9 (B2C 확장) 영역
- **JOIN COUNT 성능**: 1만 매물 / 사용자당 5 북마크 = 무시. 100만 도달 시
  materialized view 또는 Redis counter cache (FU 28 와 묶음)
- **`view_count` 동시성**: 단순 UPDATE += 1 — atomic. version bump 안 하므로
  OCC 충돌 X. 단 쓰기 빈도 높아 connection pool 압박 가능 — 1차는 그대로,
  트래픽 급증 시 batched (Redis HINCRBY → flush) 로 evolve
- **deprecated `record_bookmark` warning**: clippy + cargo check 그린 유지 위해
  `#[deprecated]` + 호출 시점 `#[allow(deprecated)]` 필요. 호출자 = 이전 outbox
  consumer 가정 코드 (현재 zero) → 영향 0
