# TypeScript 컨벤션

## 1. 도구

- **TypeScript**: 5.7+ strict (`tsconfig.base.json`)
- **포맷·lint·import sort**: Biome v2.4 (단독, `biome.json`)
- **테스트**: Vitest (sub-project 6+)
- **공급망**: Snyk + socket.dev + `pnpm audit`

## 2. tsconfig 핵심 (`tsconfig.base.json`)

- `strict: true`
- `noUncheckedIndexedAccess: true`
- `exactOptionalPropertyTypes: true`
- `verbatimModuleSyntax: true`
- `noImplicitOverride: true`
- `isolatedModules: true`

## 3. Biome 핵심 (`biome.json`)

- 포맷: 2 space, 100 width, `lineEnding: lf`
- import sort: `assist.actions.source.organizeImports: on`
- `correctness.noUnusedImports: error`
- `style.useConst: error`, `useImportType: error`
- `suspicious.noExplicitAny: error`, `noConsole: warn`

## 4. Next.js 패턴 (옵션 A)

### Server Component 기본

```tsx
// apps/platform-web/app/listings/page.tsx
import { rustApi } from "@gongzzang/api-client";

export default async function ListingsPage({ searchParams }: Props) {
  const params = await searchParams;
  const data = await rustApi.GET("/v1/listings", { params: { query: params } });
  return <ListingsList data={data.data} />;
}
```

### Client Component는 명시적

```tsx
"use client";
import { useState } from "react";
// ...
```

### Server Action = 얇은 프록시

```tsx
"use server";
import { rustApi } from "@gongzzang/api-client";
import { getSession } from "@/lib/session";

export async function bookmarkListing(listingId: string) {
  const session = await getSession();
  if (!session) throw new Error("로그인이 필요해요");

  return rustApi.POST("/v1/bookmarks", {
    body: { listingId },
    headers: { Authorization: `Bearer ${session.token}` },
  });
}
```

→ 비즈니스 로직 0줄. 검증·계산·저장은 Rust.

## 5. import 패턴

```tsx
// 1. 외부
import { useState } from "react";
import { z } from "zod";

// 2. 절대 경로 (@/)
import { Button } from "@/components/ui/button";

// 3. 워크스페이스
import { rustApi } from "@gongzzang/api-client";
import type { Listing } from "@gongzzang/api-client";

// 4. 상대 (같은 폴더만)
import { ListingCard } from "./listing-card";
```

## 6. 컴포넌트 구조

```tsx
// 1. import
// 2. type/interface
// 3. constants
// 4. component
// 5. helper (같은 파일 안에서만 쓰는 것)
```

## 7. 폼

`react-hook-form` + `zod`:

```tsx
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";

const schema = z.object({
  pnu: z.string().length(19),
  priceKrw: z.number().positive(),
});

const { register, handleSubmit } = useForm({ resolver: zodResolver(schema) });
```

## 8. 상태 관리

- 서버 상태: TanStack Query (Phase 1) 또는 Server Component fetch
- 클라이언트 상태: Zustand (필요 시만)
- 폼 상태: react-hook-form

전역 상태는 *마지막 수단*. 우선 props drilling → useContext → Zustand 순서.

## 9. 금지 패턴

- ❌ `any` 타입 (`noExplicitAny: error`)
- ❌ `console.log` (대신 `tracing` 미들웨어 또는 `console.warn/error`)
- ❌ Server Action 안에 비즈니스 로직 (Rust로 위임)
- ❌ 백엔드 응답 타입 수동 작성 (`packages/api-client/types.ts`는 OpenAPI 자동 생성만)
- ❌ TODO/HACK/XXX 코멘트
- ❌ `// @ts-ignore`, `// @ts-expect-error` (없이 해결)

## 10. 자동 강제

- pre-commit: `pnpm biome check --write` (lefthook)
- pre-push: `pnpm turbo run typecheck`
- CI: `pnpm biome check . && pnpm turbo run typecheck && pnpm turbo run test`
