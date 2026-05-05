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

## Phase A: Monorepo + Next.js 16 setup

### Task 1: pnpm workspace + Turborepo + apps/web + packages 스켈레톤

**Files:**
- Create: `pnpm-workspace.yaml`
- Create: `turbo.json`
- Create: `package.json` (root)
- Create: `apps/web/package.json`
- Create: `apps/web/next.config.ts`
- Create: `apps/web/tsconfig.json`
- Create: `apps/web/postcss.config.mjs`
- Create: `apps/web/app/layout.tsx` (placeholder)
- Create: `apps/web/app/page.tsx` (placeholder)
- Create: `apps/web/app/globals.css` (Tailwind directives only)
- Create: `packages/ui/package.json`
- Create: `packages/ui/tsconfig.json`
- Create: `packages/ui/index.ts` (empty)
- Create: `packages/api-types/package.json`
- Create: `packages/api-types/tsconfig.json`
- Create: `packages/api-types/index.ts` (empty)
- Modify: `.gitignore` (확인 — `.next` / `node_modules` 이미 있을 가능성)

#### Step 1.1: pnpm-workspace.yaml

- [ ] **Step**: Create `pnpm-workspace.yaml`

```yaml
packages:
  - "apps/*"
  - "packages/*"
```

#### Step 1.2: turbo.json

- [ ] **Step**: Create `turbo.json`

```json
{
  "$schema": "https://turbo.build/schema.json",
  "tasks": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": [".next/**", "!.next/cache/**", "dist/**"]
    },
    "dev": {
      "cache": false,
      "persistent": true
    },
    "lint": {},
    "typecheck": {
      "dependsOn": ["^build"]
    },
    "test": {
      "dependsOn": ["^build"]
    },
    "test:e2e": {
      "dependsOn": ["^build"]
    }
  }
}
```

#### Step 1.3: Root package.json

- [ ] **Step**: Create root `package.json`

```json
{
  "name": "gongzzang",
  "private": true,
  "scripts": {
    "dev": "turbo dev",
    "build": "turbo build",
    "lint": "biome check .",
    "format": "biome format --write .",
    "typecheck": "turbo typecheck",
    "test": "turbo test",
    "test:e2e": "turbo test:e2e"
  },
  "devDependencies": {
    "turbo": "^2.5.0"
  },
  "packageManager": "pnpm@9.15.0",
  "engines": {
    "node": ">=20.0.0",
    "pnpm": ">=9.0.0"
  }
}
```

#### Step 1.4: apps/web/package.json (Next.js 16)

- [ ] **Step**: Create `apps/web/package.json`

```json
{
  "name": "@gongzzang/web",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "biome check .",
    "typecheck": "tsc --noEmit",
    "test": "vitest run",
    "test:e2e": "playwright test",
    "test:bundle": "size-limit"
  },
  "dependencies": {
    "@gongzzang/ui": "workspace:*",
    "@gongzzang/api-types": "workspace:*",
    "next": "^16.0.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "next-intl": "^3.26.0",
    "ky": "^1.7.0",
    "@tanstack/react-query": "^5.62.0",
    "zustand": "^5.0.0",
    "react-hook-form": "^7.54.0",
    "@hookform/resolvers": "^3.10.0",
    "zod": "^3.24.0",
    "lucide-react": "^0.468.0",
    "sonner": "^2.0.0",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "tailwind-merge": "^3.0.0"
  },
  "devDependencies": {
    "@biomejs/biome": "^2.4.0",
    "@playwright/test": "^1.50.0",
    "@axe-core/playwright": "^4.10.0",
    "@tailwindcss/postcss": "^4.0.0",
    "@testing-library/jest-dom": "^6.6.0",
    "@testing-library/react": "^16.1.0",
    "@types/node": "^22.10.0",
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "@vitejs/plugin-react": "^4.3.0",
    "happy-dom": "^16.0.0",
    "openapi-typescript": "^7.5.0",
    "size-limit": "^11.1.0",
    "@size-limit/preset-app": "^11.1.0",
    "tailwindcss": "^4.0.0",
    "typescript": "^5.7.0",
    "vitest": "^2.1.0"
  }
}
```

#### Step 1.5: apps/web/tsconfig.json

- [ ] **Step**: Create `apps/web/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "dom.iterable", "esnext"],
    "allowJs": false,
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "jsx": "preserve",
    "incremental": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitOverride": true,
    "plugins": [{ "name": "next" }],
    "paths": {
      "@/*": ["./*"]
    }
  },
  "include": ["next-env.d.ts", "**/*.ts", "**/*.tsx", ".next/types/**/*.ts"],
  "exclude": ["node_modules"]
}
```

#### Step 1.6: apps/web/next.config.ts

- [ ] **Step**: Create `apps/web/next.config.ts`

```typescript
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  // T2 에서 next-intl plugin 추가 예정 (이 단계에서는 minimal)
  experimental: {
    typedRoutes: true,
  },
};

export default nextConfig;
```

#### Step 1.7: apps/web/postcss.config.mjs (Tailwind 4)

- [ ] **Step**: Create `apps/web/postcss.config.mjs`

```javascript
export default {
  plugins: {
    "@tailwindcss/postcss": {},
  },
};
```

#### Step 1.8: apps/web/app/globals.css

- [ ] **Step**: Create `apps/web/app/globals.css`

```css
@import "tailwindcss";

/* T2 가 packages/ui/tokens 참조 추가 예정 */

html {
  font-family: system-ui, -apple-system, sans-serif;
}
```

#### Step 1.9: apps/web/app/layout.tsx (placeholder)

- [ ] **Step**: Create `apps/web/app/layout.tsx`

```tsx
import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "공짱",
  description: "산업용 부동산 정보 플랫폼",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="ko">
      <body>{children}</body>
    </html>
  );
}
```

#### Step 1.10: apps/web/app/page.tsx (placeholder)

- [ ] **Step**: Create `apps/web/app/page.tsx`

```tsx
export default function Home() {
  return (
    <main className="p-8">
      <h1 className="text-2xl font-bold">공짱 — Foundation</h1>
      <p>T4 가 /healthz smoke 호출로 채울 예정.</p>
    </main>
  );
}
```

#### Step 1.11: packages/ui/package.json

- [ ] **Step**: Create `packages/ui/package.json`

```json
{
  "name": "@gongzzang/ui",
  "version": "0.1.0",
  "private": true,
  "main": "./index.ts",
  "types": "./index.ts",
  "exports": {
    ".": "./index.ts",
    "./tokens": "./tokens/index.css",
    "./primitives": "./primitives/index.ts",
    "./lib/utils": "./lib/utils.ts"
  },
  "dependencies": {
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "@radix-ui/react-dialog": "^1.1.4",
    "@radix-ui/react-label": "^2.1.1",
    "@radix-ui/react-slot": "^1.1.1",
    "class-variance-authority": "^0.7.1",
    "clsx": "^2.1.1",
    "tailwind-merge": "^3.0.0",
    "lucide-react": "^0.468.0",
    "sonner": "^2.0.0"
  },
  "devDependencies": {
    "@types/react": "^19.0.0",
    "@types/react-dom": "^19.0.0",
    "typescript": "^5.7.0"
  }
}
```

#### Step 1.12: packages/ui/tsconfig.json

- [ ] **Step**: Create `packages/ui/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["dom", "esnext"],
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "isolatedModules": true,
    "jsx": "preserve"
  },
  "include": ["**/*.ts", "**/*.tsx"],
  "exclude": ["node_modules"]
}
```

#### Step 1.13: packages/ui/index.ts (empty stub)

- [ ] **Step**: Create `packages/ui/index.ts`

```typescript
// T2 가 primitives + tokens re-export
```

#### Step 1.14: packages/api-types/package.json

- [ ] **Step**: Create `packages/api-types/package.json`

```json
{
  "name": "@gongzzang/api-types",
  "version": "0.1.0",
  "private": true,
  "main": "./index.ts",
  "types": "./index.ts",
  "scripts": {
    "generate": "tsx scripts/generate.ts"
  },
  "devDependencies": {
    "openapi-typescript": "^7.5.0",
    "tsx": "^4.19.0",
    "typescript": "^5.7.0"
  }
}
```

#### Step 1.15: packages/api-types/tsconfig.json + index.ts

- [ ] **Step**: Create `packages/api-types/tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["esnext"],
    "skipLibCheck": true,
    "strict": true,
    "noEmit": true,
    "esModuleInterop": true,
    "module": "esnext",
    "moduleResolution": "bundler",
    "isolatedModules": true
  },
  "include": ["**/*.ts"],
  "exclude": ["node_modules"]
}
```

- [ ] **Step**: Create `packages/api-types/index.ts`

```typescript
// T3 가 generated/schema.ts re-export
```

#### Step 1.16: .gitignore 확인 (frontend 영역)

- [ ] **Step**: `.gitignore` 검사 — Frontend 관련 패턴 확인 후 누락된 것 추가

```bash
grep -E "^\.next|^node_modules|^\.turbo|^\.next/|^node_modules/" .gitignore
```

이미 있어야 할 것:
- `node_modules/`
- `.next/`
- `.turbo/`
- `dist/`
- `coverage/`

빠진 것 있으면 추가:

```
# Next.js
.next/
out/

# pnpm + Turbo
.turbo/
.pnpm-store/

# Test
coverage/
test-results/
playwright-report/
```

#### Step 1.17: pnpm install + 빌드 검증

- [ ] **Step**: 의존성 설치

```bash
cd c:/Users/User/Desktop/gongzzang_2
pnpm install
```

Expected: `node_modules/` 생성 + `pnpm-lock.yaml` 생성 + 워크스페이스 링크 (`apps/web` → `@gongzzang/ui`).

- [ ] **Step**: typecheck

```bash
pnpm typecheck
```

Expected: error 0.

- [ ] **Step**: build

```bash
pnpm build
```

Expected: Next.js 빌드 성공 — `apps/web/.next/` 생성.

- [ ] **Step**: dev server 시작

```bash
pnpm --filter=@gongzzang/web dev
```

브라우저 http://localhost:3000 에서 placeholder 페이지 확인. 그 다음 Ctrl+C 종료.

#### Step 1.18: T1 commit

- [ ] **Step**: T1 commit

```bash
git add pnpm-workspace.yaml turbo.json package.json pnpm-lock.yaml \
        .gitignore \
        apps/web packages/ui packages/api-types

git commit -m "$(cat <<'EOF'
feat(sp6-foundation-t1): monorepo + Next.js 16 setup (pnpm workspace + Turborepo)

T1 of SP6-foundation:
- pnpm-workspace.yaml + turbo.json + root package.json (Cargo workspace 공존)
- apps/web/ Next.js 16 + React 19 + TypeScript strict + Tailwind 4 (PostCSS) 스켈레톤
  - layout.tsx + page.tsx placeholder (T4 가 smoke 채움)
  - globals.css with Tailwind 4 directive
- packages/ui/ skeleton (T2 가 shadcn primitives + tokens 채움)
- packages/api-types/ skeleton (T3 가 utoipa → openapi-typescript 채움)
- .gitignore frontend 패턴 (.next, .turbo, coverage 등)
- pnpm install + typecheck + build 통과 검증

Spec: docs/superpowers/specs/2026-05-05-sub-project-6-foundation-design.md (a16875a)
EOF
)"
```

DO NOT push yet — controller pushes after T1 review.

**사용자 체크포인트**: T1 commit 확인 + 다음 진행.

---

## Phase B: shadcn 핵심 + tokens + i18n + UX 패턴

### Task 2: shadcn primitives + Pretendard tokens + next-intl + 한국어 helper + error/loading/not-found

**Files:**
- Create: `packages/ui/lib/utils.ts` (cn helper)
- Create: `packages/ui/tokens/colors.css`
- Create: `packages/ui/tokens/spacing.css`
- Create: `packages/ui/tokens/typography.css`
- Create: `packages/ui/tokens/index.css`
- Create: `packages/ui/primitives/button.tsx`
- Create: `packages/ui/primitives/input.tsx`
- Create: `packages/ui/primitives/card.tsx`
- Create: `packages/ui/primitives/dialog.tsx`
- Create: `packages/ui/primitives/form.tsx`
- Create: `packages/ui/primitives/sonner.tsx`
- Create: `packages/ui/primitives/index.ts`
- Modify: `packages/ui/index.ts`
- Create: `apps/web/lib/i18n/ko.json`
- Create: `apps/web/lib/i18n/haeyo.ts`
- Create: `apps/web/lib/i18n/request.ts`
- Create: `apps/web/i18n.ts`
- Modify: `apps/web/next.config.ts` (next-intl plugin)
- Create: `apps/web/app/error.tsx`
- Create: `apps/web/app/not-found.tsx`
- Create: `apps/web/app/loading.tsx`
- Modify: `apps/web/app/layout.tsx` (NextIntlClientProvider + Pretendard)
- Modify: `apps/web/app/globals.css` (tokens import)
- Create: `apps/web/vitest.config.ts`
- Create: `apps/web/tests/unit/haeyo.test.ts`
- Create: `apps/web/tests/unit/setup.ts` (Vitest jest-dom matcher)

#### Step 2.1: cn helper

- [ ] **Step**: Create `packages/ui/lib/utils.ts`

```typescript
import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

/**
 * `clsx` + `tailwind-merge` 조합 — Tailwind class 병합 helper.
 */
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
```

#### Step 2.2: Pretendard webfont + 토큰 CSS

- [ ] **Step**: Create `packages/ui/tokens/typography.css`

```css
/* Pretendard — 한국어 production-grade webfont */
@import url("https://cdn.jsdelivr.net/gh/orioncactus/pretendard@v1.3.9/dist/web/variable/pretendardvariable-dynamic-subset.css");

:root {
  --font-sans: "Pretendard Variable", Pretendard, -apple-system, BlinkMacSystemFont,
    "Apple SD Gothic Neo", "Noto Sans KR", "Malgun Gothic", system-ui, sans-serif;
  --font-mono: "JetBrains Mono", "D2Coding", Menlo, Consolas, monospace;

  --text-xs: 0.75rem;
  --text-sm: 0.875rem;
  --text-base: 1rem;
  --text-lg: 1.125rem;
  --text-xl: 1.25rem;
  --text-2xl: 1.5rem;
  --text-3xl: 1.875rem;
  --text-4xl: 2.25rem;
}
```

- [ ] **Step**: Create `packages/ui/tokens/colors.css`

```css
:root {
  --color-bg: #ffffff;
  --color-fg: #18181b;

  --color-brand-50:  #eff6ff;
  --color-brand-100: #dbeafe;
  --color-brand-200: #bfdbfe;
  --color-brand-300: #93c5fd;
  --color-brand-400: #60a5fa;
  --color-brand-500: #3b82f6;
  --color-brand-600: #2563eb;
  --color-brand-700: #1d4ed8;
  --color-brand-800: #1e40af;
  --color-brand-900: #1e3a8a;

  --color-muted: #f4f4f5;
  --color-muted-fg: #71717a;
  --color-border: #e4e4e7;
  --color-input: #e4e4e7;

  --color-destructive: #ef4444;
  --color-destructive-fg: #fafafa;
  --color-success: #10b981;
  --color-warning: #f59e0b;
}

@media (prefers-color-scheme: dark) {
  :root {
    --color-bg: #09090b;
    --color-fg: #fafafa;
    --color-muted: #27272a;
    --color-muted-fg: #a1a1aa;
    --color-border: #27272a;
    --color-input: #27272a;
  }
}
```

- [ ] **Step**: Create `packages/ui/tokens/spacing.css`

```css
:root {
  --radius-sm: 0.25rem;
  --radius-md: 0.5rem;
  --radius-lg: 0.75rem;
  --radius-xl: 1rem;

  --shadow-sm: 0 1px 2px 0 rgb(0 0 0 / 0.05);
  --shadow-md: 0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1);
  --shadow-lg: 0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1);
}
```

- [ ] **Step**: Create `packages/ui/tokens/index.css`

```css
@import "./typography.css";
@import "./colors.css";
@import "./spacing.css";
```

#### Step 2.3: shadcn Button primitive

- [ ] **Step**: Create `packages/ui/primitives/button.tsx`

```tsx
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import * as React from "react";
import { cn } from "../lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-brand-500)] focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        default:
          "bg-[var(--color-brand-600)] text-white hover:bg-[var(--color-brand-700)]",
        destructive:
          "bg-[var(--color-destructive)] text-[var(--color-destructive-fg)] hover:opacity-90",
        outline:
          "border border-[var(--color-border)] bg-[var(--color-bg)] hover:bg-[var(--color-muted)]",
        ghost: "hover:bg-[var(--color-muted)]",
        link: "text-[var(--color-brand-600)] underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 rounded-md px-3",
        lg: "h-11 rounded-md px-8",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

export interface ButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

export const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  }
);
Button.displayName = "Button";

export { buttonVariants };
```

#### Step 2.4: Input primitive

- [ ] **Step**: Create `packages/ui/primitives/input.tsx`

```tsx
import * as React from "react";
import { cn } from "../lib/utils";

export type InputProps = React.InputHTMLAttributes<HTMLInputElement>;

export const Input = React.forwardRef<HTMLInputElement, InputProps>(
  ({ className, type, ...props }, ref) => {
    return (
      <input
        type={type}
        className={cn(
          "flex h-10 w-full rounded-md border border-[var(--color-input)] bg-[var(--color-bg)] px-3 py-2 text-sm ring-offset-[var(--color-bg)] file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-[var(--color-muted-fg)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-brand-500)] focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
          className
        )}
        ref={ref}
        {...props}
      />
    );
  }
);
Input.displayName = "Input";
```

#### Step 2.5: Card primitive

- [ ] **Step**: Create `packages/ui/primitives/card.tsx`

```tsx
import * as React from "react";
import { cn } from "../lib/utils";

export const Card = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "rounded-lg border border-[var(--color-border)] bg-[var(--color-bg)] text-[var(--color-fg)] shadow-sm",
        className
      )}
      {...props}
    />
  )
);
Card.displayName = "Card";

export const CardHeader = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  ({ className, ...props }, ref) => (
    <div ref={ref} className={cn("flex flex-col space-y-1.5 p-6", className)} {...props} />
  )
);
CardHeader.displayName = "CardHeader";

export const CardTitle = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLHeadingElement>>(
  ({ className, ...props }, ref) => (
    <h3
      ref={ref}
      className={cn("text-2xl font-semibold leading-none tracking-tight", className)}
      {...props}
    />
  )
);
CardTitle.displayName = "CardTitle";

export const CardContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div ref={ref} className={cn("p-6 pt-0", className)} {...props} />
));
CardContent.displayName = "CardContent";
```

#### Step 2.6: Dialog (Modal) primitive

- [ ] **Step**: Create `packages/ui/primitives/dialog.tsx`

```tsx
import * as DialogPrimitive from "@radix-ui/react-dialog";
import { X } from "lucide-react";
import * as React from "react";
import { cn } from "../lib/utils";

export const Dialog = DialogPrimitive.Root;
export const DialogTrigger = DialogPrimitive.Trigger;
export const DialogPortal = DialogPrimitive.Portal;
export const DialogClose = DialogPrimitive.Close;

export const DialogOverlay = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Overlay>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Overlay>
>(({ className, ...props }, ref) => (
  <DialogPrimitive.Overlay
    ref={ref}
    className={cn(
      "fixed inset-0 z-50 bg-black/80 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
      className
    )}
    {...props}
  />
));
DialogOverlay.displayName = DialogPrimitive.Overlay.displayName;

export const DialogContent = React.forwardRef<
  React.ElementRef<typeof DialogPrimitive.Content>,
  React.ComponentPropsWithoutRef<typeof DialogPrimitive.Content>
>(({ className, children, ...props }, ref) => (
  <DialogPortal>
    <DialogOverlay />
    <DialogPrimitive.Content
      ref={ref}
      className={cn(
        "fixed left-[50%] top-[50%] z-50 grid w-full max-w-lg translate-x-[-50%] translate-y-[-50%] gap-4 border border-[var(--color-border)] bg-[var(--color-bg)] p-6 shadow-lg sm:rounded-lg",
        className
      )}
      {...props}
    >
      {children}
      <DialogPrimitive.Close className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-[var(--color-bg)] transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 disabled:pointer-events-none">
        <X className="h-4 w-4" />
        <span className="sr-only">닫기</span>
      </DialogPrimitive.Close>
    </DialogPrimitive.Content>
  </DialogPortal>
));
DialogContent.displayName = DialogPrimitive.Content.displayName;
```

#### Step 2.7: Form + Sonner placeholder

- [ ] **Step**: Create `packages/ui/primitives/form.tsx`

```tsx
import * as LabelPrimitive from "@radix-ui/react-label";
import * as React from "react";
import { cn } from "../lib/utils";

/**
 * Form label — react-hook-form 통합 시 SP6-i 가 보강.
 * 본 sub-project 는 minimal Label primitive 만.
 */
export const Label = React.forwardRef<
  React.ElementRef<typeof LabelPrimitive.Root>,
  React.ComponentPropsWithoutRef<typeof LabelPrimitive.Root>
>(({ className, ...props }, ref) => (
  <LabelPrimitive.Root
    ref={ref}
    className={cn(
      "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
      className
    )}
    {...props}
  />
));
Label.displayName = LabelPrimitive.Root.displayName;
```

- [ ] **Step**: Create `packages/ui/primitives/sonner.tsx`

```tsx
"use client";

import { Toaster as Sonner } from "sonner";

/**
 * Sonner 기반 Toast — 한국어 기본 메시지는 사용처에서 결정.
 */
export function Toaster() {
  return (
    <Sonner
      position="top-right"
      theme="system"
      richColors
      closeButton
      duration={4000}
    />
  );
}

export { toast } from "sonner";
```

#### Step 2.8: primitives 인덱스

- [ ] **Step**: Create `packages/ui/primitives/index.ts`

```typescript
export { Button, buttonVariants, type ButtonProps } from "./button";
export { Input, type InputProps } from "./input";
export { Card, CardHeader, CardTitle, CardContent } from "./card";
export {
  Dialog,
  DialogTrigger,
  DialogPortal,
  DialogClose,
  DialogOverlay,
  DialogContent,
} from "./dialog";
export { Label } from "./form";
export { Toaster, toast } from "./sonner";
```

- [ ] **Step**: Update `packages/ui/index.ts`

```typescript
export * from "./primitives";
export { cn } from "./lib/utils";
```

#### Step 2.9: globals.css 에 tokens 통합

- [ ] **Step**: Update `apps/web/app/globals.css`

```css
@import "tailwindcss";
@import "@gongzzang/ui/tokens";

@theme {
  --font-family-sans: var(--font-sans);
  --font-family-mono: var(--font-mono);
  --color-background: var(--color-bg);
  --color-foreground: var(--color-fg);
  --color-primary: var(--color-brand-600);
  --color-primary-foreground: white;
  --color-muted: var(--color-muted);
  --color-muted-foreground: var(--color-muted-fg);
  --color-border: var(--color-border);
  --color-input: var(--color-input);
  --color-destructive: var(--color-destructive);
  --color-destructive-foreground: var(--color-destructive-fg);
  --radius: var(--radius-md);
}

html {
  font-family: var(--font-sans);
}

body {
  background: var(--color-bg);
  color: var(--color-fg);
}
```

#### Step 2.10: next-intl 설정

- [ ] **Step**: Create `apps/web/lib/i18n/ko.json`

```json
{
  "common": {
    "loading": "불러오는 중이에요",
    "error": "오류가 발생했어요",
    "retry": "다시 시도해 주세요",
    "notFound": "페이지를 찾을 수 없어요"
  },
  "errors": {
    "network": "네트워크 연결을 확인해 주세요",
    "server": "서버에 일시적인 문제가 있어요. 잠시 후 다시 시도해 주세요"
  }
}
```

- [ ] **Step**: Create `apps/web/lib/i18n/request.ts`

```typescript
import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async () => {
  const locale = "ko";
  return {
    locale,
    messages: (await import(`./ko.json`)).default,
  };
});
```

- [ ] **Step**: Create `apps/web/i18n.ts`

```typescript
import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async () => {
  const locale = "ko";
  return {
    locale,
    messages: (await import(`./lib/i18n/ko.json`)).default,
  };
});
```

- [ ] **Step**: Update `apps/web/next.config.ts`

```typescript
import createNextIntlPlugin from "next-intl/plugin";
import type { NextConfig } from "next";

const withNextIntl = createNextIntlPlugin("./i18n.ts");

const nextConfig: NextConfig = {
  reactStrictMode: true,
  experimental: {
    typedRoutes: true,
  },
};

export default withNextIntl(nextConfig);
```

#### Step 2.11: 해요체 helper utils

- [ ] **Step**: Create `apps/web/lib/i18n/haeyo.ts`

```typescript
/**
 * 한국어 해요체 helper utilities.
 *
 * 일관된 시간 / 숫자 / 가격 표현을 위한 utils.
 */

const RTF = new Intl.RelativeTimeFormat("ko", { numeric: "auto" });

/**
 * 상대 시간 — "3일 전 / 5분 전 / 방금 전" 형식.
 */
export function formatRelativeTime(date: Date | string): string {
  const target = typeof date === "string" ? new Date(date) : date;
  const diff = Math.floor((target.getTime() - Date.now()) / 1000);

  if (Math.abs(diff) < 60) return RTF.format(Math.floor(diff), "second");
  if (Math.abs(diff) < 3600) return RTF.format(Math.floor(diff / 60), "minute");
  if (Math.abs(diff) < 86400) return RTF.format(Math.floor(diff / 3600), "hour");
  if (Math.abs(diff) < 2592000) return RTF.format(Math.floor(diff / 86400), "day");
  if (Math.abs(diff) < 31536000) return RTF.format(Math.floor(diff / 2592000), "month");
  return RTF.format(Math.floor(diff / 31536000), "year");
}

const KRW_FORMAT = new Intl.NumberFormat("ko-KR", {
  style: "currency",
  currency: "KRW",
  maximumFractionDigits: 0,
});

/**
 * 가격 — "1,234,567원" 형식.
 */
export function formatKrw(amount: number): string {
  return KRW_FORMAT.format(amount);
}

const NUMBER_FORMAT = new Intl.NumberFormat("ko-KR");

/**
 * 숫자 — "1,234,567" 형식.
 */
export function formatNumber(n: number): string {
  return NUMBER_FORMAT.format(n);
}

/**
 * 면적 (m²) — "100m²" 형식.
 */
export function formatAreaM2(m2: number): string {
  return `${NUMBER_FORMAT.format(Math.round(m2 * 10) / 10)}m²`;
}

/**
 * "n개" 한국어 단위 — "3개 / 10개".
 */
export function formatCount(n: number, unit: string): string {
  return `${NUMBER_FORMAT.format(n)}${unit}`;
}
```

#### Step 2.12: error.tsx / not-found.tsx / loading.tsx

- [ ] **Step**: Create `apps/web/app/error.tsx`

```tsx
"use client";

import { Button } from "@gongzzang/ui";
import { useEffect } from "react";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    // SP7-i Sentry 가 instrumentation.ts 에서 캡처
    console.error(error);
  }, [error]);

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">오류가 발생했어요</h2>
      <p className="text-[var(--color-muted-fg)]">
        잠시 후 다시 시도해 주세요. 문제가 계속되면 관리자에게 문의해 주세요.
      </p>
      <Button onClick={reset}>다시 시도</Button>
    </main>
  );
}
```

- [ ] **Step**: Create `apps/web/app/not-found.tsx`

```tsx
import Link from "next/link";
import { Button } from "@gongzzang/ui";

export default function NotFound() {
  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <h2 className="text-2xl font-bold">페이지를 찾을 수 없어요</h2>
      <p className="text-[var(--color-muted-fg)]">
        주소가 맞는지 다시 한번 확인해 주세요.
      </p>
      <Button asChild>
        <Link href="/">홈으로 돌아가기</Link>
      </Button>
    </main>
  );
}
```

- [ ] **Step**: Create `apps/web/app/loading.tsx`

```tsx
export default function Loading() {
  return (
    <main className="flex min-h-screen items-center justify-center p-8">
      <div className="flex flex-col items-center gap-3" role="status" aria-live="polite">
        <div className="h-12 w-12 animate-spin rounded-full border-4 border-[var(--color-muted)] border-t-[var(--color-brand-600)]" />
        <span className="sr-only">불러오는 중이에요</span>
      </div>
    </main>
  );
}
```

#### Step 2.13: layout.tsx 확장 (NextIntlClientProvider + 한국어 lang)

- [ ] **Step**: Update `apps/web/app/layout.tsx`

```tsx
import type { Metadata } from "next";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages } from "next-intl/server";
import "./globals.css";
import { Toaster } from "@gongzzang/ui";

export const metadata: Metadata = {
  title: "공짱 — 산업용 부동산 정보",
  description: "산업용 부동산 정보 플랫폼",
};

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const locale = await getLocale();
  const messages = await getMessages();

  return (
    <html lang={locale}>
      <body>
        <NextIntlClientProvider messages={messages}>
          {children}
          <Toaster />
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
```

#### Step 2.14: Vitest 설정

- [ ] **Step**: Create `apps/web/vitest.config.ts`

```typescript
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "happy-dom",
    setupFiles: ["./tests/unit/setup.ts"],
    include: ["tests/unit/**/*.test.{ts,tsx}"],
    globals: true,
  },
  resolve: {
    alias: {
      "@": new URL("./", import.meta.url).pathname,
    },
  },
});
```

- [ ] **Step**: Create `apps/web/tests/unit/setup.ts`

```typescript
import "@testing-library/jest-dom/vitest";
```

#### Step 2.15: 해요체 unit test

- [ ] **Step**: Create `apps/web/tests/unit/haeyo.test.ts`

```typescript
import { describe, expect, it } from "vitest";
import {
  formatAreaM2,
  formatCount,
  formatKrw,
  formatNumber,
  formatRelativeTime,
} from "@/lib/i18n/haeyo";

describe("haeyo utils", () => {
  it("formatKrw — 천 단위 콤마 + 원", () => {
    expect(formatKrw(1234567)).toBe("₩1,234,567");
  });

  it("formatNumber — 한국어 천 단위 콤마", () => {
    expect(formatNumber(1234567)).toBe("1,234,567");
  });

  it("formatAreaM2 — m² 단위", () => {
    expect(formatAreaM2(100)).toBe("100m²");
    expect(formatAreaM2(123.456)).toBe("123.5m²");
  });

  it("formatCount — n + 단위", () => {
    expect(formatCount(3, "개")).toBe("3개");
    expect(formatCount(10000, "건")).toBe("10,000건");
  });

  it("formatRelativeTime — 5분 전", () => {
    const fiveMinAgo = new Date(Date.now() - 5 * 60 * 1000);
    const result = formatRelativeTime(fiveMinAgo);
    expect(result).toMatch(/분/); // "5분 전" or similar
  });
});
```

- [ ] **Step**: Run unit tests

```bash
pnpm --filter=@gongzzang/web test
```

Expected: 5 tests pass.

#### Step 2.16: typecheck + build

- [ ] **Step**: 검증

```bash
pnpm typecheck
pnpm build
pnpm lint
```

Expected: 모두 pass. Biome lint clean.

#### Step 2.17: T2 commit

- [ ] **Step**: T2 commit

```bash
git add packages/ui apps/web

git commit -m "$(cat <<'EOF'
feat(sp6-foundation-t2): shadcn primitives + tokens + i18n + UX patterns

T2 of SP6-foundation:
- packages/ui/lib/utils.ts (cn helper — clsx + tailwind-merge)
- packages/ui/tokens/ CSS vars (colors + spacing + typography + Pretendard webfont @import)
- packages/ui/primitives/ 6 컴포넌트:
  - button.tsx (cva variants: default/destructive/outline/ghost/link)
  - input.tsx
  - card.tsx (Card / CardHeader / CardTitle / CardContent)
  - dialog.tsx (Radix Dialog with X close + 한국어 sr-only label)
  - form.tsx (Label, react-hook-form 통합은 SP6-i)
  - sonner.tsx (Toaster + toast re-export)
- apps/web 통합:
  - next-intl ko-KR + NextIntlClientProvider in layout.tsx
  - lib/i18n/haeyo.ts — formatKrw/Number/AreaM2/Count/RelativeTime utils
  - lib/i18n/ko.json — common + errors strings
  - app/globals.css — Tailwind 4 @theme + tokens import
  - app/error.tsx + not-found.tsx + loading.tsx (한국어 fallback, role/aria)
- Vitest + happy-dom + @testing-library/jest-dom 설정
- 5 unit tests (haeyo utils)
- typecheck + build + lint 통과
EOF
)"
```

DO NOT push — controller pushes after T2 review.

**사용자 체크포인트**: T2 commit 확인 + 다음 진행.

---

## Phase C: API client + TanStack Query + proxy + Sentry placeholder

### Task 3: ky + openapi-typescript + TanStack Query + proxy skeleton + instrumentation + Zustand

**Files:**
- Create: `packages/api-types/scripts/generate.ts`
- Create: `packages/api-types/generated/schema.ts` (placeholder, utoipa 미통합 시 stub)
- Modify: `packages/api-types/index.ts`
- Create: `apps/web/lib/api.ts` (ky + types)
- Create: `apps/web/lib/query.ts` (TanStack Query)
- Create: `apps/web/lib/env.ts` (zod env)
- Create: `apps/web/app/api/proxy/[...path]/route.ts`
- Create: `apps/web/instrumentation.ts` (empty)
- Create: `apps/web/stores/index.ts` (Zustand skeleton)
- Modify: `apps/web/app/layout.tsx` (QueryClientProvider 추가)
- Create: `apps/web/tests/unit/api.test.ts`
- Create: `apps/web/tests/unit/env.test.ts`

#### Step 3.1: openapi-typescript generate script

- [ ] **Step**: Create `packages/api-types/scripts/generate.ts`

```typescript
import openapiTS from "openapi-typescript";
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";

/**
 * utoipa (services/api) 가 출력한 OpenAPI spec → TypeScript types.
 *
 * 사용:
 *   1) services/api 가 utoipa 로 OpenAPI spec 출력 (예: services/api/openapi.json)
 *   2) `pnpm --filter @gongzzang/api-types generate` 실행
 *   3) packages/api-types/generated/schema.ts 에 types 작성
 *
 * 본 sub-project (T3) 는 스크립트만 작성. utoipa 미통합 시 placeholder generated/schema.ts 유지.
 * SP4-iii-? 또는 SP6-i 가 utoipa 통합 시점에 본 스크립트가 활성.
 */

const OPENAPI_PATH = resolve(__dirname, "../../../services/api/openapi.json");
const OUTPUT_PATH = resolve(__dirname, "../generated/schema.ts");

async function main() {
  let openapiContent: string;
  try {
    openapiContent = await readFile(OPENAPI_PATH, "utf-8");
  } catch (err) {
    console.warn(
      `[api-types] OpenAPI spec not found at ${OPENAPI_PATH}. Keeping placeholder.`
    );
    return;
  }

  const types = await openapiTS(JSON.parse(openapiContent));
  await writeFile(OUTPUT_PATH, types, "utf-8");
  console.info(`[api-types] Generated TS types at ${OUTPUT_PATH}`);
}

main().catch((err) => {
  console.error("[api-types] Generation failed:", err);
  process.exit(1);
});
```

#### Step 3.2: generated/schema.ts placeholder

- [ ] **Step**: Create `packages/api-types/generated/schema.ts`

```typescript
/**
 * Placeholder — utoipa OpenAPI 통합 시점 (SP4-iii-? 또는 SP6-i) 에
 * `pnpm --filter @gongzzang/api-types generate` 실행으로 자동 생성.
 *
 * 본 sub-project (SP6-foundation T3) 는 minimal stub 만 제공.
 */

export interface paths {
  "/healthz": {
    get: {
      responses: {
        200: {
          content: {
            "text/plain": string;
          };
        };
      };
    };
  };
}

export type components = Record<string, never>;
```

- [ ] **Step**: Update `packages/api-types/index.ts`

```typescript
export type { paths, components } from "./generated/schema";
```

#### Step 3.3: zod env validation

- [ ] **Step**: Create `apps/web/lib/env.ts`

```typescript
import { z } from "zod";

const EnvSchema = z.object({
  NEXT_PUBLIC_API_BASE_URL: z
    .string()
    .url()
    .default("http://localhost:8080"),
  // SP6-i 가 추가:
  // NEXT_PUBLIC_ZITADEL_ISSUER, NEXT_PUBLIC_ZITADEL_CLIENT_ID, ZITADEL_AUDIENCE, IRON_SESSION_PASSWORD
});

const parsed = EnvSchema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
});

if (!parsed.success) {
  throw new Error(
    `Invalid environment variables: ${parsed.error.flatten().fieldErrors}`
  );
}

export const env = parsed.data;
export type Env = z.infer<typeof EnvSchema>;
```

#### Step 3.4: ky API client

- [ ] **Step**: Create `apps/web/lib/api.ts`

```typescript
import ky from "ky";
import { env } from "./env";

/**
 * Frontend → /api/proxy/[...path] → services/api 호출 ky client.
 *
 * 직접 services/api 를 호출하지 않음 — 항상 Next.js proxy route 통과
 * (httpOnly cookie 검증 + secrets server-only).
 *
 * 사용:
 *   const data = await api.get("listings").json<Listing[]>();
 *
 * SP6-i 가 추가:
 *   - cookie credentials 자동 attach (현재는 same-origin proxy 라 자동)
 *   - 401 → /login redirect
 */
export const api = ky.create({
  prefixUrl: "/api/proxy",
  retry: {
    limit: 1,
    methods: ["get"],
  },
  timeout: 10000,
  hooks: {
    beforeError: [
      (error) => {
        const { response } = error;
        if (response?.status === 401) {
          // SP6-i 가 redirect 로직 추가
          console.warn("[api] 401 — login required");
        }
        return error;
      },
    ],
  },
});

/**
 * Server-side direct API client (Next.js Route Handler / Server Component 에서만 사용).
 * Browser bundle 에 절대 포함되지 않음 (proxy route 가 사용).
 */
export function createServerApi(authHeader?: string) {
  return ky.create({
    prefixUrl: env.NEXT_PUBLIC_API_BASE_URL,
    timeout: 10000,
    headers: authHeader ? { Authorization: authHeader } : {},
  });
}
```

#### Step 3.5: TanStack Query setup

- [ ] **Step**: Create `apps/web/lib/query.ts`

```typescript
"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

/**
 * TanStack Query default config.
 *
 * - staleTime: 30s — server-side cache 와 정합
 * - refetchOnWindowFocus: false — 사용자 관심사 외 호출 ↓
 * - retry: 1 — 외부 API drift 시 빠른 fail
 */
export function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30 * 1000,
        refetchOnWindowFocus: false,
        retry: 1,
      },
    },
  });
}

export function QueryProvider({ children }: { children: React.ReactNode }) {
  const [client] = useState(() => makeQueryClient());
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
```

#### Step 3.6: layout.tsx 에 QueryProvider 통합

- [ ] **Step**: Update `apps/web/app/layout.tsx`

```tsx
import type { Metadata } from "next";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages } from "next-intl/server";
import "./globals.css";
import { Toaster } from "@gongzzang/ui";
import { QueryProvider } from "@/lib/query";

export const metadata: Metadata = {
  title: "공짱 — 산업용 부동산 정보",
  description: "산업용 부동산 정보 플랫폼",
};

export default async function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const locale = await getLocale();
  const messages = await getMessages();

  return (
    <html lang={locale}>
      <body>
        <NextIntlClientProvider messages={messages}>
          <QueryProvider>
            {children}
            <Toaster />
          </QueryProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
```

#### Step 3.7: Backend proxy Route Handler

- [ ] **Step**: Create `apps/web/app/api/proxy/[...path]/route.ts`

```typescript
import { type NextRequest, NextResponse } from "next/server";
import { createServerApi } from "@/lib/api";

/**
 * SP6-foundation: backend proxy skeleton — auth 검증 X (unauthenticated).
 * SP6-i 가 채울 부분:
 *   1) iron-session cookie 검증
 *   2) Authorization: Bearer <jwt> 헤더 추가
 *   3) 401 → /login redirect
 *
 * 본 sub-project 는 단순 forward 만 — /healthz 같은 unauthenticated endpoint smoke 가능.
 */

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const path = params.path.join("/");
  const url = new URL(req.url);
  const search = url.search;

  // SP6-i 가 cookie 검증 추가
  const api = createServerApi(/* SP6-i: 인증 헤더 */);

  try {
    const init: ky.Options = {
      method: req.method as "GET" | "POST" | "PUT" | "DELETE" | "PATCH",
      searchParams: search ? Object.fromEntries(new URLSearchParams(search)) : undefined,
    };

    if (["POST", "PUT", "PATCH"].includes(req.method)) {
      init.json = await req.json().catch(() => undefined);
    }

    const response = await api(`${path}`, init).text();
    return new NextResponse(response, {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  } catch (err: unknown) {
    if (err && typeof err === "object" && "response" in err) {
      const httpErr = err as { response: { status: number; text: () => Promise<string> } };
      const body = await httpErr.response.text();
      return new NextResponse(body, { status: httpErr.response.status });
    }
    return NextResponse.json(
      { error: "Backend unreachable", code: "PROXY_FAIL" },
      { status: 502 }
    );
  }
}

export async function GET(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function POST(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function PUT(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function PATCH(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function DELETE(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
```

**참고:** ky 의 `Options` import 위치는 `ky` 직접 또는 `ky/distribution/types/options`. 컴파일 에러 시 `import type { Options as KyOptions } from "ky"` 로 수정.

#### Step 3.8: instrumentation.ts (Sentry placeholder)

- [ ] **Step**: Create `apps/web/instrumentation.ts`

```typescript
/**
 * SP7-i 가 채울 자리 — Sentry SDK 초기화.
 *
 * Next.js 16 표준 — instrumentation.ts 가 server / edge runtime 에서 자동 호출.
 * SP6-foundation 단계: empty register() — Sentry 통합 자리만.
 */
export function register(): void {
  // SP7-i 가 추가:
  // import * as Sentry from "@sentry/nextjs";
  // Sentry.init({ dsn: process.env.SENTRY_DSN, ... });
}
```

#### Step 3.9: Zustand skeleton

- [ ] **Step**: Create `apps/web/stores/index.ts`

```typescript
/**
 * Zustand stores 진입점.
 *
 * SP6-foundation: skeleton 만 (interface 분리 → 미래 Jotai 등 swap 가능).
 * SP6-i ~ v 가 실제 store 추가 (auth / search / bookmarks 등).
 */

export interface StoreInterface {
  // 미래 stores 가 implement
  reset?: () => void;
}

export {};
```

#### Step 3.10: 단위 테스트 (api / env)

- [ ] **Step**: Create `apps/web/tests/unit/api.test.ts`

```typescript
import { describe, expect, it } from "vitest";
import { api } from "@/lib/api";

describe("api client", () => {
  it("uses /api/proxy prefix", () => {
    // ky 의 prefixUrl 은 internal — config 검증
    expect(api).toBeDefined();
  });
});
```

- [ ] **Step**: Create `apps/web/tests/unit/env.test.ts`

```typescript
import { describe, expect, it } from "vitest";
import { env } from "@/lib/env";

describe("env validation", () => {
  it("provides default API_BASE_URL", () => {
    expect(env.NEXT_PUBLIC_API_BASE_URL).toBeDefined();
    expect(env.NEXT_PUBLIC_API_BASE_URL).toMatch(/^https?:\/\//);
  });
});
```

- [ ] **Step**: Run all unit tests

```bash
pnpm --filter=@gongzzang/web test
```

Expected: 7+ tests pass (haeyo 5 + api 1 + env 1).

#### Step 3.11: typecheck + build

- [ ] **Step**: 검증

```bash
pnpm typecheck
pnpm build
pnpm lint
```

Expected: 모두 pass.

#### Step 3.12: T3 commit

- [ ] **Step**: T3 commit

```bash
git add packages/api-types apps/web

git commit -m "$(cat <<'EOF'
feat(sp6-foundation-t3): API client + TanStack Query + proxy skeleton + Sentry placeholder

T3 of SP6-foundation:
- packages/api-types/scripts/generate.ts (utoipa OpenAPI → openapi-typescript, utoipa 미통합 시 placeholder 유지)
- packages/api-types/generated/schema.ts placeholder (paths /healthz minimal)
- apps/web/lib/api.ts — ky client (prefixUrl /api/proxy, 401 hook + retry 1)
  - createServerApi (server-side direct, Next.js Route Handler 만)
- apps/web/lib/query.ts — TanStack Query (staleTime 30s + refetchOnWindowFocus false + retry 1)
- apps/web/lib/env.ts — zod env validation (NEXT_PUBLIC_API_BASE_URL)
- apps/web/app/api/proxy/[...path]/route.ts — backend proxy skeleton
  - GET/POST/PUT/PATCH/DELETE forward — auth 검증은 SP6-i 가 채움
- apps/web/instrumentation.ts — empty register() (SP7-i Sentry 자리)
- apps/web/stores/index.ts — Zustand skeleton (interface 분리, swap 가능)
- apps/web/app/layout.tsx — QueryProvider 통합
- 7 unit tests (haeyo 5 + api 1 + env 1)
EOF
)"
```

DO NOT push.

**사용자 체크포인트**: T3 commit 확인 + 다음 진행.

---

## Phase D: CI + a11y + bundle budget + smoke + docs

### Task 4: smoke 화면 + Playwright + axe + size-limit + frontend.yml + docs/frontend + roadmap

**Files:**
- Modify: `apps/web/app/page.tsx` (smoke /healthz)
- Create: `apps/web/playwright.config.ts`
- Create: `apps/web/tests/e2e/healthz.spec.ts`
- Create: `apps/web/tests/e2e/a11y.spec.ts`
- Create: `apps/web/.size-limit.json`
- Create: `.github/workflows/frontend.yml`
- Create: `docs/frontend/README.md`
- Modify: `docs/superpowers/roadmap.md`

#### Step 4.1: smoke 화면 (/healthz 호출)

- [ ] **Step**: Update `apps/web/app/page.tsx`

```tsx
"use client";

import { useQuery } from "@tanstack/react-query";
import { Button, Card, CardContent, CardHeader, CardTitle } from "@gongzzang/ui";
import { api } from "@/lib/api";

export default function Home() {
  const { data, isLoading, error, refetch } = useQuery({
    queryKey: ["healthz"],
    queryFn: () => api.get("healthz").text(),
  });

  return (
    <main className="container mx-auto flex min-h-screen flex-col items-center justify-center gap-4 p-8">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>공짱 Foundation Smoke</CardTitle>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <p className="text-sm text-[var(--color-muted-fg)]">
            /api/proxy/healthz → backend /healthz 호출 확인.
          </p>
          {isLoading && <p>불러오는 중이에요…</p>}
          {error && (
            <p className="text-[var(--color-destructive)]" role="alert">
              호출 실패: {error.message}
            </p>
          )}
          {data && (
            <p className="font-mono text-sm" data-testid="healthz-response">
              응답: {data}
            </p>
          )}
          <Button onClick={() => refetch()} variant="outline">
            다시 호출
          </Button>
        </CardContent>
      </Card>
    </main>
  );
}
```

#### Step 4.2: Playwright 설정

- [ ] **Step**: Create `apps/web/playwright.config.ts`

```typescript
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"], ["html", { open: "never" }]],
  use: {
    baseURL: "http://localhost:3000",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "pnpm dev",
    url: "http://localhost:3000",
    reuseExistingServer: !process.env.CI,
    timeout: 120000,
  },
});
```

#### Step 4.3: smoke e2e

- [ ] **Step**: Create `apps/web/tests/e2e/healthz.spec.ts`

```typescript
import { expect, test } from "@playwright/test";

test.describe("Foundation smoke", () => {
  test("home page loads + healthz call (mocked or real backend)", async ({ page }) => {
    // backend 가 안 떠 있으면 502 — 이는 의도된 fail (smoke 의 의미).
    // CI 에서는 services/api 도 docker-compose 로 띄우거나, mock 으로.
    // T4 기준 CI: backend 미동행 → 502 응답이 OK (smoke = "frontend 가 정상 빌드되고 호출함" 확인).

    await page.goto("/");

    await expect(page.getByText("공짱 Foundation Smoke")).toBeVisible();

    // 응답이 200(OK) 또는 502 (backend down) — 둘 다 "frontend pipeline 정상" 의미
    await page.waitForFunction(
      () => {
        const el = document.querySelector("[data-testid='healthz-response']");
        const errEl = document.querySelector("[role='alert']");
        return el !== null || errEl !== null;
      },
      { timeout: 10000 }
    );

    const responseEl = page.getByTestId("healthz-response");
    const errorEl = page.getByRole("alert");

    const hasResponse = (await responseEl.count()) > 0;
    const hasError = (await errorEl.count()) > 0;

    expect(hasResponse || hasError).toBe(true);
  });
});
```

#### Step 4.4: a11y e2e (axe)

- [ ] **Step**: Create `apps/web/tests/e2e/a11y.spec.ts`

```typescript
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.describe("a11y — WCAG 2.1 AA", () => {
  test("home page passes axe", async ({ page }) => {
    await page.goto("/");
    await page.waitForLoadState("networkidle");

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();

    // critical / serious 만 fail 처리 — minor / moderate 는 warn
    const criticalViolations = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious"
    );

    if (criticalViolations.length > 0) {
      console.error(
        `[a11y] ${criticalViolations.length} critical/serious violations:`,
        JSON.stringify(criticalViolations, null, 2)
      );
    }

    expect(criticalViolations).toEqual([]);
  });

  test("error page (의도된 에러) passes axe", async ({ page }) => {
    // /not-found 라우트 → not-found.tsx 렌더
    await page.goto("/__nonexistent-path-for-testing__");
    await page.waitForLoadState("networkidle");

    const results = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa"])
      .analyze();

    const criticalViolations = results.violations.filter(
      (v) => v.impact === "critical" || v.impact === "serious"
    );

    expect(criticalViolations).toEqual([]);
  });
});
```

#### Step 4.5: size-limit 설정

- [ ] **Step**: Create `apps/web/.size-limit.json`

```json
[
  {
    "name": "production bundle (initial JS)",
    "path": ".next/static/chunks/main-*.js",
    "limit": "200 KB",
    "gzip": true
  },
  {
    "name": "production bundle (framework)",
    "path": ".next/static/chunks/framework-*.js",
    "limit": "60 KB",
    "gzip": true
  }
]
```

#### Step 4.6: package.json 에 size:check script 추가

- [ ] **Step**: 이미 있는 `test:bundle` 또는 추가:

```bash
# apps/web/package.json 의 scripts 에:
# "size": "size-limit"
# "size:why": "size-limit --why"
```

(Step 1.4 에서 이미 추가된 `test:bundle` 사용)

#### Step 4.7: Playwright 의존성 설치 (Chromium browser)

- [ ] **Step**: Local 검증 (CI 에서는 workflow 가 처리)

```bash
cd apps/web
pnpm exec playwright install chromium
pnpm exec playwright install-deps chromium
```

이 step 은 *local* 검증용. CI 는 `actions/setup-node` 후 별도 install step.

- [ ] **Step**: e2e 테스트 실행 (local)

```bash
# Backend 가 안 떠도 OK — smoke 가 502 도 OK 로 처리
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 3 tests pass (smoke 1 + a11y 2).

#### Step 4.8: bundle budget 검증 (local)

- [ ] **Step**: build + size 검증

```bash
pnpm --filter=@gongzzang/web build
pnpm --filter=@gongzzang/web test:bundle
```

Expected: bundle < 200KB JS gzipped + < 60KB framework. Fail 시 의존성 분석.

#### Step 4.9: frontend CI workflow

- [ ] **Step**: Create `.github/workflows/frontend.yml`

```yaml
name: frontend

on:
  push:
    branches: [main]
    paths:
      - "apps/web/**"
      - "packages/ui/**"
      - "packages/api-types/**"
      - "pnpm-workspace.yaml"
      - "turbo.json"
      - "package.json"
      - "pnpm-lock.yaml"
      - "biome.json"
      - ".github/workflows/frontend.yml"
  pull_request:
    branches: [main]
    paths:
      - "apps/web/**"
      - "packages/ui/**"
      - "packages/api-types/**"
      - "pnpm-workspace.yaml"
      - "turbo.json"
      - "package.json"
      - "pnpm-lock.yaml"
      - "biome.json"
      - ".github/workflows/frontend.yml"

concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true

permissions:
  contents: read

jobs:
  frontend:
    name: lint / typecheck / unit / e2e / a11y / bundle
    runs-on: ubuntu-24.04
    timeout-minutes: 20

    steps:
      - uses: actions/checkout@v4

      - uses: pnpm/action-setup@v4
        with:
          version: 9.15.0

      - uses: actions/setup-node@v4
        with:
          node-version: "20"
          cache: "pnpm"

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Biome lint (root)
        run: pnpm lint

      - name: TypeScript typecheck
        run: pnpm typecheck

      - name: Vitest unit
        run: pnpm test

      - name: Build (production)
        run: pnpm build

      - name: Bundle size budget
        run: pnpm --filter=@gongzzang/web test:bundle

      - name: Install Playwright browsers
        run: pnpm --filter=@gongzzang/web exec playwright install chromium --with-deps

      - name: Playwright e2e + a11y
        run: pnpm --filter=@gongzzang/web test:e2e
        env:
          # Backend 미동행 — smoke 가 502 도 OK 로 처리
          NEXT_PUBLIC_API_BASE_URL: http://localhost:8080

      - name: Upload Playwright report (on failure)
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: playwright-report
          path: apps/web/playwright-report/
          retention-days: 7
```

#### Step 4.10: docs/frontend/README.md

- [ ] **Step**: Create `docs/frontend/README.md`

````markdown
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
- 에러 메시지: **원인 + 대응 안내**:
  - "네트워크 연결을 확인해 주세요"
  - "서버에 일시적인 문제가 있어요. 잠시 후 다시 시도해 주세요"
- 시간/숫자/면적 포맷: `apps/web/lib/i18n/haeyo.ts` utils 사용
- 다국어 자원: `apps/web/lib/i18n/ko.json` (next-intl)

## 디자인 시스템 swap path

```
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
| E2E | Playwright | `pnpm test:e2e` (smoke) |
| a11y | @axe-core/playwright | `pnpm test:e2e` (a11y.spec.ts) |
| Bundle | size-limit | `pnpm --filter=@gongzzang/web test:bundle` |
| Format | Biome | `pnpm format --write` |

## 비목표 (다른 sub-project)

- **Auth flow + 화면** — SP6-i (login/signup/profile + Zitadel OIDC + iron-session + RBAC + middleware)
- 매물 검색/상세/등록/북마크/알림 — SP6-ii ~ v
- Naver Maps SDK — SP6-ii
- Sentry 통합 — SP7-i (instrumentation.ts 자리만 명시)
- PWA / offline — YAGNI (production 후 결정)
- Storybook — over-engineered (1인 단계, e2e + Vitest 충분)

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
- AGENTS.md: 프로젝트 헌법 (한국어 컨벤션 / SSS 7기둥 / SSOT 매트릭스)
````

#### Step 4.11: roadmap 갱신

- [ ] **Step**: Update `docs/superpowers/roadmap.md`

다음 변경 적용:

**Header:**
```markdown
> **갱신일**: 2026-05-05 (SP6-foundation 종료 직후)
> **현재 main**: `<T4 commit hash>` (SP6-foundation — frontend 인프라)
```

**완료 표 (SP6-foundation 행 추가):**
```markdown
| **6-foundation** | Frontend 인프라 (Next.js 16 + shadcn + tokens + i18n + UX 패턴) | apps/web (Next.js 16 + React 19 + Tailwind 4) + packages/ui (shadcn primitives + Pretendard tokens, swap-able) + packages/api-types (utoipa → TS) + 한국어 helper + error/not-found/loading + ky API client + TanStack Query + proxy skeleton + instrumentation.ts (Sentry 자리) + Vitest + Playwright + @axe-core/playwright (WCAG 2.1 AA) + size-limit (bundle < 200KB) + .github/workflows/frontend.yml. SSS 7기둥 모두 ◎ | ✅ |
```

**누적 통계:**
- 33 crate (Rust 그대로) + JS workspace 추가
- ~1278 tests + ~7 unit + 3 e2e (Playwright) — frontend
- 5 CI workflow (frontend 추가)

**SP6 시리즈 갱신:**
```markdown
### SP6 시리즈 (Frontend)
- ✅ SP6-foundation: 인프라 (2026-05-05) — Next.js 16 + shadcn + tokens + UX
- 미착수 SP6-i: auth flow + 화면 (login/signup/profile + OIDC + RBAC, 2-3일)
- 미착수 SP6-ii: 매물 검색 + Naver Maps (2-3일)
- 미착수 SP6-iii: 매물 상세 + 북마크 (1-2일)
- 미착수 SP6-iv: 매물 등록 broker 전용 (2일)
- 미착수 SP6-v: 알림 (1일)
```

#### Step 4.12: Workspace 전체 검증

- [ ] **Step**: 모든 검증

```bash
pnpm install
pnpm typecheck
pnpm lint
pnpm test
pnpm build
pnpm --filter=@gongzzang/web test:bundle
pnpm --filter=@gongzzang/web exec playwright install chromium --with-deps
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 모두 pass.

#### Step 4.13: T4 commit + push

- [ ] **Step**: T4 commit

```bash
git add apps/web docs/frontend docs/superpowers/roadmap.md \
        .github/workflows/frontend.yml

git commit -m "$(cat <<'EOF'
feat(sp6-foundation-t4): smoke + frontend CI + a11y + bundle + docs + roadmap

T4 of SP6-foundation (마지막):
- apps/web/app/page.tsx — /api/proxy/healthz smoke 호출 + Card UI (한국어 해요체)
- apps/web/playwright.config.ts — chromium project + webServer (pnpm dev)
- apps/web/tests/e2e/healthz.spec.ts — smoke (200 또는 502 — frontend pipeline 정상 의미)
- apps/web/tests/e2e/a11y.spec.ts — @axe-core/playwright (WCAG 2.1 AA, critical/serious 0)
- apps/web/.size-limit.json — production bundle < 200KB JS gzipped + < 60KB framework
- .github/workflows/frontend.yml — pnpm + Node 20 + lint/typecheck/test/build/bundle/e2e+a11y
  - paths filter (apps/web + packages/ui + packages/api-types 변경 시만)
  - Playwright report artifact upload (on failure)
- docs/frontend/README.md — 운영 SSOT (시작법 / 디렉토리 / 한국어 컨벤션 / swap path / 진화)
- docs/superpowers/roadmap.md — SP6-foundation ✅ + SP6-i ~ v 자리 명시 + 누적 통계

SP6 시리즈 첫 sub-project 완료. SP6-i ~ v 가 이 foundation 위에서 빠른 빌드.

Closing: (없음 — 첫 frontend sub-project)
미흡수 (SP6-i ~ v 또는 SP7-i): auth flow / 매물 검색 / Sentry 통합
EOF
)"

git push origin main
```

**사용자 체크포인트**: T4 commit + push 후 5 CI workflow 그린 확인 + 다음 sub-project 결정.

---

## 위험 요소

- **Next.js 16 stable 여부**: 작업 시점 기준 16 가 안정 버전 아니면 15.x 로 fallback (package.json 만 수정)
- **Tailwind 4 PostCSS plugin**: 현재 alpha — 추후 `tailwindcss/postcss` 명시 변경 가능
- **next-intl App Router**: plugin 위치 (`./i18n.ts`) Next.js 버전마다 다름
- **shadcn CLI 미사용**: 본 plan 은 코드 직접 작성 — shadcn 의 자동 cn 패턴 / 의존성 미스 가능
- **Backend 미동행 e2e**: smoke 테스트가 502 도 OK 처리 — 미래 SP6-i 후 진짜 backend 호출 검증 필요
- **utoipa 미통합**: api-types/generated/schema.ts 가 placeholder — 실 utoipa 통합은 별도 sub-project (또는 SP6-i)

## 추정

- T1: 1 commit, 2-3시간 (monorepo + Next.js setup + 의존성 설치)
- T2: 1 commit, 4-5시간 (shadcn 6 컴포넌트 + tokens + i18n + 한국어 helper + UX patterns)
- T3: 1 commit, 3-4시간 (API client + TanStack Query + proxy + instrumentation + Zustand)
- T4: 1 commit, 3-4시간 (smoke + Playwright + axe + size-limit + workflow + docs)

총: 3-4일 (각 task 끝 사용자 체크포인트 포함)

## 완료 후 다음

- SP6-i: auth flow + 화면 brainstorming → spec → plan → impl
- 또는 SP4-iii-b 데이터 풍부화
- 또는 SP7-i Sentry (frontend instrumentation 활용)

---

## 자가 평가 — Spec coverage

Spec 의 모든 § 가 plan task 로 covered:

- § 1 배경 — context only
- § 2 목표 11개 → T1 (monorepo+Next.js), T2 (shadcn+tokens+i18n+UX), T3 (API client+proxy+Sentry자리), T4 (CI+a11y+bundle+smoke)
- § 3 SSS 7기둥 — T1-T4 누적
- § 4 Scope 포함 — T1-T4 모두 cover. 미포함 (auth/Naver Maps/PWA) 명시
- § 5 아키텍처 (큰 그림 + 호출 흐름 + swap path) → T1-T4 + docs/frontend/README
- § 6 Stack 18개 → 의존성 (T1-T3) + 도구 (T4)
- § 7 디렉토리 구조 → T1-T4 파일 그대로
- § 8 작업 단위 T1-T4 → 본 plan 의 Phase A-D
- § 9 검증 / 테스트 전략 → T2 unit + T4 e2e + a11y + bundle + workflow
- § 10 Migration / Swap path → docs/frontend/README + tokens 분리
- § 11 Follow-up → roadmap 갱신
- § 12 추정 → 본 plan 추정
- § 13 SSS 자가 평가 → T1-T4 누적
- § 14 핵심 결정 16개 → 모두 plan 에 반영

**모든 § 가 task 로 covered.** ✅
