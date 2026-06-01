# Sub-project 6-i Auth - Part 06A: Login, Profile, E2E, and A11y

Parent index: [Sub-project 6-i Auth - Part 06](./2026-05-05-sub-project-6-i-auth.part-06.md).
## Task 7: /login + /forbidden + /(authenticated)/profile 화면 + e2e + a11y

**Files:**
- Create: `apps/web/app/(public)/login/page.tsx`
- Create: `apps/web/app/(public)/forbidden/page.tsx`
- Create: `apps/web/app/(authenticated)/layout.tsx`
- Create: `apps/web/app/(authenticated)/profile/page.tsx`
- Modify: `apps/web/playwright.config.ts`
- Modify: `.github/workflows/frontend.yml`
- Test: `apps/web/tests/e2e/auth.spec.ts`

- [ ] **Step 7.1: /login page**

`apps/web/app/(public)/login/page.tsx`:

```tsx
import { useTranslations } from "next-intl";
import { Button } from "@gongzzang/ui";

export default function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ returnTo?: string }>;
}) {
  return <LoginForm searchParams={searchParams} />;
}

async function LoginForm({
  searchParams,
}: {
  searchParams: Promise<{ returnTo?: string }>;
}) {
  const params = await searchParams;
  const returnTo = params.returnTo ?? "/profile";

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-6 p-8">
      <h1 className="text-2xl font-bold">로그인</h1>
      <p className="text-center text-muted-foreground">
        공짱에 오신 것을 환영해요
      </p>

      <form action="/api/auth/login" method="POST" className="w-full">
        <input type="hidden" name="returnTo" value={returnTo} />
        <Button type="submit" className="w-full">
          로그인하기
        </Button>
      </form>
    </main>
  );
}
```

(NOTE: `useTranslations` 는 Server Component 에서는 `getTranslations` 사용. 위 코드는 단순화 — 실제로는 i18n key 호출. 후속 step 에서 i18n 정확히.)

- [ ] **Step 7.2: i18n 정확히 적용**

`apps/web/app/(public)/login/page.tsx` 수정:

```tsx
import { getTranslations } from "next-intl/server";
import { Button } from "@gongzzang/ui";

export default async function LoginPage({
  searchParams,
}: {
  searchParams: Promise<{ returnTo?: string }>;
}) {
  const t = await getTranslations("auth.login");
  const params = await searchParams;
  const returnTo = params.returnTo ?? "/profile";

  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-6 p-8">
      <h1 className="text-2xl font-bold">{t("title")}</h1>
      <p className="text-center text-muted-foreground">{t("description")}</p>

      <form action="/api/auth/login" method="POST" className="w-full">
        <input type="hidden" name="returnTo" value={returnTo} />
        <Button type="submit" className="w-full">
          {t("loginButton")}
        </Button>
      </form>
    </main>
  );
}
```

- [ ] **Step 7.3: /forbidden page**

`apps/web/app/(public)/forbidden/page.tsx`:

```tsx
import { getTranslations } from "next-intl/server";

export default async function ForbiddenPage() {
  const t = await getTranslations("auth.forbidden");
  return (
    <main className="mx-auto flex min-h-screen max-w-md flex-col items-center justify-center gap-4 p-8 text-center">
      <h1 className="text-2xl font-bold">{t("title")}</h1>
      <p className="text-muted-foreground">{t("description")}</p>
    </main>
  );
}
```

- [ ] **Step 7.4: (authenticated)/layout.tsx**

`apps/web/app/(authenticated)/layout.tsx`:

```tsx
import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

export default async function AuthenticatedLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const cookieStore = await cookies();
  const sid = cookieStore.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    redirect("/login");
  }
  const session = await getSession(sid);
  if (!session) {
    redirect("/login");
  }
  return <>{children}</>;
}
```

(NOTE: middleware.ts 가 이미 redirect — 이 layout 은 defense-in-depth. SSR 단계에서 한번 더 검증.)

- [ ] **Step 7.5: /profile page**

`apps/web/app/(authenticated)/profile/page.tsx`:

```tsx
import { cookies } from "next/headers";
import { getTranslations } from "next-intl/server";
import { Button } from "@gongzzang/ui";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

export default async function ProfilePage() {
  const t = await getTranslations("auth.profile");
  const cookieStore = await cookies();
  const sid = cookieStore.get(SID_COOKIE_NAME)?.value!;
  const session = (await getSession(sid))!;

  return (
    <main className="mx-auto flex min-h-screen max-w-2xl flex-col gap-6 p-8">
      <h1 className="text-2xl font-bold">{t("title")}</h1>
      <dl className="grid grid-cols-[8rem_1fr] gap-2">
        <dt className="text-muted-foreground">사용자 ID</dt>
        <dd>{session.sub}</dd>
        <dt className="text-muted-foreground">역할</dt>
        <dd>{session.role}</dd>
      </dl>

      <form action="/api/auth/logout" method="POST">
        <Button type="submit" variant="outline">
          {t("logoutButton")}
        </Button>
      </form>
    </main>
  );
}
```

- [ ] **Step 7.6: e2e test**

`apps/web/tests/e2e/auth.spec.ts`:

```typescript
import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test.describe("auth flow", () => {
  test("/login is publicly accessible + a11y", async ({ page }) => {
    await page.goto("/login");
    await expect(page.getByRole("heading", { name: "로그인" })).toBeVisible();
    await expect(page.getByRole("button", { name: "로그인하기" })).toBeVisible();

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  test("unauthenticated /profile redirects to /login", async ({ page }) => {
    const response = await page.goto("/profile");
    expect(page.url()).toContain("/login");
    expect(page.url()).toContain("returnTo=%2Fprofile");
  });

  test("/forbidden displays role-mismatch message + a11y", async ({ page }) => {
    await page.goto("/forbidden");
    await expect(page.getByRole("heading", { name: "접근 권한이 없어요" })).toBeVisible();

    const accessibility = await new AxeBuilder({ page }).analyze();
    expect(accessibility.violations).toEqual([]);
  });

  test("login → callback (mocked Zitadel via real container) → profile", async ({ page, request }) => {
    // 실 Zitadel container 에 dev user 가 있다고 가정 (init-zitadel.sh 가 admin 계정 발급).
    // hosted login UI 자동 입력
    await page.goto("/login");
    await page.click('button[type="submit"]');
    // Zitadel hosted UI
    await page.fill('input[name="loginName"]', "admin@zitadel.localhost");
    await page.click('button[type="submit"]');
    await page.fill('input[name="password"]', "Admin123!");
    await page.click('button[type="submit"]');
    // 로그인 후 /profile 도달
    await page.waitForURL(/\/profile$/);
    await expect(page.getByRole("heading", { name: "내 정보" })).toBeVisible();
  });

  test("logout returns to root with cookie cleared", async ({ page, context }) => {
    // 위 테스트 이후 (또는 별도 setup) — logout 클릭
    await page.goto("/profile");
    await page.click('button:has-text("로그아웃")');
    await page.waitForURL("/");
    const cookies = await context.cookies();
    expect(cookies.find((c) => c.name === "__Host-sid")).toBeUndefined();
  });
});
```

- [ ] **Step 7.7: playwright config — global setup**

`apps/web/playwright.config.ts` 의 `webServer` 직전에 `globalSetup` 추가:

```typescript
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: false, // auth flow 는 sequential
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
    timeout: 180000,
  },
});
```

(globalSetup 은 frontend.yml workflow 의 service container 가 이미 Zitadel + Redis 시작 — local 은 docker compose up 수동.)

- [ ] **Step 7.8: frontend.yml 확장**

`.github/workflows/frontend.yml` 의 `frontend:` job 에 service container + setup 추가:

```yaml
    services:
      redis:
        image: redis:7-alpine
        ports: ["6379:6379"]
        options: >-
          --health-cmd "redis-cli ping"
          --health-interval 5s
          --health-timeout 3s
          --health-retries 5

      zitadel-db:
        image: postgres:16-alpine
        env:
          POSTGRES_USER: zitadel
          POSTGRES_PASSWORD: zitadel-dev
          POSTGRES_DB: zitadel
        ports: ["5433:5432"]
        options: >-
          --health-cmd "pg_isready -U zitadel"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 10

      zitadel:
        image: ghcr.io/zitadel/zitadel:v2.65.1
        env:
          ZITADEL_MASTERKEY: MasterkeyNeedsToHave32Characters
          ZITADEL_DATABASE_POSTGRES_HOST: zitadel-db
          ZITADEL_DATABASE_POSTGRES_PORT: 5432
          ZITADEL_DATABASE_POSTGRES_DATABASE: zitadel
          ZITADEL_DATABASE_POSTGRES_USER_USERNAME: zitadel
          ZITADEL_DATABASE_POSTGRES_USER_PASSWORD: zitadel-dev
          ZITADEL_DATABASE_POSTGRES_USER_SSL_MODE: disable
          ZITADEL_DATABASE_POSTGRES_ADMIN_USERNAME: zitadel
          ZITADEL_DATABASE_POSTGRES_ADMIN_PASSWORD: zitadel-dev
          ZITADEL_DATABASE_POSTGRES_ADMIN_SSL_MODE: disable
          ZITADEL_EXTERNALSECURE: "false"
          ZITADEL_EXTERNALDOMAIN: localhost
          ZITADEL_EXTERNALPORT: 8443
          ZITADEL_FIRSTINSTANCE_ORG_HUMAN_USERNAME: admin
          ZITADEL_FIRSTINSTANCE_ORG_HUMAN_PASSWORD: Admin123!
        ports: ["8443:8080"]

      - name: Init Zitadel project + capture CLIENT_ID
        run: |
          bash infra/zitadel/init-zitadel.sh > /tmp/zitadel.out
          cat /tmp/zitadel.out
          CLIENT_ID=$(grep -E "^  ZITADEL_CLIENT_ID=" /tmp/zitadel.out | cut -d= -f2 | tr -d '[:space:]')
          test -n "$CLIENT_ID" || (echo "::error::Failed to capture CLIENT_ID" && exit 1)
          echo "ZITADEL_CLIENT_ID=$CLIENT_ID" >> "$GITHUB_ENV"
          echo "ZITADEL_AUDIENCE=$CLIENT_ID" >> "$GITHUB_ENV"

      - name: Playwright e2e + a11y
      - name: Playwright e2e + a11y
        run: pnpm --filter=@gongzzang/web test:e2e
        env:
          NEXT_PUBLIC_API_BASE_URL: http://localhost:8080
          ZITADEL_ISSUER: http://localhost:8443
          ZITADEL_CLIENT_ID: ${{ env.ZITADEL_CLIENT_ID }}
          ZITADEL_AUDIENCE: ${{ env.ZITADEL_AUDIENCE }}
          ZITADEL_REDIRECT_URI: http://localhost:3000/api/auth/callback
          REDIS_URL: redis://localhost:6379
          SESSION_SECRET: ${{ secrets.SESSION_SECRET }}
```

(NOTE: `init-zitadel.sh` 가 stdout 으로 CLIENT_ID 출력 → workflow 가 `>> $GITHUB_ENV` 캡처. SESSION_SECRET 은 GitHub repo secret 으로 random 32+ chars 등록.)

- [ ] **Step 7.9: pnpm dev 로컬 검증**

```
docker compose -f infra/zitadel/docker-compose.yml up -d
sleep 30
bash infra/zitadel/init-zitadel.sh > /tmp/zitadel.out
# CLIENT_ID 추출하여 .env.local 에 작성
pnpm --filter=@gongzzang/web dev
```

브라우저로 `http://localhost:3000/login` → "로그인하기" → Zitadel UI → admin 계정 → `/profile` 도달 → 로그아웃 → `/` 복귀 확인.

- [ ] **Step 7.10: e2e + a11y 로컬 실행**

```
pnpm --filter=@gongzzang/web test:e2e
```

Expected: 5 test PASS.

- [ ] **Step 7.11: bundle size 검증**

```
pnpm --filter=@gongzzang/web test:bundle
```

Expected: under 200 KB threshold (size-limit 설정).

- [ ] **Step 7.12: Commit**

```bash
git add apps/web/app/(public)/ apps/web/app/(authenticated)/ apps/web/tests/e2e/auth.spec.ts apps/web/playwright.config.ts .github/workflows/frontend.yml
git commit -m "feat(6i-T7): /login + /forbidden + /profile + e2e (Zitadel real container) + a11y

- /(public)/login: 해요체 i18n + 'returnTo' 보존
- /(public)/forbidden: 403 화면
- /(authenticated)/layout.tsx: SSR getSession 검증 (defense-in-depth)
- /(authenticated)/profile: sub/role 표시 + 로그아웃
- e2e: 5 시나리오 (login + 미인증 redirect + forbidden + login flow + logout)
- frontend.yml: Zitadel + Redis service container + init-zitadel.sh"
```

---
