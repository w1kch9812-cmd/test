# SP10: Panel System — 지도 클릭 → 패널 stack 시스템

| | |
|---|---|
| 작성일 | 2026-05-07 |
| 상태 | Draft |
| 결정 ADR | (작성 예정) ADR 0022 — Panel Stack as Single SSOT URL-Driven Mechanism |
| 목적 | 지도 위 entity (필지/매물/...) 클릭 시 정보 패널을 띄우는 *시스템* — 단순 컴포넌트가 아니라 framework |
| 추정 | 6 task, 1주 |

## 1. 목표

사용자가 매물 검색 지도 (`/listings`) 에서:

1. 필지 폴리곤 클릭 → Panel 1 = 필지 요약 (PNU, 지목, 면적 등)
2. Panel 1 의 자식 link 클릭 → Panel 2 = 그 entity 의 확장 (예: 건축물, 등록 매물)
3. Panel 1 + Panel 2 가 *동시* 표시되어 맥락 비교 가능 (데스크톱)
4. 모바일에서는 동일 stack 을 full-screen 으로 takeover, breadcrumb 으로 navigation
5. URL 만으로 정확히 어느 stack 상태인지 표현되어 — 새로고침 / 공유 / 뒤로가기 / Android hw back / iOS swipe-back 모두 *standard 호환*
6. 새 entity kind 추가 시 framework 코드 0 변경

## 2. 비목표 (Phase 2+ 로 미룸)

- Tear-off (다중 stack 동시 운용) — 현재는 단일 linear stack
- 스크린리더 `aria-live` 음성 알림 — v1 에 ARIA 기본 (`role="dialog"`, `aria-modal`) 만
- per-kind staleTime 튜닝표 — v1 에 단일 default (5min)
- RSC SSR pre-hydration — v1 에 client-only fetch (단 *URL hydration 자체* 는 v1 필수)
- 분석 대시보드 (panel funnel) — v1 에 Sentry breadcrumb + OTEL trace 까지만
- Panel 내 inline edit form — v1 에 read-only 카드만 (action 패널은 별도 view 로 v2)

## 3. 핵심 추상화 — Pattern E (Typed Stack)

drill-down / cross-entity navigation / action 모두 *동일 mechanism*.

```ts
type PanelKind = 'parcel' | 'listing';                         // v1 lock
type PanelView<K extends PanelKind> =
  | (K extends 'parcel'  ? 'summary' | 'buildings' | 'listings' : never)
  | (K extends 'listing' ? 'summary' : never);

type PanelStackEntry = {
  [K in PanelKind]: { kind: K; id: string; view: PanelView<K> };
}[PanelKind];

type PanelStack = { v: 1; entries: PanelStackEntry[] };
```

**slot 아닌 stack** — depth 가변, 의미는 entry 내용으로 결정.

## 4. 레이아웃

| viewport | renderer | 동작 |
|---|---|---|
| ≥ xl (1280px) | `SideBySideStack` | top **2장** side-by-side. depth 3+ 는 sliding window of top 2 + breadcrumb 의 "회색 항목" 으로 표시. |
| < xl | `FullScreenStack` | top **1장** 만 full-screen. 상단 `‹ 이전` + `현재/총 depth`. |

전환은 `<PanelRenderer>` 에서 `useMediaQuery('(min-width: 1280px)')` 한 줄로. 그 외 *어느 컴포넌트에도 viewport 분기 코드 없음*.

같은 `<PanelCard>` / `<Breadcrumb>` 컴포넌트가 두 renderer 모두에서 재사용. 사본 0.

## 5. URL = SSOT

### 5.1. 직렬화 grammar (v1 impl = G1)

```
/listings?p=parcel:1168010100107370000.summary>listing:abc-uuid.summary
            └────────── entry 1 ───────────┘ └─── entry 2 ───┘
            kind ':' id '.' view
            entry 들은 '>' 로 stack push 표현
```

### 5.2. Codec interface (lock-in)

```ts
interface PanelStackCodec {
  serialize(stack: PanelStack): string;
  deserialize(s: string): Result<PanelStack, ParseError>;
  CURRENT_VERSION: 1;
}
```

- v1 impl = G1 (readable text)
- 향후 G2 (encoded JSON) 또는 G3 (압축 base64) 로 swap 가능 — 호출자 영향 0
- 모든 string 파싱은 `codec.deserialize` 한 곳만 — `string.split('>')` 같은 ad-hoc 파싱 lint 차단

### 5.3. Kind 정규식 강제 (등록 시점)

```ts
defineKind('parcel', { idPattern: /^\d{19}$/, ... });
defineKind('listing', { idPattern: /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/, ... });
```

unsafe character 가 들어간 ID 는 등록 시점에 컴파일 또는 런타임 검증으로 차단 — URL grammar 안전성 보장.

### 5.4. URL = SSOT 원칙

- 모든 panel mutation 은 `router.push` / `router.back` (Next.js router)
- zustand 의 `panelStack` 은 URL 의 *reactive 사본* — URL 을 driving 하지 않음
- 효과: 브라우저 ‹ 버튼, Android hw back, iOS edge-swipe back, 인앱 webview chrome, forward, 새 탭, 공유, 새로고침 — *모두 추가 코드 0 으로 작동*

### 5.5. 깨진 URL 처리

`codec.deserialize` 가 `Err(ParseError)` 반환 시 `?p=` 무시하고 패널 0 으로 시작. 사용자에게 silent 회복. Sentry 에 `panel.url_decode_failed` 이벤트.

## 6. Registry Shape (R1)

```ts
defineKind({
  // 필수 (lock-in)
  kind: 'parcel',
  idPattern: /^\d{19}$/,

  views: {
    summary:   { component: ParcelSummaryCard,    fetcher: (id) => api.parcels.get(id),                 staleTime: 5 * 60_000, links: [...] },
    buildings: { component: ParcelBuildingsCard,  fetcher: (id) => api.buildings.list({ parcel_pnu: id }), staleTime: 5 * 60_000, links: [...] },
    listings:  { component: ParcelListingsCard,   fetcher: (id) => api.listings.list({ pnu: id }),     staleTime: 60_000,    links: [] },
  },

  // 4-state 강제 (없으면 컴파일 에러)
  loadingComponent: ParcelSkeleton,
  errorComponent:   ParcelErrorCard,
  emptyComponent:   ParcelEmptyCard,
  authGate:         { required: false },

  // i18n
  i18nNamespace: 'panels.parcel',

  // 텔레메트리 (Sentry breadcrumb / OTEL trace 자동 attach)
  telemetryAttrs: (entry) => ({ pnu: entry.id }),
});
```

`links` = `{ from: 'fieldId', to: (entry) => PanelStackEntry }` 배열. 등록 시점에 *target kind/view* 가 다른 kind 의 registry 에 정의되어 있는지 type-check.

## 7. Fetch Path — F1-pure (REST 리소스 + 관계 필터)

backend = pure REST resource server. panel view = frontend mapping.

| view | endpoint | 상태 |
|---|---|---|
| `parcel.summary` | `GET /api/parcels/:pnu` | **신규** (parcel-lookup crate 위 thin REST shell) |
| `parcel.buildings` | `GET /api/buildings?parcel_pnu=:pnu` | **신규** (data.go.kr building reader 위) |
| `parcel.listings` | `GET /api/listings?pnu=:pnu` | ✅ 기존 (SP6-ii) |
| `listing.summary` | `GET /api/listings/:id` | ✅ 기존 (SP6-iii) |

**backend 는 "panel view" 라는 단어를 모름** — 그냥 REST resource server. SSOT 깨짐 0.

### 7.1. v1 backend 변경

| crate / service | 변경 |
|---|---|
| `crates/parcel-lookup` | 기존 (T4) — public 함수 그대로 |
| `services/api` | `GET /api/parcels/:pnu` 라우트 추가, parcel-lookup 호출 |
| `services/api` | `GET /api/buildings` 라우트 추가, `?parcel_pnu=` 필터 처리 |
| `crates/data-clients/data-go-kr/building_register` | 기존 (SP4-iii-a) — Reader trait 그대로 |

OpenAPI (utoipa) 자동 생성 → frontend 의 `api.parcels.get` / `api.buildings.list` 가 type-safe 하게 wired.

## 8. v1 Entity Scope

| kind | v1 | 데이터 source | 비고 |
|---|---|---|---|
| `parcel` | ✅ Locked | V-World ACL (T4) | 사용자 명시 lock |
| `listing` | ✅ Recommended | DB | 사용자가 옵션 A 채택 근거로 직접 시나리오 언급 |
| `building` | FU | data.go.kr (SP4-iii-a) | parcel.buildings view 의 child 가 될 수 있음 (FU 시점) |
| `broker` | FU | DB schema | listing 의 child link |
| `realTransaction` | FU | data.go.kr (SP4-iii-b) | parcel 의 child |
| `industrialComplex` | FU | SP9 ETL T3b.5 후 | 폴리곤 클릭 신규 source |
| `admin` | FU | SP9 ETL 후 | 폴리곤 클릭 신규 source |
| `officialLandPrice` | FU | data.go.kr (TBD) | parcel 의 child |

> 새 kind 추가 = registry entry 1 + 컴포넌트 파일들 + i18n namespace. **framework 코드 (lib/panel/*) 변경 0.** Lint 가 강제.

## 9. v1 Production Rules — 17 lock + 3 FU

| # | 기둥 | 규칙 | 강제 방법 |
|---|---|---|---|
| 1 | SSOT | URL = panel state SSOT | lint: zustand 가 router 안 거치고 set 하면 fail |
| 2 | SSOT | registry = 모든 kind/view 정의 | TS discriminated union — 등록 안 된 view import 컴파일 에러 |
| 3 | 자동강제 | discriminated union | TS strict |
| 4 | 자동강제 | `disallowed-types` lint — 패널 상태 만들 때 `useState` 차단 | eslint config |
| 5 | 자동강제 | framework → kind 폴더 import 차단 | eslint `no-restricted-imports` |
| 6 | 안전성 | error boundary per card | `<PanelCard>` 가 `<ErrorBoundary>` 로 감쌈 + registry 의 errorComponent 강제 |
| 7 | 안전성 | AuthGate 컴파일 강제 | registry 의 `authGate` 필수 필드 |
| 8 | 안전성 | AbortController per slot | `usePanelStack` 이 push 시 직전 slot 의 AbortController.abort() 자동 |
| 9 | 가시성 | Sentry breadcrumb + OTEL trace + analytics event 표준 | `usePanelStack` 의 push/pop 이 telemetry helper 호출 |
| 10 | 가시성 | panel-loaded p95 SLO | Grafana 대시보드 (FU)  + OTEL span 측정 자동 |
| 11 | 일관성 | 모바일 = 단일 규칙 (xl breakpoint) | `<PanelRenderer>` 한 곳에서만 분기 |
| 12 | 일관성 | i18n type-safe namespace | next-intl typed namespace, registry `i18nNamespace` 필수 |
| 13 | 추적성 | `/listings/[id]` page = `listing.summary` 동일 컴포넌트 | server redirect → `/listings?p=listing:id.summary`. 컴포넌트 사본 0. |
| 14 | 안전성 | focus push on open / restore on close | `<PanelCard>` 의 `useFocusTrap` 표준 hook |
| 15 | 안전성 | ESC 닫기 표준 | `<PanelCard>` 의 keydown handler |
| 16 | 안전성 | `prefers-reduced-motion` 존중 | tailwind motion-safe / motion-reduce, `<PanelCard>` 가 자동 |
| 17 | 안전성 | 4-state shell (loading/error/empty/auth-required) | `<PanelCard>` 가 fetcher state 따라 자동 분기, registry 의 4 컴포넌트 필수 |

**FU**:
- F1: `aria-live="polite"` 음성 알림 ("필지 패널 열림 depth 2/3")
- F2: per-kind staleTime 튜닝표 (필지=24h, 매물=5min, 실거래=1h ...)
- F3: RSC SSR pre-hydration (Next.js 16 RSC 의 server fetch + stream)

## 10. Acceptance Criteria

### 10.1. 컴파일·Lint

- `pnpm typecheck` (apps/web) 그린
- `pnpm lint` (apps/web) 그린 — 신규 lint rule 포함:
  - `panel/no-framework-imports-kind`: `lib/panel/**` 가 `components/panels/**` import 시 에러
  - `panel/no-direct-codec`: `panel/codec.ts` 외에서 `string.split('>')` 패턴 검출 시 경고
  - `panel/no-state-without-router`: zustand 의 panelStack 직접 set 검출 시 에러

### 10.2. 자동 회귀

- e2e (Playwright):
  - 폴리곤 클릭 → Panel 1 (parcel.summary) → 매물 link → Panel 2 (listing.summary) → 브라우저 back → Panel 1 복귀 → 새로고침 → Panel 1 그대로 + 컨텐츠 동일
  - 모바일 viewport (375×667) — 동일 시퀀스가 full-screen + ‹back 으로 작동
  - depth 3 만들고 breadcrumb 의 "회색 항목" 클릭 → 그 지점까지 stack pop 확인
  - URL 직접 navigate 로 depth 2 hydration 확인
  - 깨진 URL `?p=invalid:bad.thing` → silent 회복 (패널 0, Sentry 이벤트 1건)

- a11y: `axe-core` Playwright integration — Panel 1, 2, 3 각 스냅샷에서 위반 0
- 키보드 only navigation: Tab 으로 link 도달 가능, Enter 로 push, ESC 로 pop
- `prefers-reduced-motion: reduce` 환경에서 slide animation 0, cross-fade 만

### 10.3. SSS 확장성 회귀 테스트

`__tests__/panel-extensibility.test.ts` — 가짜 `mockKind` 를 `defineKind` 만으로 추가하고 push/pop/render 가 작동하는지 검증. 이 테스트가 framework 변경 없이 통과해야 SSS 확장성 lock.

### 10.4. 텔레메트리

- Sentry breadcrumb schema: `{ category: 'panel', message: 'opened', data: { kind, view, id, depth } }`
- OTEL span attribute: `panel.kind`, `panel.view`, `panel.id`, `panel.depth`, `panel.fetch_ms`
- analytics event: `panel_opened` with same attrs

## 11. 통합 변경 (existing code)

| 파일 | 변경 |
|---|---|
| `apps/web/components/listings/parcel-info-panel.tsx` | **삭제** |
| `apps/web/app/(authenticated)/listings/page.tsx` | aside 의 `<ParcelInfoPanel/>` → `<PanelRenderer/>` |
| `apps/web/components/listings/listing-map.tsx` | polygon click → `pushPanel({kind:'parcel', id:pnu, view:'summary'})`; marker click → `pushPanel({kind:'listing', id:listing.id, view:'summary'})` |
| `apps/web/app/(authenticated)/listings/[id]/page.tsx` | `redirect('/listings?p=listing:${id}.summary')` |
| `apps/web/stores/listings.ts` | `filters.pnu` 제거, `selectedListingId` 제거 (둘 다 stack 에서 derive) |
| `apps/web/components/listings/listing-card-list.tsx` | filter 를 `usePanelStack` derive 로 변경 |
| `apps/web/components/listings/listing-card.tsx` | onClick → `pushPanel`. `<Link to /listings/[id]>` 는 middle-click 만 (새 탭) — 다만 어차피 redirect 로 panel 으로 옴 |

## 12. v1 신규 파일

```
apps/web/lib/panel/
├── types.ts                    PanelKind / PanelView / PanelStack / PanelStackEntry
├── codec.ts                    PanelStackCodec interface + g1 impl + Result
├── codec.test.ts               serialize/deserialize 회귀
├── registry.ts                 defineKind + register imports
├── use-panel-stack.ts          URL ↔ zustand 동기 (router.push 로만 mutate)
├── panel-renderer.tsx          xl breakpoint switch
├── side-by-side-stack.tsx      desktop renderer
├── full-screen-stack.tsx       mobile renderer
├── panel-card.tsx              4-state shell, focus trap, ESC handler, error boundary
├── breadcrumb.tsx              sliding window 회색 항목 + mobile back
├── focus-trap.ts               focus push / restore hook
└── telemetry.ts                Sentry / OTEL / analytics helper

apps/web/components/panels/parcel/
├── summary.tsx
├── buildings.tsx
├── listings.tsx
├── skeletons.tsx               Loading / Error / Empty 컴포넌트 모음
└── register.ts                 defineKind('parcel', {...})

apps/web/components/panels/listing/
├── summary.tsx
├── skeletons.tsx
└── register.ts                 defineKind('listing', {...})

apps/web/messages/ko.json        + panels.parcel.* / panels.listing.* namespace

apps/web/lib/api/
├── parcels.ts                  GET /api/parcels/:pnu client
└── buildings.ts                GET /api/buildings?... client

apps/web/eslint-rules/           (or .eslintrc 의 inline rules)
└── panel-rules.cjs              위 § 10.1 의 3개 rule

services/api/src/routes/
├── parcels.rs                   GET /api/parcels/:pnu (parcel-lookup 호출)
└── buildings.rs                 GET /api/buildings (data-go-kr building reader 호출)

apps/web/tests/e2e/
└── panel-system.spec.ts        § 10.2 e2e 시나리오

apps/web/tests/unit/
└── panel-extensibility.test.ts § 10.3 SSS 확장성 회귀
```

## 13. Task 분해 (plan T1~T6)

| T | 작업 | 검증 |
|---|---|---|
| T1 | `lib/panel/` framework 본체 (types, codec, use-panel-stack, panel-renderer, panel-card, breadcrumb, telemetry) | unit: codec 회귀 / focus trap / ESC handler |
| T2 | side-by-side + full-screen renderer + breakpoint switch | Playwright 모바일 viewport 회귀 |
| T3 | backend endpoint 신규 — `/api/parcels/:pnu` + `/api/buildings` | wiremock + integration |
| T4 | `parcel` kind 등록 + 3 view (summary / buildings / listings) + i18n | e2e: 폴리곤 click → Panel 1 |
| T5 | `listing` kind 등록 + summary view + `/listings/[id]` redirect | e2e: marker click → Panel 1 / Panel 2 chain |
| T6 | 통합 변경 (filters.pnu / selectedListingId 제거, ParcelInfoPanel 삭제) + extensibility 회귀 테스트 + a11y/Lighthouse 검증 | § 10 acceptance 전체 |

추정: 1주 (5 영업일).

## 14. 리스크 → 완화

| 리스크 | 완화 |
|---|---|
| URL = SSOT 원칙이 zustand 사본 driven 으로 망가짐 | lint rule + e2e (URL 새로고침 시 stack 동일) |
| Next.js 16 router back 동작이 panel stack 과 어긋남 | usePanelStack 이 useSearchParams 만 읽고 useState 미사용 — router 가 단일 source |
| 모바일 fullscreen 모드에서 지도 인터랙션 못 함 | full-screen panel 이 close 시 지도 viewport 보존 — bounds 가 store 에 있어 자동 |
| 새 kind 추가 시 framework 깨짐 → SSS 확장성 lost | extensibility 회귀 테스트 (mockKind) 가 PR 마다 강제 |
| URL 길이 폭발 (depth 10+) | depth max 8 hard limit (warn at 6) — 실 사용 case 에서 depth 4 이상 거의 없음 |

## 15. 후속 (FU)

| # | 항목 | 트리거 |
|---|---|---|
| F1 | aria-live 음성 알림 | a11y audit 사용자 피드백 |
| F2 | per-kind staleTime 튜닝표 | 사용 패턴 데이터 수집 후 |
| F3 | RSC SSR pre-hydration | Next.js 16 RSC 안정화 + 첫 paint 측정 |
| F4 | tear-off (다중 stack) | 사용자 요청 시 |
| F5 | inline edit panel (action mode) | 매물 등록 / 수정 흐름 통합 시 |
| F6 | panel funnel 분석 대시보드 | Sentry/OTEL 데이터 누적 후 |
| F7 | building / broker / realTransaction / industrialComplex / admin / officialLandPrice kind 추가 | 각 도메인 데이터 source 준비되면 registry entry 1줄로 |

## 16. SSS 7기둥 매핑 (final)

| 기둥 | 본 spec 내 보장 |
|---|---|
| 일관성 | 모든 클릭 = `pushPanel`. 모든 뒤로 = `router.back`. xl breakpoint = `<PanelRenderer>` 한 곳 |
| 자동 강제 | 17 production rules 의 강제 column 모두 컴파일 또는 CI 차단 |
| 추적성 | URL = SSOT, Sentry breadcrumb, OTEL trace, analytics event |
| 안전성 | error boundary, AbortController, AuthGate, focus trap, ESC, 4-state shell |
| 가시성 | Sentry / OTEL / Grafana SLO, breadcrumb sliding window |
| SSOT | URL state SSOT, registry 1 파일, 컴포넌트 사본 0, codec interface 단일 |
| 명확성 | grep 가능한 URL, registry 한 파일에서 모든 kind/view 답, type-safe i18n |

---

**다음 단계**: 본 spec → spec-document-reviewer → 사용자 review → `writing-plans` skill → T1~T6 implementation plan.
