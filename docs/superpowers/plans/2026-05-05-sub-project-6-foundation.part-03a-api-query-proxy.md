# SP6 Foundation - Part 03A: API Client, Query, Proxy, Instrumentation, And Store

Parent index: [SP6 Foundation Part 03](./2026-05-05-sub-project-6-foundation.part-03.md).
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
