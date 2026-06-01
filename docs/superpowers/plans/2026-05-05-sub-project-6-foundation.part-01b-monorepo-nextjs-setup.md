# SP6 Foundation - Part 01B: Monorepo And Next.js Setup

Parent index: [SP6 Foundation Part 01](./2026-05-05-sub-project-6-foundation.part-01.md).

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
