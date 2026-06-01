# SP6 Foundation - Part 03B: CI, A11y, Bundle Budget, And Smoke Tests

Parent index: [SP6 Foundation Part 03](./2026-05-05-sub-project-6-foundation.part-03.md).

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
