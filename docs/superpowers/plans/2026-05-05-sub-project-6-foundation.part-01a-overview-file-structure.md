# SP6 Foundation - Part 01A: Overview And File Structure

Parent index: [SP6 Foundation Part 01](./2026-05-05-sub-project-6-foundation.part-01.md).
# SP6-foundation Implementation Plan — Frontend 인프라

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Next.js 16 + React 19 + shadcn/ui + Tailwind 4 + TanStack Query + i18n 인프라 구축 — 모든 SP6-i ~ v 화면이 의존하는 foundation. 디자인 시스템 swap-able 구조 + WCAG 2.1 AA + bundle budget + 한국어 UX 표준.

**Architecture:** Monorepo (`pnpm workspace + Turborepo`) 위에 `apps/web` (Next.js App Router) + `packages/ui` (shadcn primitives + tokens) + `packages/api-types` (utoipa → TS). Backend 호출은 `/api/proxy/[...path]` Route Handler 통과. 4 task 분해 — T1 setup → T2 shadcn+i18n+UX → T3 API client+proxy → T4 CI+a11y+bundle+smoke.

**Tech Stack:** Next.js 16, React 19, TypeScript 5 strict, Tailwind 4 (CSS-first), shadcn/ui + Radix headless, lucide-react, sonner, ky + openapi-typescript, TanStack Query 5, Zustand 5, react-hook-form + zod, next-intl ko-KR, Pretendard 한국어 폰트, Vitest + Testing Library + Playwright + @axe-core/playwright + eslint-plugin-jsx-a11y, Biome (기존), pnpm workspace + Turborepo + size-limit.

**Spec:** `docs/superpowers/specs/2026-05-05-sub-project-6-foundation-design.md` (commit `a16875a`)

**main:** `a16875a` (시작 시점)

---

## 추천 진행 순서

- **T1**: Monorepo + Next.js 16 setup (pnpm workspace + Turborepo + apps/web + packages 스켈레톤) — 1 commit
- **T2**: shadcn 핵심 6 컴포넌트 + tokens + Pretendard + i18n + UX 패턴 (error/not-found/loading) — 1 commit
- **T3**: API client (ky + openapi-typescript) + TanStack Query + proxy skeleton + instrumentation.ts + Zustand skeleton — 1 commit
- **T4**: CI workflow + a11y + bundle budget + smoke 화면 + docs + roadmap — 1 commit

각 task: `pnpm typecheck && pnpm test && pnpm build` 통과 후 push → CI 그린 확인. 사용자 체크포인트.

---

## 파일 구조

```
gongzzang_2/
├── pnpm-workspace.yaml                 (T1 — NEW)
├── turbo.json                          (T1 — NEW)
├── package.json                        (T1 — NEW root)
├── .gitignore                          (T1 — modify, .next/.turbo 추가는 이미 있음 확인)
├── biome.json                          (T1 — modify, 필요 시 frontend rules)
│
├── apps/
│   └── web/                            (T1-T4 — NEW Next.js 16 app)
│       ├── package.json
│       ├── next.config.ts
│       ├── tsconfig.json
│       ├── postcss.config.mjs          (Tailwind 4)
│       ├── vitest.config.ts            (T2)
│       ├── playwright.config.ts        (T4)
│       ├── .size-limit.json            (T4)
│       ├── i18n.ts                     (T2 — next-intl config)
│       ├── instrumentation.ts          (T3 — empty Sentry placeholder)
│       ├── public/
│       │   └── (Pretendard 자리, 또는 webfont @import 사용)
│       ├── app/
│       │   ├── layout.tsx              (T1 placeholder → T2 + T3 modify)
│       │   ├── page.tsx                (T1 placeholder → T4 smoke)
│       │   ├── globals.css             (T1 + T2 — Tailwind + tokens)
│       │   ├── error.tsx               (T2 — 한국어 fallback)
│       │   ├── not-found.tsx           (T2)
│       │   ├── loading.tsx             (T2)
│       │   └── api/
│       │       └── proxy/
│       │           └── [...path]/
│       │               └── route.ts    (T3 — skeleton)
│       ├── lib/
│       │   ├── api.ts                  (T3 — ky)
│       │   ├── query.ts                (T3 — TanStack Query)
│       │   ├── env.ts                  (T3 — zod)
│       │   └── i18n/
│       │       ├── ko.json             (T2)
│       │       ├── haeyo.ts            (T2 — 해요체 utils)
│       │       └── request.ts          (T2 — next-intl getRequestConfig)
│       ├── stores/
│       │   └── index.ts                (T3 — Zustand skeleton)
│       └── tests/
│           ├── unit/
│           │   └── haeyo.test.ts       (T2)
│           └── e2e/
│               ├── healthz.spec.ts     (T4 — smoke)
│               └── a11y.spec.ts        (T4 — axe)
│
├── packages/
│   ├── ui/                             (T1-T2 — NEW)
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── index.ts                    (T1 empty → T2 re-exports)
│   │   ├── lib/
│   │   │   └── utils.ts                (T2 — cn helper)
│   │   ├── tokens/
│   │   │   ├── index.css               (T2)
│   │   │   ├── colors.css              (T2 — CSS vars)
│   │   │   ├── spacing.css             (T2)
│   │   │   └── typography.css          (T2 — Pretendard)
│   │   └── primitives/
│   │       ├── button.tsx              (T2)
│   │       ├── input.tsx               (T2)
│   │       ├── card.tsx                (T2)
│   │       ├── dialog.tsx              (T2 — Modal)
│   │       ├── form.tsx                (T2)
│   │       ├── sonner.tsx              (T2 — Toast)
│   │       └── index.ts                (T2)
│   └── api-types/                      (T1-T3 — NEW)
│       ├── package.json
│       ├── tsconfig.json
│       ├── index.ts                    (T1 empty → T3 re-export)
│       ├── generated/
│       │   └── schema.ts               (T3 — openapi-typescript output)
│       └── scripts/
│           └── generate.ts             (T3)
│
├── docs/
│   └── frontend/                       (T4 — NEW)
│       └── README.md                   (T4)
│
└── .github/
    └── workflows/
        └── frontend.yml                (T4 — NEW)
```

---
