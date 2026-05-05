# Sub-project 6-foundation: Frontend 기초 (Next.js 16 + shadcn/ui + Auth + API client)

> **작성일**: 2026-05-05
> **이전 sub-project**: SP7-iii (정부 API drift 자동 검출, `b466c3a`)
> **SP6 시리즈의 첫 sub-project — 디자인 일관성 보장 foundation**
> **상태**: 디자인 — implementation plan 작성 대기

---

## 1. 배경 및 문제

### 1.1 Frontend 0 — engineer-only product

Backend 33 crate, ~1278 tests, 4 CI workflow 가 갖춰졌지만 **사용자 화면 0**.
production 의 절반 = frontend. 비즈니스 critical path.

### 1.2 SP6 분해 단위에서 SP6-foundation 의 역할

```
SP6-foundation (본 sub-project)        ← 인프라 only (디자인 + API client + i18n)
  ↓
SP6-i: auth flow + 화면 (login/signup/profile + OIDC + RBAC)
SP6-ii: 매물 검색 + 지도 (Naver Maps)
SP6-iii: 매물 상세 + 북마크
SP6-iv: 매물 등록 (broker 전용)
SP6-v: 알림
```

**왜 foundation 이 먼저:** vertical slice (SP6-i 부터) 만 진행하면 화면별 디자인 / API client 패턴이 파편화. **SP6-foundation 이 디자인 일관성 + 미래 디자인시스템 swap 가능 구조 보장.**

**왜 auth 분리:** Single Responsibility — foundation = "모든 화면이 의존하는 인프라", SP6-i = "auth flow + 화면". 통합 시 sub-project 의도 모호 + 점진 ship 어려움 (foundation 끝 시점 사용자 가치 0). 분리 시 foundation 끝 → /healthz smoke ship, SP6-i 끝 → /login 화면 ship — 단계별 사용자 가치.

### 1.3 미래 디자인 시스템 swap 의도

사용자 요구: "처음엔 새로 만들고 나중에 별도 디자인시스템으로 교체" (memory: project_design_system.md).
별도 폴더 `gongzzang/gongzzang/apps/gongzzang-design-lab` 가 실험실 (참고 안 함, 새로 시작).

→ **SP6-foundation 이 swap-able 구조 설계** (토큰 + 헤드리스 컴포넌트 분리).

---

## 2. 목표

### 2.1 핵심 목표

1. **monorepo 통합** — `gongzzang_2` 의 Cargo workspace 옆에 pnpm workspace 추가 (Turborepo orchestration)
2. **Next.js 16 + React 19 setup** — App Router 기반 첫 화면 (smoke test)
3. **swap-able 디자인 시스템 토대** — `packages/ui/` (shadcn/ui 코드 흡수) + 토큰 분리 + Pretendard 한국어 폰트
4. **API client SSOT 활용** — `utoipa` (Rust) → `openapi-typescript` 자동 TS types → `ky` 호출
5. **i18n 라이브러리 선택** — next-intl (한국어 1언어, 미래 swap 가능)
6. **에러 + 로딩 UX 일관성** — `error.tsx` / `not-found.tsx` / `loading.tsx` 패턴 표준화 (한국어 fallback)
7. **a11y 자동 검증** — `eslint-plugin-jsx-a11y` (lint) + `@axe-core/playwright` (e2e) — WCAG 2.1 AA 기본
8. **Sentry 자리 (SP7-i 통합 지점)** — `apps/web/instrumentation.ts` 빈 파일 (미래 SP7-i 가 채움)
9. **Bundle size budget** — Next.js `experimental.bundlePagesRouterDependencies` + size-limit script (production 진입 전 critical)
10. **CI 통합** — `.github/workflows/frontend.yml` (lint / typecheck / unit / e2e + a11y)
11. **smoke test** — `/healthz` backend 호출 e2e (unauthenticated)

### 2.2 비목표 (SP6-i 이후)

- **Auth flow + 화면** — SP6-i (login/signup/profile + OIDC PKCE + iron-session + RBAC + middleware)
- 매물 검색/상세/등록/북마크/알림 — SP6-ii ~ v
- Naver Maps 통합 — SP6-ii
- design-lab 의 디자인 토큰 이식 — 사용자 명시적 무시
- Storybook — 1인 단계 over-engineered (e2e + Vitest 충분)
- PWA / offline / Service Worker — YAGNI (production 진입 후 사용 패턴 보고 결정)
- SSR cache / ISR — 매물 화면 (SP6-ii) 단계에서 결정

### 2.3 SP6 시리즈 안의 위치

```
[Backend SSOT — Rust + utoipa]
   ↓
[OpenAPI spec 자동 생성]
   ↓
[openapi-typescript: TS types]
   ↓
SP6-foundation (본 sub-project)
   ├── apps/web/lib/api.ts (ky client + types)
   ├── packages/ui/ (swap-able 컴포넌트)
   └── auth + i18n + state foundation
   ↓
SP6-i ~ SP6-v: 화면별 vertical slice (foundation 위에 빠른 빌드)
```

---

## 3. SSS 7기둥 매칭

| 기둥 | 보장 방법 |
|---|---|
| **1 일관성** | shadcn primitives + Tailwind tokens 한 곳. 모든 SP6-i~v 가 같은 컴포넌트 사용. utoipa → TS types 자동 (수동 sync 0) |
| **2 자동 강제** | Biome + TypeScript strict + Vitest + Playwright + CI 자동. openapi-typescript script 가 backend 변경 시 fail |
| **3 추적성** | apps/web/api/proxy 가 모든 backend 호출 → audit_log 자동 (backend 가 이미 기록) |
| **4 안전성** | TS strict + zod 검증 + httpOnly cookie + CSRF + Zitadel JWT verify (backend) |
| **5 가시성** | Sentry 통합 자리 (SP7-i) + e2e Playwright + smoke test |
| **6 SSOT** | 1) utoipa → TS types (API 계약 SSOT), 2) Tailwind theme + CSS vars (디자인 토큰 SSOT), 3) Zitadel (auth SSOT) |
| **7 명확성** | docs/frontend/README + 한국어 UI 컨벤션 (해요체) helper + 명시적 swap point |

---

## 4. Scope

### 4.1 포함

- pnpm workspace + Turborepo 설정
- `apps/web/` Next.js 16 + React 19 + Tailwind 4 + Biome
- `packages/ui/` shadcn 핵심 6 컴포넌트 (Button / Input / Card / Modal / Form / Toast)
- `packages/ui/tokens/` CSS vars (color / spacing / typography) + Pretendard 한국어 폰트
- `apps/web/app/error.tsx` + `not-found.tsx` + `loading.tsx` 패턴 (한국어 fallback)
- `apps/web/lib/i18n/` next-intl ko-KR + 해요체 utils
- `apps/web/app/api/proxy/[...path]/route.ts` 단순 proxy (auth 검증 skeleton — 실제 검증은 SP6-i)
- `apps/web/lib/api.ts` ky + openapi-typescript types
- `packages/api-types/scripts/generate.ts` (utoipa OpenAPI → TS)
- TanStack Query (staleTime 30s default)
- Zustand 빈 store skeleton (interface 분리, 미래 swap)
- `apps/web/instrumentation.ts` 빈 파일 (SP7-i Sentry 자리)
- `apps/web/app/page.tsx` /healthz smoke 호출 화면 1개 (unauthenticated)
- Vitest + Testing Library + Playwright 설정
- `eslint-plugin-jsx-a11y` (lint) + `@axe-core/playwright` (e2e) — a11y 자동 검증
- size-limit script (bundle size budget, CI 게이트)
- `.github/workflows/frontend.yml`
- `docs/frontend/README.md` 운영 SSOT

### 4.2 미포함 (SP6-i ~ v 이후)

- **Auth flow + 화면 — SP6-i** (login/signup/profile + Zitadel OIDC PKCE + iron-session + middleware + /api/auth/callback + RBAC 일반/broker/admin)
- 매물 검색 / 상세 / 등록 / 북마크 / 알림 — SP6-ii ~ v
- Naver Maps SDK 통합 — SP6-ii
- design-lab 의 디자인 토큰 이식 — 명시적 X (사용자 의도)
- Sentry 통합 — SP7-i (instrumentation.ts 자리만 명시)
- Storybook — over-engineered (1인 단계, e2e + Vitest 충분)
- PWA / Service Worker / offline — YAGNI (production 후 사용 패턴 보고 결정)
- SSR cache / ISR / streaming — SP6-ii (매물 화면) 에서 결정

---

## 5. 아키텍처

### 5.1 큰 그림

```
[Browser]
   ↓ HTTPS
[Next.js 16 App Router (apps/web)]
   ├── app/page.tsx                    ← /healthz smoke (이 sub-project)
   ├── app/error.tsx                   ← 에러 boundary (한국어 fallback)
   ├── app/not-found.tsx               ← 404 화면
   ├── app/loading.tsx                 ← 로딩 skeleton 표준
   ├── app/api/proxy/[...path]/route.ts ← backend proxy (auth 검증 skeleton — SP6-i 가 채움)
   ├── instrumentation.ts              ← Sentry 자리 (SP7-i 가 채움)
   ├── components/                     ← composite (미래 SP6-i~v)
   ├── lib/
   │   ├── api.ts                      ← ky client + openapi-typescript types
   │   ├── i18n/                       ← next-intl ko-KR + 해요체
   │   └── query.ts                    ← TanStack Query client
   └── stores/                         ← Zustand
        ↓
[packages/ui]
   ├── primitives/                     ← Button/Input/Card/Modal/Form/Toast (shadcn 코드)
   ├── tokens/                         ← CSS vars (color/spacing/typography) + Pretendard
   └── (미래 packages/design-system/ ← 자리만)
        ↓ (build proxy)
[services/api on Axum]                  ← backend (이미 갖춰짐)
   └── /healthz, /users/me, /listings/*, etc
```

**SP6-i 가 추가할 것 (foundation 의 자리들):**
- `apps/web/lib/auth/` (OIDC PKCE + iron-session)
- `apps/web/middleware.ts` (protected routes)
- `apps/web/app/(auth)/login/page.tsx` + `/api/auth/callback/route.ts`
- `apps/web/app/api/proxy/[...path]/route.ts` 의 cookie 검증 채우기

### 5.2 Frontend → Backend 호출 흐름 (foundation 단계 — auth skeleton)

```
[Browser] /api/proxy/healthz   (foundation: unauthenticated)
   ↓
[Next.js Route Handler — proxy]
   ├── (SP6-i 가 채움) iron-session cookie 검증
   ├── (SP6-i 가 채움) Authorization: Bearer <jwt>
   └── ky → services/api/healthz
        ↓
[services/api]
   └── /healthz → "OK"
```

**왜 backend proxy 인가:**
- 사용자가 직접 services/api 호출 X — frontend 가 알아야 할 것 0
- API 키 / secrets 모두 server-side (`apps/web/lib/api.ts` server-only)
- AGENTS.md § 1 (API 키 하드코딩 금지) 준수
- 미래 SSR / Cache layer 추가 자유
- foundation 단계: proxy skeleton 만 (auth 미통합) — SP6-i 가 cookie 검증 추가

### 5.3 디자인 시스템 swap-able 구조

```
[packages/ui/tokens/colors.css]    ← --color-brand-500: #...; (CSS vars)
       ↓ (referenced by)
[packages/ui/primitives/button.tsx] ← className="bg-[var(--color-brand-500)]"
       ↓ (used by)
[apps/web/components/PropertyCard.tsx]

[미래: packages/design-system 들어옴]
       ↓ (단 토큰만 교체)
[packages/ui/tokens/colors.css]    ← 새 디자인시스템 토큰
                                      나머지 컴포넌트 코드 변경 0
```

---

## 6. Stack 결정 (16개)

### 6.1 Core (1-3)

| # | 영역 | 선택 | 근거 |
|---|---|---|---|
| 1 | Framework | Next.js 16 + React 19 | AGENTS.md 명시 |
| 2 | Language | TypeScript 5 strict | 안전성 |
| 3 | Styling | Tailwind 4 (CSS-first) | shadcn 표준, swap-able |

### 6.2 UI (4-6)

| # | 영역 | 선택 | 근거 |
|---|---|---|---|
| 4 | Component lib | shadcn/ui (코드 복사 + Radix headless) | npm lock-in 0, swap-able |
| 5 | Icon | lucide-react | shadcn 표준 |
| 6 | Toast | sonner | shadcn 표준 |

### 6.3 Data + State (7-10)

| # | 영역 | 선택 | 근거 |
|---|---|---|---|
| 7 | API client | `ky` + `openapi-typescript` | utoipa → TS types 자동 (SSOT) ⭐ |
| 8 | Server cache | TanStack Query 5 | staleTime 30s default |
| 9 | Client state | Zustand 5 | 가벼움. store interface 분리 → 미래 swap (Jotai 등) |
| 10 | Form | react-hook-form + zod | TS friendly |

### 6.4 i18n + Font (11-12)

| # | 영역 | 선택 | 근거 |
|---|---|---|---|
| 11 | i18n | `next-intl` ko-KR | 1 언어 시작, 미래 swap |
| 12 | Font | Pretendard (한국어 webfont) + system fallback | 한국 production-grade 표준 |

**참고:** OIDC + Session 은 SP6-i 의 결정 사항 (auth flow). 본 sub-project 는 proxy skeleton 만.

### 6.5 Test + Tooling + a11y (13-18)

| # | 영역 | 선택 | 근거 |
|---|---|---|---|
| 13 | Unit test | Vitest + Testing Library | 표준 |
| 14 | E2E | Playwright | 표준 |
| 15 | a11y lint | `eslint-plugin-jsx-a11y` | WCAG 2.1 AA lint |
| 16 | a11y e2e | `@axe-core/playwright` | WCAG 2.1 AA 자동 검증 |
| 17 | Bundle budget | `size-limit` script + Next.js bundle analyzer | production 진입 전 critical |
| 18 | Lint + Format | Biome | AGENTS.md 명시 |
| (Monorepo) | pnpm workspace + Turborepo | Cargo workspace 공존, build cache | (이미 § 6 # 의 일부) |

**참고:** 16개 → 18개로 보강 (a11y + bundle budget 추가).

---

## 7. 디렉토리 구조

```
gongzzang_2/
├── Cargo.toml                          (Rust workspace, 기존)
├── pnpm-workspace.yaml                 (NEW — js workspace 정의)
├── turbo.json                          (NEW — Turborepo 설정)
├── package.json                        (NEW — root, workspace scripts)
├── biome.jsonc                         (기존, frontend 도 cover)
│
├── apps/
│   └── web/                            (NEW)
│       ├── app/
│       │   ├── layout.tsx              (root layout — i18n + theme + Pretendard)
│       │   ├── page.tsx                (/healthz smoke 화면)
│       │   ├── error.tsx               (전역 에러 boundary, 한국어 fallback)
│       │   ├── not-found.tsx           (404 화면)
│       │   ├── loading.tsx             (전역 로딩 skeleton)
│       │   └── api/
│       │       └── proxy/
│       │           └── [...path]/route.ts  (backend proxy — auth 검증 skeleton, SP6-i 가 채움)
│       ├── instrumentation.ts          (빈 파일 — SP7-i Sentry 자리)
│       ├── components/                 (composite, 미래 SP6-i~v)
│       ├── lib/
│       │   ├── api.ts                  (ky + types)
│       │   ├── i18n/
│       │   │   ├── ko.json
│       │   │   └── haeyo.ts            (해요체 utils)
│       │   ├── query.ts                (TanStack Query client)
│       │   └── env.ts                  (zod env validation)
│       ├── stores/                     (Zustand skeleton)
│       ├── tests/
│       │   ├── unit/                   (Vitest)
│       │   └── e2e/                    (Playwright)
│       ├── public/
│       ├── package.json
│       ├── next.config.ts
│       ├── tsconfig.json
│       ├── playwright.config.ts
│       └── vitest.config.ts
│
├── packages/
│   ├── ui/                             (NEW — shadcn 코드 위치)
│   │   ├── primitives/
│   │   │   ├── button.tsx
│   │   │   ├── input.tsx
│   │   │   ├── card.tsx
│   │   │   ├── modal.tsx
│   │   │   ├── form.tsx
│   │   │   └── toast.tsx
│   │   ├── tokens/
│   │   │   ├── colors.css              (CSS vars)
│   │   │   ├── spacing.css
│   │   │   └── typography.css
│   │   ├── index.ts
│   │   └── package.json
│   └── api-types/                      (NEW — utoipa → TS types)
│       ├── generated.ts                (openapi-typescript 출력, gitignored or committed)
│       ├── scripts/generate.ts         (build hook)
│       └── package.json
│
├── docs/
│   └── frontend/
│       └── README.md                   (NEW — 운영 SSOT)
│
└── .github/
    └── workflows/
        └── frontend.yml                (NEW — lint/type/unit/e2e)
```

---

## 8. 작업 단위 (T1-T4)

### T1: Monorepo + Next.js 16 setup
- `pnpm-workspace.yaml` + `turbo.json` + root `package.json`
- `apps/web/` Next.js 16 + React 19 + Tailwind 4 setup
- `packages/ui/` 빈 skeleton + `packages/api-types/` skeleton
- Cargo workspace 와 공존 (root .gitignore + biome.jsonc 갱신)
- Biome lint 통과
- T1 commit

### T2: shadcn 핵심 + tokens + i18n + 한국어 폰트 + UX 패턴
- `packages/ui/primitives/` 6 컴포넌트 (shadcn 코드 복사)
- `packages/ui/tokens/` CSS vars (color/spacing/typography) + Pretendard webfont @import
- Tailwind 4 `@theme` 가 tokens CSS vars 참조
- `apps/web/lib/i18n/` next-intl ko-KR + 해요체 utils
- `apps/web/app/error.tsx` 전역 에러 boundary (한국어 fallback)
- `apps/web/app/not-found.tsx` 404 화면 (해요체)
- `apps/web/app/loading.tsx` 전역 로딩 skeleton
- Vitest unit test (각 primitive 의 render + accessibility)
- T2 commit

### T3: API client + TanStack Query + proxy skeleton + Sentry 자리
- `packages/api-types/scripts/generate.ts` (utoipa OpenAPI → openapi-typescript)
- `apps/web/lib/api.ts` ky client (typed)
- `apps/web/app/api/proxy/[...path]/route.ts` backend proxy (auth 검증 skeleton — SP6-i 가 채움)
- `apps/web/lib/query.ts` TanStack Query client (staleTime 30s)
- `apps/web/stores/` Zustand skeleton (interface only, swap-able)
- `apps/web/instrumentation.ts` 빈 파일 (Sentry 자리, SP7-i 가 채움)
- `apps/web/lib/env.ts` zod env validation
- Vitest unit test (api helpers)
- T3 commit

### T4: CI + a11y + bundle budget + smoke + docs
- `apps/web/app/page.tsx` /healthz 호출 화면 (smoke + 한국어 UI)
- Playwright e2e 1건 (홈 → /api/proxy/healthz → "OK" 표시)
- `eslint-plugin-jsx-a11y` 추가 + Biome 또는 ESLint 통합
- `@axe-core/playwright` e2e 통합 (홈 페이지 a11y 검증)
- `size-limit` script + budget 설정 (production bundle < 200KB JS gzipped)
- `.github/workflows/frontend.yml` (lint / type / unit / e2e + a11y + bundle budget)
- `docs/frontend/README.md` 운영 SSOT (디렉토리 / stack / 시작법 / swap path / 한국어 컨벤션)
- 기존 CI workflow 와 통합 (concurrency / 정합)
- `roadmap.md` 갱신 (SP6-foundation ✅ + SP6-i ~ v 자리)
- T4 commit + push

---

## 9. 검증 / 테스트 전략

### 9.1 Unit (Vitest)
- packages/ui/primitives/ 6 컴포넌트 render + accessibility
- apps/web/lib/api.ts (mock ky)
- apps/web/lib/i18n/haeyo.ts (해요체 utils)
- apps/web/lib/env.ts (zod validation)

### 9.2 E2E (Playwright + axe)
- 홈 페이지 → /api/proxy/healthz → backend /healthz → "OK" 표시 (smoke)
- 홈 페이지 a11y 검증 (`@axe-core/playwright` — WCAG 2.1 AA 위반 0)
- error.tsx fallback 검증 (의도된 에러 → 한국어 fallback 표시)
- not-found.tsx 검증 (404 페이지 → "찾을 수 없어요")
- (미래 SP6-i 추가) login flow

### 9.3 a11y 자동 검증 (CI 게이트)
- `eslint-plugin-jsx-a11y` (lint 시점) — alt 누락 / button label / form label 등
- `@axe-core/playwright` (e2e 시점) — runtime accessibility tree 검증
- 둘 다 fail 시 CI fail

### 9.4 Bundle size budget (CI 게이트)
- `size-limit` script — production bundle < 200KB JS gzipped
- Next.js bundle analyzer 보고 (수동 확인용)
- budget 초과 시 CI fail → bundle 분석 + 의존성 정리

### 9.5 CI workflow
- `.github/workflows/frontend.yml`:
  - Biome lint
  - TypeScript typecheck
  - Vitest unit
  - Playwright e2e + a11y (chromium 만)
  - size-limit bundle budget
  - openapi-typescript 자동 생성 (utoipa → TS, 변경 시 PR fail)

### 9.6 Manual smoke (구현 후)
1. `pnpm dev` → http://localhost:3000 → "OK"
2. `pnpm build` → production build 성공 + bundle budget 통과
3. `pnpm test:e2e` → Playwright pass
4. axe DevTools 브라우저 확장으로 수동 a11y 검증 (보강)

---

## 10. Migration / Swap path

### 10.1 디자인 시스템 swap (미래)

```
[현재] packages/ui/tokens/colors.css 가 SSOT
       packages/ui/primitives/button.tsx 가 var(--color-brand-500) 참조
       ↓
[미래 packages/design-system 들어옴]
       packages/design-system/ 가 새 토큰 정의 (예: 새 brand color)
       packages/ui/tokens/ 가 design-system import (단순 re-export)
       ↓
[결과]
       primitives 코드 변경 0
       apps/web/components 코드 변경 0
       토큰만 교체 → 전체 UI 재 spectrum
```

### 10.2 OIDC provider swap

`oidc-client-ts` 는 표준 — Zitadel → Auth0 / Keycloak swap 시 환경변수 (issuer / audience / clientId) 만 변경.

### 10.3 i18n 영문 추가

`apps/web/lib/i18n/en.json` 추가 + next-intl middleware 설정 → 한국어/영문 toggle.

### 10.4 SP7-i Sentry 통합 시점

`apps/web/instrumentation.ts` (Next.js 16 표준) 에 Sentry SDK 추가. SP6-foundation 의 코드 변경 0.

---

## 11. Follow-up

### 11.1 본 sub-project 가 closing 하는 FU

- (없음 — frontend 가 첫 etablish)

### 11.2 본 sub-project 에서 발견 가능

- **FU 48 (예상)**: utoipa OpenAPI 자동 생성 — Rust 측 (services/api) 에 utoipa 가 미통합. SP6-foundation T3 에서 발견 시 SP6-foundation 안 또는 별도 sub-project
- **FU 49 (예상)**: shadcn primitives 의 한국어 라벨 표준화 (해요체) — SP6-i 에서 사용 시 발견

### 11.3 미흡수 (다른 sub-project)

- 사용자 RBAC 분리 (일반/broker/admin) — SP6-i
- 매물 검색 + Naver Maps — SP6-ii
- 매물 등록 (broker UI) — SP6-iv
- design-lab 디자인 토큰 — 사용자 명시적 X
- Sentry 통합 — SP7-i

---

## 12. 추정

- **작업량**: 3-4일 (T1-T4)
- **신규 디렉토리**: 3 (`apps/web`, `packages/ui`, `packages/api-types`)
- **신규 workflow**: 1 (`frontend.yml`)
- **신규 docs**: 1 (`docs/frontend/README.md`)
- **누적 통계 변화**: 33 crate 그대로 (Rust 코드 변경 0). JS workspace 추가 — 첫 frontend foundation

---

## 13. SSS 자가 평가

| 기둥 | 보장 | 정도 |
|---|---|---|
| 1 일관성 | shadcn primitives + Tailwind tokens 한 곳 + utoipa types 자동 + error/loading/not-found 표준 + Pretendard 폰트 | ◎ |
| 2 자동 강제 | Biome + TS strict + Vitest + Playwright + a11y lint + axe e2e + size-limit bundle + CI fail on backend drift | ◎ |
| 3 추적성 | proxy 통과 → backend audit_log 자동 + Sentry 자리 (SP7-i) | ◎ |
| 4 안전성 | TS strict + zod env + a11y 자동 검증 (WCAG 2.1 AA) + bundle size budget | ◎ |
| 5 가시성 | smoke test + e2e + a11y 자동 검증 + Sentry 자리 (SP7-i) | ◎ |
| 6 SSOT | 1) utoipa → TS, 2) Tailwind theme + CSS vars (디자인 토큰), 3) (SP6-i 후) Zitadel | ◎ |
| 7 명확성 | docs/frontend + 해요체 helper + swap point 명시 + YAGNI 명시 (PWA/Storybook) + 한국어 fallback | ◎ |

= **근본 SSS 80%+ 달성 (foundation 단계 — auth 분리 후)**.

**Auth 분리의 SSS 효과:**
- foundation 의 단일 책임 (인프라만) → 명확
- SP6-i = auth flow 단독 sub-project → 변경 boundary 명확
- 점진 ship — foundation 끝 = smoke, SP6-i 끝 = login 화면

향후 SP6-i ~ v 가 이 foundation 위에서 일관성 100% 보장.

---

## 14. 핵심 결정 요약 (chronological)

1. **SP6 분해 = 옵션 1 (vertical slice)** + **foundation 먼저** (디자인 일관성)
2. **Auth 는 SP6-i 로 분리** (사용자 직관) — Single Responsibility, 점진 ship
3. **design-lab 무시** — 사용자 명시적 (memory:project_design_system.md 참조용)
4. **shadcn/ui** — swap-able, npm lock-in 0
5. **utoipa → openapi-typescript** — SSOT 활용 (AGENTS.md § 8)
6. **Backend proxy** — Next.js Route Handler (frontend 에 secrets 0). foundation 단계 = skeleton, SP6-i 가 cookie 검증 채움
7. **Turborepo** — Cargo workspace 공존 + build cache
8. **next-intl** — ko-KR 1 언어, 미래 swap
9. **Zustand** — store interface 분리 (미래 Jotai swap 가능)
10. **components/primitives 분리** — packages/ui = primitive, apps/web/components = composite
11. **에러/로딩 UX 표준화** — error.tsx / not-found.tsx / loading.tsx 한국어 fallback
12. **a11y 자동 검증** — eslint-plugin-jsx-a11y + @axe-core/playwright (WCAG 2.1 AA 게이트)
13. **Sentry 자리 명시** — instrumentation.ts 빈 파일, SP7-i 가 채움
14. **Pretendard 한국어 폰트** — packages/ui/tokens 안에 webfont @import
15. **Bundle size budget** — size-limit < 200KB JS gzipped, CI 게이트
16. **PWA / Service Worker — YAGNI 명시** — production 후 사용 패턴 보고 결정
