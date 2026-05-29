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

## Task 8: docs/auth/frontend-integration.md (운영 SSOT)

**Files:**
- Create: `docs/auth/frontend-integration.md`
- Modify: `docs/auth/README.md` (link 추가)

- [ ] **Step 8.1: docs/auth/frontend-integration.md 작성**

`docs/auth/frontend-integration.md` (full content):

````markdown
# Frontend Auth Integration — 운영 SSOT

> SP6-i 의 운영 가이드. 디버깅·장애 대응·로컬 개발 절차의 단일 출처.

## 1. 로컬 개발 환경

```bash
# 1. Zitadel + Redis dev container 시작
docker compose -f infra/zitadel/docker-compose.yml up -d

# 2. Zitadel 첫 부팅 후 (~30초 대기)
sleep 30

# 3. OIDC app 등록 (idempotent)
bash infra/zitadel/init-zitadel.sh > /tmp/zitadel.out
cat /tmp/zitadel.out  # CLIENT_ID 확인

# 4. apps/web/.env.local 작성 (CLIENT_ID 반영)
cp apps/web/.env.local.example apps/web/.env.local
# CLIENT_ID 수정

# 5. 백엔드 실행 (별도 터미널)
DATABASE_URL=postgres://gongzzang:gongzzang@localhost:5432/gongzzang \
ZITADEL_ISSUER=http://localhost:8443 \
ZITADEL_AUDIENCE=$CLIENT_ID \
REDIS_URL=redis://localhost:6379 \
cargo run -p api

# 6. 프론트엔드 실행
pnpm --filter=@gongzzang/web dev
```

브라우저로 http://localhost:3000/login → admin@zitadel.localhost / Admin123! 로 로그인.

## 2. 인증 흐름

```
사용자 → /login → POST /api/auth/login (PKCE start, tmp cookie 발급)
       → 302 → Zitadel /oauth/v2/authorize
       → 사용자 인증 → 302 → /api/auth/callback?code=&state=
       → state CSRF 검증 → token exchange → Redis session 발급 (sid)
       → Set-Cookie __Host-sid → 302 → returnTo (default /profile)
```

## 3. 디버깅

| 증상 | 원인 후보 | 확인 방법 |
|---|---|---|
| `/login` 누르면 401 state mismatch | tmp cookie 만료 (10분) 또는 SameSite | `__Host-auth-tmp` 쿠키 존재 확인 |
| `/profile` 가 무한 redirect | Redis 연결 실패 → session null | `redis-cli ping`, middleware fail-closed |
| 401 token revoked | logout 후 재사용 시도, 또는 role 변경 직후 | Redis `GET jti:deny:<jti>` 확인 |
| 403 forbidden | role 이 admin/broker 아님 | profile 화면에서 role 확인, backend `users.roles` 확인 |
| 429 rate limit | login 5/min/IP 초과 | Redis `ZRANGE rate:login:<ip> 0 -1 WITHSCORES` |

## 4. 장애 대응

### Zitadel 다운
- 기존 session 은 access_token TTL (5분) 까지 동작
- 만료 후 refresh 시도 → fail → frontend 가 /login redirect → Zitadel 다운 시 503
- 영향: 신규 로그인 + token refresh 차단. 기존 세션 처리는 가용

### Redis 다운
- `getSession` fail → middleware 가 /login redirect (closed-fail)
- JTI denylist check 도 fail → backend Verifier 가 fail-open 정책 (가용성 우선)
- audit_log emit fail → tracing::warn 로깅, 사용자 영향 없음

### Postgres 다운
- frontend 인증은 동작 (Zitadel + Redis 만 의존)
- backend `/me` 등 user 조회 실패 → 502 → frontend RFC 7807 응답

## 5. JTI denylist 운영

```bash
# 특정 jti 무효화 (관리자 수동 — role 변경 시 backend 가 자동 처리)
redis-cli SET jti:deny:<jti> 1 EX 300

# 활성 deny 목록
redis-cli KEYS "jti:deny:*"

# 사용자의 모든 활성 jti (role 변경 직전 조회)
psql -c "SELECT after_state->>'jti' FROM audit_log
         WHERE actor_id = '<user_id>'
           AND action IN ('auth.login', 'auth.refresh.succeeded')
           AND created_at > now() - interval '30 days';"
```

## 6. 모니터링 (SP7-i 통합 후)

| 메트릭 | 임계 | 의미 |
|---|---|---|
| `auth.login.failure_rate` | > 5% | Zitadel 또는 frontend 버그 |
| `auth.refresh.failure_rate` | > 1% | Zitadel down 또는 refresh_token 만료 비율 비정상 |
| `auth.role_guard.denied` | spike | 권한 설정 오류 또는 공격 |
| `redis.session.miss_rate` | > 0.1% | Redis 데이터 손실 또는 TTL 설정 오류 |

## 7. 미래 sub-project 의 자리

- **SP6-CI** (KISA 본인확인): `users.ci` 채움. NICE/Toss SDK 통합 + CI state machine.
- **SP6-Social**: 카카오/네이버/Google federation. `external_account` 가 매 provider 채워짐. 동일인 매칭 = `users.ci` UNIQUE.
- **SP6-org**: organization 분리, JWT `org_id` claim, org switcher UI.
- **SP6-iam-infra**: Zitadel self-host 의 Pulumi 코드화, JWKS rotation, DB backup, alert.

## 8. Spec / Plan 참조

- Spec: `docs/superpowers/specs/2026-05-05-sub-project-6-i-auth-design.md`
- Plan: `docs/superpowers/plans/2026-05-05-sub-project-6-i-auth.md`
- ADR-0005: `docs/adr/0005-auth-zitadel.md`
````

- [ ] **Step 8.2: docs/auth/README.md 에 link 추가**

`docs/auth/README.md` 의 적절한 섹션에:

```markdown
## Frontend 통합

- [SP6-i Frontend Integration](./frontend-integration.md) — 로컬 개발 / 디버깅 / 장애 대응
```

- [ ] **Step 8.3: markdownlint**

```
pnpm markdownlint-cli2 docs/auth/frontend-integration.md
```

Expected: 0 errors.

- [ ] **Step 8.4: Commit**

```bash
git add docs/auth/frontend-integration.md docs/auth/README.md
git commit -m "docs(6i-T8): frontend-integration.md 운영 SSOT (로컬 개발 + 디버깅 + 장애 대응)"
```

---

## 최종 검증 (T7 완료 후)

- [ ] **Step F.1: Push + 5 CI workflow 그린 확인**

```
git push origin main
gh run list --branch main --limit 5 --json status,conclusion,name
```

Expected: 5/5 success (CI / db-migrations / walking-skeleton / api-drift-smoke-test / frontend).

- [ ] **Step F.2: smoke 사용자 검증 (수동)**

브라우저로 production-like 시나리오 1회 (로그인 → /profile → 로그아웃) 실행. 로그에 token 노출 없는지 확인 (`pnpm --filter=@gongzzang/web start` + log inspection).

- [ ] **Step F.3: SP6-i 완료 보고 + 다음 sub-project 의향 묻기**

다음 후보 (사용자가 결정):
- SP6-org: multi-org switcher
- SP6-CI: 본인확인 SDK
- SP6-Social: 카카오/네이버 federation
- SP6-ii: 매물 검색 화면 (auth 가 깔린 후 첫 사용자 가치)
- SP6-iam-infra: Zitadel Pulumi (production 배포 직전)

---

## Spec 커버리지 자가 점검

| Spec § | 요구사항 | 구현 task |
|---|---|---|
| 2.1 Zitadel self-host | dev docker-compose | T1 |
| 2.1 OIDC PKCE oauth4webapi | lib/oidc.ts | T3 |
| 2.1 Redis backed session | lib/session/store.ts | T2 |
| 2.1 __Host- cookie + Partitioned | lib/session/cookie.ts | T2 |
| 2.1 Refresh single-flight | lib/session/single-flight.ts + /api/auth/refresh | T2, T3 |
| 2.1 Back-channel logout | /api/auth/logout | T3 |
| 2.1 Path 분기 RBAC | middleware.ts | T4 |
| 2.1 JTI denylist | crates/auth/jti_denylist.rs + middleware hook | T5 |
| 2.1 Role 즉시반영 | audit_log 의 jti 인덱스 활용 (SP6-iv 가 admin UI 추가) | T5 (자리) |
| 2.1 Rate limit | lib/ratelimit.ts + middleware.ts | T4 |
| 2.1 CSP/HSTS | middleware.ts (CSP) + next.config.ts (HSTS) | T4 |
| 2.1 Log redaction | lib/observability/redact.ts | T4 |
| 2.1 lefthook sqlx prepare check | lefthook.yml | T6 |
| 2.1 Audit emit | crates/auth/audit.rs + /internal/auth/event | T5 |
| 2.1 OTel span | lib/observability/tracer.ts + instrumentation.ts | T4 |
| 2.1 RFC 7807 | lib/http/problem.ts (모든 /api/auth/* 응답) | T2, T3 |
| 2.1 i18n auth.ko.json | messages/auth.ko.json | T3 |
| 2.1 a11y WCAG 2.1 AA | tests/e2e/auth.spec.ts (axe-core) | T7 |
| 2.1 V004 schema 자리 | migrations/30008_user_ci_external_account.sql | T6 |
| 5 V004 SQL | 동일 | T6 |
| 6 디렉토리 구조 | T1-T8 분산 | 전체 |
| 8 SSS 7 기둥 | 일관성/자동강제/추적성/안전성/가시성/SSOT/명확성 모두 강제 코드 | T1-T8 |
| 9 Testing 전략 | unit + integration + e2e + a11y | T2-T7 |
| 10 RFC 7807 error 표 | lib/http/problem.ts + i18n auth.errors.* | T2, T3 |

**미반영 = 0**. 모든 spec 요구사항이 task 1개 이상에 매핑됨.
