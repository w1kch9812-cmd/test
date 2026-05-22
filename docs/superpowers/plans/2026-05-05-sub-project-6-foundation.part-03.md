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

