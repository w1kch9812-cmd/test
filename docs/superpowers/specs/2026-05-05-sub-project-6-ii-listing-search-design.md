# Sub-project 6-ii: Listing 검색 화면 — Naver Maps + 카드 list (Spec)

| | |
|---|---|
| 작성일 | 2026-05-05 |
| 상태 | Design (사용자 승인 대기) |
| 선행 | SP6-foundation (Next.js + ky + i18n + shadcn), SP6-i (Auth Core — 로그인 후 진입), SP2 V001 (`listing` 테이블) |
| 후속 | SP6-iii (매물 상세 + 즐겨찾기), SP6-iv (매물 등록 — broker), SP6-data-sync (V-World/data.go.kr → DB) |

---

## 1. 개요

공짱의 첫 진짜 product 화면 — **로그인 후 사용자가 산업용 매물을 지도 + 카드로 탐색**.
Naver 부동산 스타일 (좌측 지도 + 우측 카드 list + 상단 필터) 로 6 종 매물 (공장 / 창고 / 사무실 / 지식산업센터 / 산업단지 / 물류센터) 을 통합 검색해요.

본 sub-project 는 frontend (검색 UI) + backend (`GET /listings`) 양쪽을 함께 다뤄요.
외부 API (V-World, data.go.kr) 자동 sync 는 **SP6-data-sync** 가 책임 — 본 SP 는 DB 의 `listing` 테이블에 들어있는 매물을 그대로 표시.

---

## 2. 범위 (Scope)

### 포함

**Frontend 화면**
- `/listings` 메인 검색 화면 (로그인 후 진입)
- Naver Maps 지도 + 매물 핀 (종류별 색상 구분)
- 우측 매물 카드 list (지도 영역과 동기화)
- 상단 검색바 + 필터 (지역 / 종류 / 거래방식 / 평수 / 가격)
- 카드 상호작용: hover → 핀 highlight, 핀 클릭 → 카드 highlight + 스크롤
- 무한 스크롤 (페이지당 20건)
- Skeleton + Suspense (loading UX)
- Mobile responsive (지도 / 카드 toggle, 작은 화면)
- Dark mode toggle (Pretendard self-host)

**Backend API**
- `GET /listings` — bounds + filter + page + sort 기반 매물 list
- DB 의 `status='active'` 매물만 표시 (default)
- PostGIS bounding box query (`ST_MakeEnvelope` + `ST_Within`)
- 페이지네이션 (offset + limit)
- Response: 매물 array + 총 개수 + 다음 page cursor

**디자인 시스템 작업** (이번 SP 의 visual 진짜 손봄)
- Pretendard self-host (`next/font` 으로 cdn 의존 제거 — CSP block 해소)
- Card / Badge / Filter / Range Slider primitives 추가
- Dark mode 정확한 token (color / background / border)

**Naver Maps 통합**
- `@naver/maps` SDK 또는 직접 script 삽입
- 매물 핀 클러스터링 (지도 zoom-out 시 묶음 표시)
- 지도 이동 → debounced bounds 업데이트 → list 자동 갱신

### 미포함 (후속 SP)

- **SP6-iii**: 매물 상세 (`/listings/:id`) + 즐겨찾기 + 사진 갤러리
- **SP6-iv**: 매물 등록 (broker 전용) — listing CRUD
- **SP6-data-sync**: V-World / data.go.kr / 공공 API → DB 자동 sync (cron / event-driven)
- 매물 사진 업로드 / 변환 (S3 + image transform)
- 알림 (관심 지역 새 매물 push)
- AI 매칭 / 추천 (사용자 행동 기반)

### 결정 사항 (이번 brainstorming 에서 확정)

| # | 항목 | 결정 | 사유 |
|---|---|---|---|
| 1 | 데이터 source | **DB 의 listing 테이블** (실 데이터, dummy 0) | 사용자 요구. 외부 API sync 는 별도 SP |
| 2 | 화면 형태 | **Naver 부동산 스타일** (좌 지도 + 우 카드) | 한국 부동산 표준 패턴 |
| 3 | 지도 vendor | **Naver Maps** | 한국 산업단지 정확도, 부동산 표준 |
| 4 | 매물 종류 | **6종 모두** (factory / warehouse / office / knowledge_industry_center / industrial_land / logistics_center) | DB schema 이미 6종 정의 — 그대로 활용 |
| 5 | 거래방식 | **3종 모두** (sale / monthly_rent / jeonse) | DB schema 이미 정의 |
| 6 | 페이징 | **무한 스크롤 + 페이지당 20건** | 모바일 친화 + 지도 영역 변경 시 자연스러움 |
| 7 | Default 필터 | **status='active'** | draft / pending_review / sold 는 일반 사용자에게 표시 X |
| 8 | 즐겨찾기 표시 | **카드의 하트 아이콘 자리만** (toggle 동작은 SP6-iii) | SP6-iii 가 진짜 채움 |
| 9 | 매물 상세 진입 | **카드 클릭 → `/listings/:id`** (SP6-iii 가 페이지 채움) | SP6-iii 가 화면 만듦 |

---

## 3. 아키텍처

```
┌──────────────────────────────────────────────────────────────┐
│  Browser  (인증된 사용자, SP6-i 의 sid cookie)               │
└────────────────────┬─────────────────────────────────────────┘
                     │
                     ▼
┌─ Next.js 16 (apps/web) ──────────────────────────────────────┐
│  /(authenticated)/listings/page.tsx                          │
│    ├── SearchBar (지역 검색 — 카카오 주소 검색 또는 직접)    │
│    ├── FilterBar (종류 / 거래방식 / 평수 range / 가격 range) │
│    ├── ListingMap (Naver Maps + 핀 + 클러스터)              │
│    └── ListingCardList (카드 + 무한 스크롤 + sentinel)      │
│                                                              │
│  /api/proxy/listings  →  backend GET /listings              │
└────────────────────┬─────────────────────────────────────────┘
                     │ Authorization: Bearer (sid → access_token)
                     ▼
┌─ services/api (Rust) ────────────────────────────────────────┐
│  GET /listings                                               │
│    Query: bounds=south,west,north,east                       │
│           types=factory,warehouse,...                        │
│           transaction=sale,jeonse                            │
│           min_area=300&max_area=2000                         │
│           min_price=0&max_price=10000000000                  │
│           page=0&size=20                                     │
│           sort=created_at_desc                               │
│                                                              │
│  → ListingRepository (port-only trait + PgImpl)              │
│    ↓ PostGIS ST_MakeEnvelope + ST_Within(geom_point, env)    │
│  → Postgres listing 테이블                                    │
└──────────────────────────────────────────────────────────────┘
```

### 3.1 Trust 경계

- 모든 `/listings` 호출은 **인증 필수** (SP6-i 의 proxy.ts auth gate). 일반 사용자도 매물 list 보려면 로그인.
- `contact_visibility` field 는 **응답에서 제외** (SP6-iii 가 detail 페이지에서 처리). list 는 매물 위치 + 가격 + 사진만.
- broker 의 owner contact 는 SP6-iii 에서 verified_only 검증 후 노출.

### 3.2 Naver Maps SDK

- 정식 가입 (Naver Cloud Platform — Maps API). dev key 무료 ~10만 호출/월.
- Client-side script tag (직접 inject) — `@types/navermaps` 으로 type 추가.
- API key 환경변수: `NEXT_PUBLIC_NAVER_MAPS_CLIENT_ID` (zod env.ts 추가).

---

## 4. Backend API Contract

### `GET /listings`

**Query parameters** (전부 optional, 없으면 default):

| Param | Type | Default | 의미 |
|---|---|---|---|
| `bounds` | `south,west,north,east` (4 floats) | 한국 전체 | PostGIS ST_MakeEnvelope (4326) |
| `types` | comma-separated | 6 종 모두 | listing_type filter |
| `transaction` | comma-separated | 3 종 모두 | transaction_type filter |
| `min_area_m2` | float | 0 | area_m2 >= |
| `max_area_m2` | float | infinity | area_m2 <= |
| `min_price_krw` | int | 0 | price_krw >= |
| `max_price_krw` | int | infinity | price_krw <= |
| `page` | int | 0 | offset = page × size |
| `size` | int | 20 | limit (max 100) |
| `sort` | enum | `created_at_desc` | created_at_desc / price_asc / price_desc / area_asc / area_desc |

**Response 200**:

```json
{
  "listings": [
    {
      "id": "lst_01HXY...",
      "title": "평택 첨단산업단지 공장",
      "listing_type": "factory",
      "transaction_type": "sale",
      "price_krw": 8000000000,
      "deposit_krw": null,
      "monthly_rent_krw": null,
      "area_m2": 3960.0,
      "lat": 37.0779,
      "lng": 127.0876,
      "thumbnail_url": null,
      "view_count": 42,
      "bookmark_count": 3,
      "created_at": "2026-04-12T09:30:00+09:00"
    }
  ],
  "total": 1234,
  "page": 0,
  "size": 20,
  "has_next": true
}
```

**Errors**: RFC 7807 ProblemDetails (SP6-i 의 패턴 일관)

- `400 listings/invalid-bounds` — bounds 가 4개 float 아닌 경우
- `400 listings/invalid-filter` — types/transaction 의 enum 외 값
- `401` — 미인증 (proxy.ts 차단)
- `502 proxy/upstream-unavailable` — backend 다운

---

## 5. 디렉토리 구조

```
apps/web/
├── app/(authenticated)/listings/
│   ├── page.tsx                      # 메인 검색 화면
│   └── loading.tsx                   # Suspense fallback (skeleton)
├── components/listings/               # SP6-ii 의 핵심
│   ├── search-bar.tsx                # 지역 검색 (또는 자유 keyword)
│   ├── filter-bar.tsx                # 종류 / 거래 / 평수 / 가격 multiselect + range
│   ├── listing-map.tsx               # Naver Maps + 핀 + 클러스터
│   ├── listing-pin.tsx               # 종류별 색상 핀 컴포넌트
│   ├── listing-card.tsx              # 카드 (사진 + 정보 + 즐겨찾기 hint)
│   └── listing-card-list.tsx         # 무한 스크롤 + sentinel + skeleton
├── lib/listings/
│   ├── api.ts                        # ky 호출 + zod 응답 스키마
│   ├── filters.ts                    # filter state (URL query 동기화)
│   ├── format.ts                     # 가격 한국 표기 (123억 4,500만원), 평수 변환
│   └── pin-color.ts                  # listing_type → 색상 매핑
├── stores/
│   └── listings.ts                   # Zustand: 지도 bounds + filter + selected listing
└── lib/i18n/messages/listings.ko.json # listings.{search, filter, pin, card, errors}

packages/ui/
├── primitives/
│   ├── range-slider.tsx              # 평수 / 가격 range (shadcn 추가)
│   └── multi-select.tsx              # 종류 / 거래방식 multi (shadcn 추가)
└── tokens/
    └── (Pretendard self-host — next/font + 무게별 적용)

services/api/src/
└── routes/listings.rs                # GET /listings handler

crates/db/src/
└── listing.rs                        # ListingRepository::find_in_bounds (신규 메서드)

crates/domain/core/listing/src/
└── repository.rs                     # 신규 trait method 추가
```

---

## 6. Task 분해 (writing-plans 에서 상세화)

| Task | 내용 | 파일 | 추정 |
|---|---|---|---|
| **T1** | Backend `GET /listings` — `ListingRepository::find_in_bounds` (PostGIS bounding box + filter) + handler + RFC 7807 에러 | `crates/db`, `crates/domain/core/listing`, `services/api/src/routes/listings.rs` | 0.75d |
| **T2** | Frontend `lib/listings/api.ts` (ky + zod 응답) + `stores/listings.ts` (Zustand) + filter URL query 동기화 + format helpers | `apps/web/lib/listings/`, `apps/web/stores/listings.ts` | 0.5d |
| **T3** | Naver Maps 통합 — `lib/naver-maps.ts` 로더 + `<ListingMap>` + 핀 + 클러스터 + bounds 이벤트 → store | `apps/web/components/listings/listing-map.tsx`, `apps/web/lib/naver-maps.ts` | 0.75d |
| **T4** | Filter / Search bar — 지역 / 종류 / 거래 / 평수 / 가격 + URL query 동기화 | `apps/web/components/listings/{search-bar,filter-bar}.tsx`, `packages/ui/primitives/{range-slider,multi-select}.tsx` | 0.75d |
| **T5** | Listing Card + Card List — 무한 스크롤 + skeleton + 핀 ↔ 카드 highlight | `apps/web/components/listings/{listing-card,listing-card-list,listing-pin}.tsx` | 0.75d |
| **T6** | `/(authenticated)/listings/page.tsx` 통합 + i18n + 한국어 가격/평수 포맷 | `apps/web/app/(authenticated)/listings/`, `apps/web/lib/i18n/messages/listings.ko.json` | 0.5d |
| **T7** | Pretendard self-host (`next/font`) + dark mode token 정리 + CSP 의 cdn.jsdelivr 제거 | `apps/web/app/layout.tsx`, `packages/ui/tokens/`, `apps/web/proxy.ts` (CSP) | 0.5d |
| **T8** | E2E (Playwright) + a11y + 모바일 responsive + bundle size 검증 | `apps/web/tests/e2e/listings.spec.ts`, `apps/web/playwright.config.ts` | 0.5d |
| **T9** | docs/frontend/listings-search.md 운영 가이드 + ADR (지도 vendor 결정 근거) | `docs/frontend/`, `docs/adr/` | 0.25d |

총 **5.25d** (≈ 5-6일).

---

## 7. SSS 7 기둥 매핑

| 기둥 | SP6-ii 의 구체 강제 |
|---|---|
| **일관성** | 모든 매물 = 동일 `<ListingCard>` 컴포넌트, 동일 가격 포맷 (`format.ts`). filter URL query 동기화 (북마크 가능) |
| **자동 강제** | proxy.ts 의 auth gate 가 `/listings` 진입 차단 (미인증 → /login). zod 응답 검증 (backend 가 schema 외 응답 시 client throw). E2E + a11y CI 차단 |
| **추적성** | 매물 view 시 `view_count` 증가 (backend trigger), 즐겨찾기 시 audit_log (SP6-iii) — SP6-ii 시점에는 자리만 |
| **안전성** | 모든 응답 zod parse + RFC 7807. PostGIS bounding box query (SQL injection 차단 — sqlx prepared) |
| **가시성** | TanStack Query devtools (dev only) + `withSpan("listings.fetch", ...)` (T4 의 tracer 사용) |
| **SSOT** | DB `listing` 테이블 = 매물 SSOT. backend `Listing` entity = 도메인 SSOT. frontend zod schema = backend response 의 type-safe 사본 (openapi-typescript 자동 생성 자리) |
| **명확성** | 매물 종류 6종 + 거래방식 3종 = `crates/domain` 의 enum SSOT. `format.ts` 가 한국 가격/평수 표기 SSOT. `pin-color.ts` 가 종류 색상 SSOT |

---

## 8. Testing 전략

| Layer | Tool | What |
|---|---|---|
| Unit | Vitest | `format.ts` (한국 가격 변환), `filters.ts` (URL query parse), `pin-color.ts` (종류 매핑), zod schema |
| Backend | cargo test | `ListingRepository::find_in_bounds` (Postgres + PostGIS), filter combination, sort |
| Integration | Vitest + msw | `lib/listings/api.ts` 의 backend 호출 mock (`/listings?bounds=...`) |
| E2E | Playwright | `/login → /listings` redirect 후 카드 표시 확인. 필터 변경 → URL query 변경. 지도 이동 → list 갱신 (실 backend + dev DB) |
| A11y | @axe-core/playwright | `/listings` WCAG 2.1 AA — 키보드 navigation, screen reader landmark, color contrast (dark mode) |
| Mobile | Playwright (모바일 viewport) | 지도 / 카드 toggle, terach 제스처 |
| Bundle | size-limit | 새 deps (Naver Maps SDK + range-slider) bundle 영향 — threshold 조정 |

**Coverage 목표**: backend `crates/db/src/listing.rs::find_in_bounds` + `crates/domain/core/listing/src/repository.rs` 90% 유지.

---

## 9. 디자인 시스템 진짜 손봄 (T7)

SP6-foundation 이 디자인 system 토대 작성, SP6-i 가 인증 화면. SP6-ii 가 처음 **진짜 product 화면** — 디자인 system 본격 사용.

**Pretendard self-host**:
- `next/font/local` 으로 4 가중치 (Regular/Medium/Bold/Heavy)
- CSP 의 cdn.jsdelivr 차단 (proxy.ts 의 style-src) → 자동 해소
- variable font (1 file) 또는 weight 별 파일 — 결정 후 plan

**Dark mode token**:
- `packages/ui/tokens/colors.css` 의 light/dark CSS variables
- listing-card 의 background / border / text 가 dark mode 에서 올바른 contrast

**Mobile responsive**:
- 지도 + 카드 동시 표시는 PC (≥1024px). 모바일은 toggle.
- Filter bar 는 모달 / drawer 패턴.

---

## 10. Open questions

| # | 질문 | 결정 시점 |
|---|---|---|
| 1 | 매물 사진 (thumbnail_url) — 지금은 backend 가 어디서 생성? | T1 시점 — 기존 `listing-photo` 테이블 + S3 자리 확인. 없으면 placeholder |
| 2 | 지역 검색 — 카카오 주소 검색 API vs Naver 자체 검색 | T4 시점. 둘 다 사업자 등록 필요 |
| 3 | Bounds 변경 시 debounce 시간 (300ms? 500ms?) | T3 시점, UX 테스트 |
| 4 | Pin 클러스터링 — Naver SDK 내장 vs 직접 구현 | T3 시점. Naver SDK 가 클러스터 plugin 제공 |
| 5 | 무한 스크롤 vs "더 보기" 버튼 | T5 시점. 무한 스크롤 default, fallback |

---

## 11. Reference

- [SP6-foundation spec](./2026-05-05-sub-project-6-foundation-design.md) — Next.js + ky + i18n
- [SP6-i spec](./2026-05-05-sub-project-6-i-auth-design.md) — Auth Core
- [V001 listing schema](../../../migrations/10001_core_tables.sql) — listing 테이블 SSOT
- [Naver Maps API](https://navermaps.github.io/maps.js.ncp/docs/) — JavaScript SDK
- [Naver Cloud Platform — Maps Pricing](https://www.ncloud.com/product/applicationService/maps) — dev 무료 quota
- [PostGIS ST_MakeEnvelope](https://postgis.net/docs/ST_MakeEnvelope.html) — bounding box
- [Pretendard webfont](https://github.com/orioncactus/pretendard) — Korean web font

---

**다음 단계**: 사용자 review → spec 승인 → `writing-plans` skill 로 implementation plan → `subagent-driven-development` 로 T1-T9 실행.
