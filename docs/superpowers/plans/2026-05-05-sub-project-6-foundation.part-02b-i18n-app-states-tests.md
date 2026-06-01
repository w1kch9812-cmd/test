# SP6 Foundation - Part 02B: I18n, App States, Tests, And Verification

Parent index: [SP6 Foundation Part 02](./2026-05-05-sub-project-6-foundation.part-02.md).

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
