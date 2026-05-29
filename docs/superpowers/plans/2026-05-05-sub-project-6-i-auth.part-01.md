# Sub-project 6-i Auth Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Frontend OIDC PKCE 흐름 + Redis backed session + JTI denylist + audit_log emit + RFC 7807 + i18n + CSP/HSTS + rate limit 으로 production 등급 인증 코어 구축.

**Architecture:** Browser → `__Host-sid` cookie → Next.js middleware (rate limit + CSP + auth gate) → Redis (session + denylist + ratelimit) → `/api/proxy` 가 sid → access_token Bearer 변환 → Rust `services/api` `crates/auth` Verifier (JTI denylist 추가 검증) → audit_log INSERT.

**Tech Stack:** Next.js 16.2 / React 19 / oauth4webapi / ioredis / pino / @opentelemetry/api / Zitadel self-host / Postgres / Rust 1.88 / sqlx / deadpool-redis.

**Spec:** `docs/superpowers/specs/2026-05-05-sub-project-6-i-auth-design.md` (`feeaeb3`).

**Sub-project 분해 (B 결정):** 본 SP6-i 는 **인증 코어** 만. multi-org / KISA 본인확인 / 카카오·네이버 federation / Zitadel infra-as-code 는 각각 SP6-org / SP6-CI / SP6-Social / SP6-iam-infra.

---

## File Structure

각 파일 한 가지 책임. 1500줄 안티패턴 회피 (모두 ≤300줄 목표).

### `infra/zitadel/` (T1)

| 파일 | 책임 |
|---|---|
| `docker-compose.yml` | Zitadel + Postgres + Redis (dev) container 정의 |
| `init-zitadel.sh` | machine user 생성 → project / app / redirect URI 등록 (idempotent) |

### `apps/web/lib/` (T2-T4)

| 파일 | 책임 |
|---|---|
| `env.ts` (modify) | zod schema 확장: `ZITADEL_*`, `REDIS_URL`, `SESSION_SECRET` |
| `session/redis.ts` | ioredis singleton (lazy connection) |
| `session/store.ts` | `createSession`, `getSession`, `deleteSession`, `refreshSession` |
| `session/single-flight.ts` | Redis SETNX 기반 mutex (`acquire`, `release`, `withLock`) |
| `session/cookie.ts` | `__Host-sid` cookie set / get / delete helpers |
| `oidc.ts` | oauth4webapi 래퍼: `discover`, `authorizationUrl`, `exchange`, `endSession` |
| `http/problem.ts` | RFC 7807 ProblemDetails class + `problem()` factory |
| `ratelimit.ts` | Redis sliding window: `checkRate(key, limit, windowSec)` |
| `observability/redact.ts` | pino redaction config (token / ci 마스킹) |
| `observability/logger.ts` | pino logger singleton (redact 적용) |
| `observability/tracer.ts` | OpenTelemetry span helper (`withSpan`) |
| `i18n/messages/auth.ko.json` | auth UI / error string SSOT |

### `apps/web/app/api/auth/` (T3)

| 파일 | 책임 |
|---|---|
| `login/route.ts` | PKCE start → state/verifier cookie → Zitadel redirect |
| `callback/route.ts` | code 교환 → JWT 검증 → session 발급 → returnTo redirect |
| `logout/route.ts` | back-channel: JTI denylist + Redis del + Zitadel end_session |
| `refresh/route.ts` | single-flight refresh (silent renew) |

### `apps/web/middleware.ts` + `next.config.ts` (T4)

| 파일 | 책임 |
|---|---|
| `middleware.ts` | rate limit + CSP nonce 주입 + auth gate (path 분기 RBAC) |
| `next.config.ts` (modify) | strict CSP nonce + HSTS preload + X-Frame-Options DENY |
| `instrumentation.ts` (modify) | OTel SDK init (Sentry connector 자리는 SP7-i) |
| `app/api/proxy/[...path]/route.ts` (modify) | sid → access_token → Bearer 변환 |

### `apps/web/app/(public)` + `(authenticated)/` (T7)

| 파일 | 책임 |
|---|---|
| `(public)/login/page.tsx` | "로그인" / "가입" 버튼 (i18n) |
| `(public)/forbidden/page.tsx` | 403 화면 |
| `(authenticated)/layout.tsx` | Server Component, getSession() 강제 |
| `(authenticated)/profile/page.tsx` | /me 표시 + 로그아웃 버튼 |

### `crates/auth/` (T5)

| 파일 | 책임 |
|---|---|
| `src/jti_denylist.rs` | `JtiDenylist` trait + `RedisJtiDenylist` impl |
| `src/audit.rs` | `AuthEvent` enum + `AuditWriter` (sqlx INSERT) |
| `src/claims.rs` (modify) | `jti` field 추가 |
| `src/middleware.rs` (modify) | verify 후 JTI denylist check + Login event emit |
| `src/lib.rs` (modify) | `pub mod jti_denylist; pub mod audit;` |
| `Cargo.toml` (modify) | `deadpool-redis`, `sqlx` 추가 |

### `services/api/src/` + `migrations/` (T5, T6)

| 파일 | 책임 |
|---|---|
| `routes/auth_event.rs` | `POST /internal/auth/event` (frontend AuthEvent 수신) |
| `main.rs` (modify) | jti_denylist + auth_event 라우트 |
| `migrations/30008_user_ci_external_account.sql` | `users.ci` + `external_account` 자리 |

### CI / config (T1-T7)

| 파일 | 변경 |
|---|---|
| `lefthook.yml` | pre-push 에 `cargo sqlx prepare --check` 추가 |
| `tarpaulin.toml` | `crates/auth/jti_denylist.rs`, `audit.rs` include (90% threshold) |
| `deny.toml` | 신규 transitive RUSTSEC 검토 |
| `.github/workflows/frontend.yml` | e2e 에 Zitadel + Redis service container 추가 |
| `apps/web/playwright.config.ts` | webServer global setup (docker compose up) |
| `apps/web/package.json` | `oauth4webapi`, `ioredis`, `pino`, `@opentelemetry/*` 추가 |

### `docs/auth/` (T8)

| 파일 | 책임 |
|---|---|
| `frontend-integration.md` | 운영 SSOT (실행/디버그/장애 대응) |

---

## Task 1: Zitadel + Redis dev infra + zod env schema

**Files:**
- Create: `infra/zitadel/docker-compose.yml`
- Create: `infra/zitadel/init-zitadel.sh`
- Create: `apps/web/.env.local.example`
- Modify: `apps/web/lib/env.ts`
- Modify: `apps/web/package.json` (devDependencies — 없음. 의존성 추가는 T2 부터)
- Test: `apps/web/tests/unit/env.test.ts`

- [ ] **Step 1.1: env.ts 확장 — failing test**

`apps/web/tests/unit/env.test.ts` 생성:

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";

describe("env schema (SP6-i extension)", () => {
  beforeEach(() => {
    vi.resetModules();
  });

  it("parses ZITADEL_* and REDIS_URL when set", async () => {
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "demo-client";
    process.env.ZITADEL_AUDIENCE = "demo-client";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";
    process.env.SESSION_SECRET = "x".repeat(32);
    process.env.NEXT_PUBLIC_API_BASE_URL = "http://localhost:8080";

    const { env } = await import("@/lib/env");
    expect(env.ZITADEL_ISSUER).toBe("http://localhost:8443");
    expect(env.SESSION_SECRET.length).toBeGreaterThanOrEqual(32);
  });

  it("throws on missing SESSION_SECRET", async () => {
    delete process.env.SESSION_SECRET;
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";

    await expect(import("@/lib/env")).rejects.toThrow(/Invalid environment/);
  });

  it("throws on too-short SESSION_SECRET (< 32 chars)", async () => {
    process.env.SESSION_SECRET = "short";
    process.env.ZITADEL_ISSUER = "http://localhost:8443";
    process.env.ZITADEL_CLIENT_ID = "x";
    process.env.ZITADEL_AUDIENCE = "x";
    process.env.ZITADEL_REDIRECT_URI = "http://localhost:3000/api/auth/callback";
    process.env.REDIS_URL = "redis://localhost:6379";

    await expect(import("@/lib/env")).rejects.toThrow();
  });
});
```

- [ ] **Step 1.2: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/env.test.ts
```

Expected: FAIL — 현재 env.ts 에 ZITADEL_* / REDIS_URL / SESSION_SECRET 필드 없음.

- [ ] **Step 1.3: env.ts 확장**

`apps/web/lib/env.ts` 전체 교체:

```typescript
import { z } from "zod";

const EnvSchema = z.object({
  // SP6-foundation
  NEXT_PUBLIC_API_BASE_URL: z.string().url().default("http://localhost:8080"),

  // SP6-i: Zitadel OIDC
  ZITADEL_ISSUER: z.string().url(),
  ZITADEL_CLIENT_ID: z.string().min(1),
  ZITADEL_AUDIENCE: z.string().min(1),
  ZITADEL_REDIRECT_URI: z.string().url(),

  // SP6-i: Redis session + ratelimit
  REDIS_URL: z.string().url(),

  // SP6-i: cookie sealing (iron-session 호환 길이 32+)
  SESSION_SECRET: z.string().min(32),
});

const parsed = EnvSchema.safeParse({
  NEXT_PUBLIC_API_BASE_URL: process.env.NEXT_PUBLIC_API_BASE_URL,
  ZITADEL_ISSUER: process.env.ZITADEL_ISSUER,
  ZITADEL_CLIENT_ID: process.env.ZITADEL_CLIENT_ID,
  ZITADEL_AUDIENCE: process.env.ZITADEL_AUDIENCE,
  ZITADEL_REDIRECT_URI: process.env.ZITADEL_REDIRECT_URI,
  REDIS_URL: process.env.REDIS_URL,
  SESSION_SECRET: process.env.SESSION_SECRET,
});

if (!parsed.success) {
  throw new Error(
    `Invalid environment variables: ${JSON.stringify(parsed.error.flatten().fieldErrors)}`,
  );
}

export const env = parsed.data;
export type Env = z.infer<typeof EnvSchema>;
```

- [ ] **Step 1.4: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/env.test.ts
```

Expected: PASS (3/3).

- [ ] **Step 1.5: `.env.local.example` 작성**

`apps/web/.env.local.example`:

```
# SP6-foundation
NEXT_PUBLIC_API_BASE_URL=http://localhost:8080

# SP6-i — Zitadel OIDC (dev: docker-compose 자동 발급)
ZITADEL_ISSUER=http://localhost:8443
ZITADEL_CLIENT_ID=gongzzang-web-dev
ZITADEL_AUDIENCE=gongzzang-web-dev
ZITADEL_REDIRECT_URI=http://localhost:3000/api/auth/callback

# SP6-i — Redis (session + denylist + ratelimit)
REDIS_URL=redis://localhost:6379

# SP6-i — cookie sealing (32+ chars; production 은 Pulumi secret)
SESSION_SECRET=change-me-to-random-32-byte-base64-string-aaaaaaaaaa
```

- [ ] **Step 1.6: docker-compose.yml 작성**

`infra/zitadel/docker-compose.yml`:

```yaml
services:
  zitadel-db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: zitadel
      POSTGRES_PASSWORD: zitadel-dev
      POSTGRES_DB: zitadel
    volumes:
      - zitadel-db:/var/lib/postgresql/data
    ports:
      - "5433:5432"
    healthcheck:
      test: ["CMD", "pg_isready", "-U", "zitadel"]
      interval: 5s
      timeout: 5s
      retries: 10

  zitadel:
    image: ghcr.io/zitadel/zitadel:v2.65.1
    command: ["start-from-init", "--masterkeyFromEnv", "--tlsMode", "disabled"]
    environment:
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
    ports:
      - "8443:8080"
    depends_on:
      zitadel-db:
        condition: service_healthy

  redis:
    image: redis:7-alpine
    command: ["redis-server", "--appendonly", "yes"]
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

volumes:
  zitadel-db:
  redis-data:
```

- [ ] **Step 1.7: init-zitadel.sh 작성**

`infra/zitadel/init-zitadel.sh`:

```bash
#!/usr/bin/env bash
# Idempotent: Zitadel 에 dev project + OIDC app 등록.
# 첫 부팅 후 1회 실행. 이미 존재하면 skip.
set -euo pipefail

ZITADEL_HOST="${ZITADEL_HOST:-http://localhost:8443}"
ADMIN_USER="${ADMIN_USER:-admin@zitadel.localhost}"
ADMIN_PASS="${ADMIN_PASS:-Admin123!}"

echo "==> Waiting for Zitadel readiness at ${ZITADEL_HOST}/debug/healthz ..."
for i in {1..60}; do
  if curl -sf "${ZITADEL_HOST}/debug/healthz" >/dev/null; then
    echo "    ready."
    break
  fi
  sleep 2
done

echo "==> Login as admin (Zitadel REST)"
TOKEN=$(curl -sf -X POST "${ZITADEL_HOST}/oauth/v2/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=${ADMIN_USER}&password=${ADMIN_PASS}&scope=openid+profile+urn:zitadel:iam:org:project:id:zitadel:aud" \
  | jq -r '.access_token')

if [[ -z "$TOKEN" || "$TOKEN" == "null" ]]; then
  echo "ERROR: failed to obtain admin token"
  exit 1
fi

echo "==> Create project 'gongzzang-dev'"
PROJECT_ID=$(curl -sf -X POST "${ZITADEL_HOST}/management/v1/projects" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{"name":"gongzzang-dev"}' \
  | jq -r '.id // empty')

if [[ -z "$PROJECT_ID" ]]; then
  echo "    project may exist; fetching..."
  PROJECT_ID=$(curl -sf "${ZITADEL_HOST}/management/v1/projects/_search" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{"queries":[{"nameQuery":{"name":"gongzzang-dev","method":"TEXT_QUERY_METHOD_EQUALS"}}]}' \
    | jq -r '.result[0].id')
fi

echo "    PROJECT_ID=${PROJECT_ID}"

echo "==> Create OIDC app 'gongzzang-web-dev' with PKCE + back-channel logout"
APP_RESP=$(curl -sf -X POST "${ZITADEL_HOST}/management/v1/projects/${PROJECT_ID}/apps/oidc" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "gongzzang-web-dev",
    "redirectUris": ["http://localhost:3000/api/auth/callback"],
    "postLogoutRedirectUris": ["http://localhost:3000/"],
    "responseTypes": ["OIDC_RESPONSE_TYPE_CODE"],
    "grantTypes": ["OIDC_GRANT_TYPE_AUTHORIZATION_CODE","OIDC_GRANT_TYPE_REFRESH_TOKEN"],
    "appType": "OIDC_APP_TYPE_WEB",
    "authMethodType": "OIDC_AUTH_METHOD_TYPE_NONE",
    "version": "OIDC_VERSION_1_0",
    "devMode": true,
    "accessTokenType": "OIDC_TOKEN_TYPE_JWT",
    "idTokenRoleAssertion": true,
    "accessTokenRoleAssertion": true,
    "skipNativeAppSuccessPage": true
  }')

CLIENT_ID=$(echo "$APP_RESP" | jq -r '.clientId // empty')
echo "    CLIENT_ID=${CLIENT_ID}"

cat <<EOF

================================================================
Zitadel dev setup complete.

Add to apps/web/.env.local:
  ZITADEL_CLIENT_ID=${CLIENT_ID}
  ZITADEL_AUDIENCE=${CLIENT_ID}

Verify: open http://localhost:8443/ui/console (admin / Admin123!)
================================================================
EOF
```

`chmod +x infra/zitadel/init-zitadel.sh`

- [ ] **Step 1.8: Smoke test — local docker compose**

```
docker compose -f infra/zitadel/docker-compose.yml up -d
sleep 30
bash infra/zitadel/init-zitadel.sh
```

Expected: stdout 에 `CLIENT_ID=...` 출력, `http://localhost:8443/ui/console` 접속 가능.

- [ ] **Step 1.9: pnpm typecheck**

```
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 1.10: Commit**

```bash
git add infra/zitadel/ apps/web/.env.local.example apps/web/lib/env.ts apps/web/tests/unit/env.test.ts
git commit -m "feat(6i-T1): zitadel + redis dev docker-compose + zod env schema

ZITADEL_*, REDIS_URL, SESSION_SECRET 추가. init-zitadel.sh 가 PKCE 가능 OIDC app
'gongzzang-web-dev' 등록 (idempotent). Production 은 SP6-iam-infra 가 Pulumi 화."
```

---

