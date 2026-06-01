# SP6 Foundation - Part 02A: UI Primitives And Design Tokens

Parent index: [SP6 Foundation Part 02](./2026-05-05-sub-project-6-foundation.part-02.md).
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
