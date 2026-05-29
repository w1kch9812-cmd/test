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

