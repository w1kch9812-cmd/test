# Frontend (apps/web) — SP6-foundation

> **목적**: 공짱 frontend 인프라. 모든 SP6-i ~ v 화면이 의존하는 foundation.
> **Stack**: Next.js 16 + React 19 + TypeScript strict + Tailwind 4 + shadcn/ui (코드 복사)
> **Spec**: `docs/superpowers/specs/2026-05-05-sub-project-6-foundation-design.md`

## 시작

```bash
# 의존성 설치 (root 에서)
pnpm install

# dev server (apps/web)
pnpm --filter=@gongzzang/web dev
# 또는
pnpm dev

# 브라우저: http://localhost:3000
```

## 디렉토리 구조

- `apps/web/` — Next.js 16 App Router
  - `app/` — 라우트 (page.tsx / layout.tsx / error.tsx / not-found.tsx / loading.tsx)
  - `app/api/proxy/[...path]/route.ts` — backend proxy (auth 검증은 SP6-i)
  - `lib/` — utilities (api / query / env / i18n)
  - `stores/` — Zustand (skeleton, SP6-i ~ v 가 채움)
  - `instrumentation.ts` — Sentry placeholder (SP7-i 가 채움)
  - `tests/unit/` — Vitest
  - `tests/e2e/` — Playwright + axe
- `packages/ui/` — shadcn primitives + tokens (swap point)
  - `primitives/` — Button / Input / Card / Dialog / Form / Toaster
  - `tokens/` — CSS vars (color / spacing / typography + Pretendard webfont)
- `packages/api-types/` — utoipa OpenAPI → TypeScript types

## 주요 명령어

| 명령어 | 설명 |
|---|---|
| `pnpm dev` | 개발 서버 (turbo) |
| `pnpm build` | production 빌드 |
| `pnpm typecheck` | TypeScript 검증 |
| `pnpm test` | Vitest unit |
| `pnpm test:e2e` | Playwright e2e + a11y |
| `pnpm lint` | Biome lint |
| `pnpm format` | Biome format |
| `pnpm --filter=@gongzzang/web test:bundle` | size-limit bundle budget |
| `pnpm --filter=@gongzzang/api-types generate` | utoipa OpenAPI → TS types |

## 한국어 UI 컨벤션 (해요체)

- 사용자 노출 문자열은 모두 **해요체**:
  - "조회했어요", "잠시 후 다시 시도해 주세요", "로그인이 필요해요"
- 에러 메시지: **원인 + 대응 안내**
- 시간/숫자/면적 포맷: `apps/web/lib/i18n/haeyo.ts` utils 사용
- 다국어 자원: `apps/web/lib/i18n/ko.json` (next-intl)

## 디자인 시스템 swap path

```text
[현재] packages/ui/tokens/ — CSS vars (color/spacing/typography + Pretendard)
       ↓ Tailwind 4 @theme 가 var(--color-brand-500) 등 참조
       ↓ packages/ui/primitives/ 가 className 으로 사용

[미래] packages/design-system/ 도입
       ↓ packages/ui/tokens/ 만 design-system 의 토큰 re-export
       ↓ primitives 코드 변경 0 — 시각적 swap
```

## 검증 게이트

| 게이트 | 도구 | CI step |
|---|---|---|
| Lint | Biome | `pnpm lint` |
| TypeScript | tsc --noEmit | `pnpm typecheck` |
| Unit | Vitest | `pnpm test` |
| E2E | Playwright | `pnpm test:e2e` |
| a11y | @axe-core/playwright | `pnpm test:e2e` (a11y.spec.ts) |
| Bundle | size-limit | `pnpm --filter=@gongzzang/web test:bundle` |
| Format | Biome | `pnpm format --write` |

## 비목표 (다른 sub-project)

- **Auth flow + 화면** — SP6-i (login/signup/profile + Zitadel OIDC + iron-session + RBAC + middleware)
- 매물 검색/상세/등록/북마크/알림 — SP6-ii ~ v
- Naver Maps SDK — SP6-ii
- Sentry 통합 — SP7-i (instrumentation.ts 자리만 명시)
- PWA / offline — YAGNI
- Storybook — over-engineered

## 진화 path

| 시점 | 통합 |
|---|---|
| SP6-i (auth) | `apps/web/lib/auth/`, middleware.ts, /api/proxy 의 cookie 검증 |
| SP6-ii (지도) | Naver Maps SDK, /listings 화면 |
| SP7-i (Sentry) | `instrumentation.ts` 채움, `app/error.tsx` 에 capture |
| (미래) packages/design-system | `packages/ui/tokens/` 만 교체 |

## 참고

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-foundation-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-foundation.md`
- AGENTS.md: 프로젝트 헌법
