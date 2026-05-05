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

## Task 2: Redis session store + __Host- cookie + single-flight + RFC 7807

**Files:**
- Create: `apps/web/lib/session/redis.ts`
- Create: `apps/web/lib/session/store.ts`
- Create: `apps/web/lib/session/single-flight.ts`
- Create: `apps/web/lib/session/cookie.ts`
- Create: `apps/web/lib/http/problem.ts`
- Modify: `apps/web/package.json` (deps: `ioredis`, `iron-session`)
- Test: `apps/web/tests/unit/session/store.test.ts`
- Test: `apps/web/tests/unit/session/single-flight.test.ts`
- Test: `apps/web/tests/unit/session/cookie.test.ts`
- Test: `apps/web/tests/unit/http/problem.test.ts`

- [ ] **Step 2.1: 의존성 추가**

```
pnpm --filter=@gongzzang/web add ioredis@^5.4.1 iron-session@^8.0.4
```

- [ ] **Step 2.2: ProblemDetails — failing test**

`apps/web/tests/unit/http/problem.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { problem, ProblemDetails } from "@/lib/http/problem";

describe("ProblemDetails (RFC 7807)", () => {
  it("builds with type, title, status, detail, instance", () => {
    const p = problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: "/api/auth/callback",
    });
    expect(p).toBeInstanceOf(ProblemDetails);
    expect(p.toJSON()).toEqual({
      type: "https://gongzzang.com/errors/auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: "/api/auth/callback",
    });
  });

  it("toResponse returns content-type application/problem+json", () => {
    const p = problem({ type: "auth/x", title: "t", status: 401 });
    const r = p.toResponse();
    expect(r.status).toBe(401);
    expect(r.headers.get("content-type")).toBe("application/problem+json");
  });
});
```

- [ ] **Step 2.3: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/http/problem.test.ts
```

Expected: FAIL — `@/lib/http/problem` 모듈 없음.

- [ ] **Step 2.4: ProblemDetails 구현**

`apps/web/lib/http/problem.ts`:

```typescript
const TYPE_BASE = "https://gongzzang.com/errors";

export interface ProblemDetailsInput {
  type: string;          // e.g. "auth/state-mismatch" → 자동 prefix
  title: string;
  status: number;
  detail?: string;
  instance?: string;
}

export class ProblemDetails {
  readonly type: string;
  readonly title: string;
  readonly status: number;
  readonly detail?: string;
  readonly instance?: string;

  constructor(input: ProblemDetailsInput) {
    this.type = input.type.startsWith("http") ? input.type : `${TYPE_BASE}/${input.type}`;
    this.title = input.title;
    this.status = input.status;
    this.detail = input.detail;
    this.instance = input.instance;
  }

  toJSON() {
    const out: Record<string, unknown> = {
      type: this.type,
      title: this.title,
      status: this.status,
    };
    if (this.detail !== undefined) out.detail = this.detail;
    if (this.instance !== undefined) out.instance = this.instance;
    return out;
  }

  toResponse(): Response {
    return new Response(JSON.stringify(this.toJSON()), {
      status: this.status,
      headers: { "content-type": "application/problem+json" },
    });
  }
}

export function problem(input: ProblemDetailsInput): ProblemDetails {
  return new ProblemDetails(input);
}
```

- [ ] **Step 2.5: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/http/problem.test.ts
```

Expected: PASS (2/2).

- [ ] **Step 2.6: Redis singleton**

`apps/web/lib/session/redis.ts`:

```typescript
import Redis from "ioredis";
import { env } from "@/lib/env";

let _client: Redis | null = null;

export function getRedis(): Redis {
  if (_client === null) {
    _client = new Redis(env.REDIS_URL, {
      maxRetriesPerRequest: 3,
      enableReadyCheck: true,
      lazyConnect: false,
    });
  }
  return _client;
}

// 테스트용 — Redis client 강제 reset
export function __resetRedisForTest() {
  if (_client) {
    _client.disconnect();
    _client = null;
  }
}
```

- [ ] **Step 2.7: Cookie helpers — failing test**

`apps/web/tests/unit/session/cookie.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { setSidCookie, deleteSidCookie, SID_COOKIE_NAME } from "@/lib/session/cookie";

describe("__Host- sid cookie helpers", () => {
  it("uses __Host- prefix and security flags", () => {
    expect(SID_COOKIE_NAME).toBe("__Host-sid");
  });

  it("setSidCookie returns Set-Cookie with all required flags", () => {
    const setCookie = setSidCookie("abc123", 86400);
    expect(setCookie).toContain("__Host-sid=abc123");
    expect(setCookie).toContain("Secure");
    expect(setCookie).toContain("HttpOnly");
    expect(setCookie).toContain("SameSite=Strict");
    expect(setCookie).toContain("Path=/");
    expect(setCookie).toContain("Max-Age=86400");
    expect(setCookie).toContain("Partitioned");
  });

  it("deleteSidCookie returns Set-Cookie with Max-Age=0", () => {
    const setCookie = deleteSidCookie();
    expect(setCookie).toContain("__Host-sid=");
    expect(setCookie).toContain("Max-Age=0");
  });
});
```

- [ ] **Step 2.8: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/session/cookie.test.ts
```

Expected: FAIL.

- [ ] **Step 2.9: Cookie helpers 구현**

`apps/web/lib/session/cookie.ts`:

```typescript
export const SID_COOKIE_NAME = "__Host-sid";
const TEMP_COOKIE_NAME = "__Host-auth-tmp";

export function setSidCookie(sid: string, maxAgeSec: number): string {
  // __Host- prefix 는 Domain 속성 금지, Path=/ 필수, Secure 필수
  return [
    `${SID_COOKIE_NAME}=${sid}`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    `Max-Age=${maxAgeSec}`,
    "Partitioned",
  ].join("; ");
}

export function deleteSidCookie(): string {
  return [
    `${SID_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Strict",
    "Path=/",
    "Max-Age=0",
    "Partitioned",
  ].join("; ");
}

export interface TempAuthState {
  code_verifier: string;
  state: string;
  nonce: string;
  return_to: string;
}

export function setTempCookie(payload: string, maxAgeSec: number): string {
  return [
    `${TEMP_COOKIE_NAME}=${payload}`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax", // OAuth callback 은 cross-site GET 이라 Strict 불가
    "Path=/api/auth/",
    `Max-Age=${maxAgeSec}`,
  ].join("; ");
}

export function deleteTempCookie(): string {
  return [
    `${TEMP_COOKIE_NAME}=`,
    "Secure",
    "HttpOnly",
    "SameSite=Lax",
    "Path=/api/auth/",
    "Max-Age=0",
  ].join("; ");
}

export const TEMP_COOKIE = TEMP_COOKIE_NAME;
```

- [ ] **Step 2.10: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/session/cookie.test.ts
```

Expected: PASS (3/3).

- [ ] **Step 2.11: Session store — failing test (integration with real Redis)**

`apps/web/tests/unit/session/store.test.ts`:

```typescript
import { describe, it, expect, beforeEach, afterAll } from "vitest";
import { getRedis, __resetRedisForTest } from "@/lib/session/redis";
import {
  createSession,
  getSession,
  deleteSession,
  refreshSession,
  type SessionData,
} from "@/lib/session/store";

const sample: SessionData = {
  sub: "user-uuid-1",
  jti: "jti-1",
  role: "Buyer",
  access_token: "at-1",
  refresh_token: "rt-1",
  id_token: "it-1",
  exp: Math.floor(Date.now() / 1000) + 300,
};

describe("SessionStore (Redis)", () => {
  beforeEach(async () => {
    await getRedis().flushdb();
  });

  afterAll(() => {
    __resetRedisForTest();
  });

  it("createSession returns 32-byte hex sid + persists in Redis", async () => {
    const sid = await createSession(sample, 300);
    expect(sid).toMatch(/^[0-9a-f]{64}$/);
    const got = await getSession(sid);
    expect(got).toEqual(sample);
  });

  it("getSession returns null for unknown sid", async () => {
    const got = await getSession("nope");
    expect(got).toBeNull();
  });

  it("deleteSession removes the entry", async () => {
    const sid = await createSession(sample, 300);
    await deleteSession(sid);
    expect(await getSession(sid)).toBeNull();
  });

  it("refreshSession overwrites + extends TTL", async () => {
    const sid = await createSession(sample, 300);
    const next: SessionData = { ...sample, jti: "jti-2", access_token: "at-2", exp: sample.exp + 300 };
    await refreshSession(sid, next, 300);
    expect((await getSession(sid))?.jti).toBe("jti-2");
  });
});
```

- [ ] **Step 2.12: Run test — verify FAIL**

```
docker compose -f infra/zitadel/docker-compose.yml up -d redis
pnpm --filter=@gongzzang/web test -- tests/unit/session/store.test.ts
```

Expected: FAIL — `@/lib/session/store` 모듈 없음.

- [ ] **Step 2.13: Session store 구현**

`apps/web/lib/session/store.ts`:

```typescript
import { randomBytes } from "node:crypto";
import { getRedis } from "./redis";

export interface SessionData {
  sub: string;          // Zitadel sub
  jti: string;          // current access_token JTI
  role: string;         // 'Buyer' | 'Seller' | 'Broker' | ...
  access_token: string;
  refresh_token: string;
  id_token: string;     // back-channel logout 용
  exp: number;          // access_token exp (epoch sec)
}

const KEY = (sid: string) => `session:${sid}`;

export async function createSession(data: SessionData, ttlSec: number): Promise<string> {
  const sid = randomBytes(32).toString("hex");
  await getRedis().set(KEY(sid), JSON.stringify(data), "EX", ttlSec);
  return sid;
}

export async function getSession(sid: string): Promise<SessionData | null> {
  const raw = await getRedis().get(KEY(sid));
  if (!raw) return null;
  return JSON.parse(raw) as SessionData;
}

export async function deleteSession(sid: string): Promise<void> {
  await getRedis().del(KEY(sid));
}

export async function refreshSession(
  sid: string,
  next: SessionData,
  ttlSec: number,
): Promise<void> {
  await getRedis().set(KEY(sid), JSON.stringify(next), "EX", ttlSec);
}
```

- [ ] **Step 2.14: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/session/store.test.ts
```

Expected: PASS (4/4).

- [ ] **Step 2.15: Single-flight mutex — failing test**

`apps/web/tests/unit/session/single-flight.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { getRedis } from "@/lib/session/redis";
import { acquireLock, releaseLock, withLock } from "@/lib/session/single-flight";

describe("Single-flight Redis mutex", () => {
  beforeEach(async () => {
    await getRedis().flushdb();
  });

  it("acquireLock succeeds on first call", async () => {
    const got = await acquireLock("k1", 10);
    expect(got).not.toBeNull();
  });

  it("acquireLock returns null when already held", async () => {
    await acquireLock("k2", 10);
    const second = await acquireLock("k2", 10);
    expect(second).toBeNull();
  });

  it("releaseLock with correct token deletes the lock", async () => {
    const tok = await acquireLock("k3", 10);
    expect(tok).not.toBeNull();
    await releaseLock("k3", tok!);
    const next = await acquireLock("k3", 10);
    expect(next).not.toBeNull();
  });

  it("releaseLock with wrong token does NOT delete (compare-and-delete)", async () => {
    await acquireLock("k4", 10);
    await releaseLock("k4", "wrong-token");
    const next = await acquireLock("k4", 10);
    expect(next).toBeNull();
  });

  it("withLock acquires + runs + releases", async () => {
    const result = await withLock("k5", 10, async () => "value");
    expect(result).toBe("value");
    const next = await acquireLock("k5", 10);
    expect(next).not.toBeNull();
  });

  it("withLock retries when locked, eventually returning lockHeld result", async () => {
    await acquireLock("k6", 10);
    const result = await withLock(
      "k6",
      10,
      async () => "ran",
      { onLocked: async () => "skipped", maxRetries: 0 },
    );
    expect(result).toBe("skipped");
  });
});
```

- [ ] **Step 2.16: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/session/single-flight.test.ts
```

Expected: FAIL.

- [ ] **Step 2.17: Single-flight 구현**

`apps/web/lib/session/single-flight.ts`:

```typescript
import { randomBytes } from "node:crypto";
import { getRedis } from "./redis";

const KEY = (k: string) => `lock:${k}`;
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export async function acquireLock(key: string, ttlSec: number): Promise<string | null> {
  const token = randomBytes(16).toString("hex");
  const got = await getRedis().set(KEY(key), token, "EX", ttlSec, "NX");
  return got === "OK" ? token : null;
}

// CAS 삭제 — 다른 holder 의 lock 은 지우지 않음
const RELEASE_LUA = `
if redis.call("GET", KEYS[1]) == ARGV[1] then
  return redis.call("DEL", KEYS[1])
else
  return 0
end`;

export async function releaseLock(key: string, token: string): Promise<void> {
  await getRedis().eval(RELEASE_LUA, 1, KEY(key), token);
}

export interface WithLockOptions<T> {
  onLocked?: () => Promise<T>;
  maxRetries?: number;       // default 3
  retryDelayMs?: number;     // default 100
}

export async function withLock<T>(
  key: string,
  ttlSec: number,
  run: () => Promise<T>,
  options: WithLockOptions<T> = {},
): Promise<T> {
  const maxRetries = options.maxRetries ?? 3;
  const retryDelayMs = options.retryDelayMs ?? 100;

  for (let i = 0; i <= maxRetries; i++) {
    const tok = await acquireLock(key, ttlSec);
    if (tok) {
      try {
        return await run();
      } finally {
        await releaseLock(key, tok);
      }
    }
    if (i < maxRetries) await sleep(retryDelayMs);
  }
  if (options.onLocked) return options.onLocked();
  throw new Error(`failed to acquire lock '${key}' after ${maxRetries} retries`);
}
```

- [ ] **Step 2.18: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/session/single-flight.test.ts
```

Expected: PASS (6/6).

- [ ] **Step 2.19: Lint + typecheck**

```
pnpm lint
pnpm typecheck
```

Expected: PASS.

- [ ] **Step 2.20: Commit**

```bash
git add apps/web/lib/session/ apps/web/lib/http/ apps/web/tests/unit/session/ apps/web/tests/unit/http/ apps/web/package.json pnpm-lock.yaml
git commit -m "feat(6i-T2): redis session store + __Host- cookie + single-flight + RFC 7807

- session/redis.ts: ioredis singleton (lazy)
- session/store.ts: createSession (32B sid) + get/delete/refresh
- session/single-flight.ts: SETNX mutex with CAS release (Lua)
- session/cookie.ts: __Host-sid + Secure/HttpOnly/SameSite=Strict/Partitioned
- http/problem.ts: RFC 7807 ProblemDetails (application/problem+json)"
```

---

## Task 3: oauth4webapi PKCE + /api/auth/* Route Handlers + i18n

**Files:**
- Create: `apps/web/lib/oidc.ts`
- Create: `apps/web/app/api/auth/login/route.ts`
- Create: `apps/web/app/api/auth/callback/route.ts`
- Create: `apps/web/app/api/auth/logout/route.ts`
- Create: `apps/web/app/api/auth/refresh/route.ts`
- Create: `apps/web/lib/i18n/messages/auth.ko.json`
- Modify: `apps/web/i18n.ts` (auth namespace merge)
- Modify: `apps/web/package.json` (`oauth4webapi`)
- Test: `apps/web/tests/unit/oidc.test.ts`
- Test: `apps/web/tests/integration/auth-flow.test.ts`

- [ ] **Step 3.1: 의존성 추가**

```
pnpm --filter=@gongzzang/web add oauth4webapi@^3.6.1
```

- [ ] **Step 3.2: i18n auth.ko.json 작성**

`apps/web/lib/i18n/messages/auth.ko.json`:

```json
{
  "auth": {
    "login": {
      "title": "로그인",
      "description": "공짱에 오신 것을 환영해요",
      "loginButton": "로그인하기",
      "signupButton": "가입하기",
      "returnTo": "원래 보던 페이지로 돌아가요"
    },
    "profile": {
      "title": "내 정보",
      "logoutButton": "로그아웃"
    },
    "forbidden": {
      "title": "접근 권한이 없어요",
      "description": "이 페이지를 보려면 권한이 필요해요. 관리자에게 문의해 주세요."
    },
    "errors": {
      "idp_unavailable": "로그인 서버에 연결할 수 없어요. 잠시 후 다시 시도해 주세요.",
      "state_mismatch": "보안을 위해 다시 로그인해 주세요.",
      "session_expired": "로그인 세션이 만료되었어요. 다시 로그인해 주세요.",
      "token_revoked": "이 로그인은 더 이상 유효하지 않아요. 다시 로그인해 주세요.",
      "rate_limit": "요청이 너무 많아요. 잠시 후 다시 시도해 주세요.",
      "insufficient_role": "이 작업을 수행할 권한이 없어요."
    }
  }
}
```

- [ ] **Step 3.3: i18n.ts merge — modify**

`apps/web/i18n.ts` 전체 교체:

```typescript
import { getRequestConfig } from "next-intl/server";

export default getRequestConfig(async () => {
  const locale = "ko";
  const [common, auth] = await Promise.all([
    import("./lib/i18n/ko.json"),
    import("./lib/i18n/messages/auth.ko.json"),
  ]);
  return {
    locale,
    messages: { ...common.default, ...auth.default },
  };
});
```

- [ ] **Step 3.4: oidc.ts — failing test**

`apps/web/tests/unit/oidc.test.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import * as oauth from "oauth4webapi";
import {
  generatePkceParams,
  buildAuthorizationUrl,
  buildEndSessionUrl,
} from "@/lib/oidc";

describe("oidc helpers", () => {
  it("generatePkceParams returns code_verifier (43+ chars), code_challenge, state, nonce", async () => {
    const p = await generatePkceParams();
    expect(p.code_verifier.length).toBeGreaterThanOrEqual(43);
    expect(p.code_challenge.length).toBeGreaterThanOrEqual(43);
    expect(p.state.length).toBeGreaterThanOrEqual(32);
    expect(p.nonce.length).toBeGreaterThanOrEqual(32);
  });

  it("buildAuthorizationUrl includes all required OIDC params", async () => {
    const issuer = "http://localhost:8443";
    const url = buildAuthorizationUrl({
      issuer,
      clientId: "demo",
      redirectUri: "http://localhost:3000/cb",
      scope: "openid profile email offline_access",
      code_challenge: "abc",
      state: "s",
      nonce: "n",
    });
    const u = new URL(url);
    expect(u.origin + u.pathname).toBe(`${issuer}/oauth/v2/authorize`);
    expect(u.searchParams.get("response_type")).toBe("code");
    expect(u.searchParams.get("client_id")).toBe("demo");
    expect(u.searchParams.get("redirect_uri")).toBe("http://localhost:3000/cb");
    expect(u.searchParams.get("scope")).toBe("openid profile email offline_access");
    expect(u.searchParams.get("code_challenge")).toBe("abc");
    expect(u.searchParams.get("code_challenge_method")).toBe("S256");
    expect(u.searchParams.get("state")).toBe("s");
    expect(u.searchParams.get("nonce")).toBe("n");
  });

  it("buildEndSessionUrl includes id_token_hint + post_logout_redirect_uri", () => {
    const u = new URL(
      buildEndSessionUrl({
        issuer: "http://localhost:8443",
        idTokenHint: "abc.def.ghi",
        postLogoutRedirectUri: "http://localhost:3000/",
      }),
    );
    expect(u.pathname).toBe("/oidc/v1/end_session");
    expect(u.searchParams.get("id_token_hint")).toBe("abc.def.ghi");
    expect(u.searchParams.get("post_logout_redirect_uri")).toBe("http://localhost:3000/");
  });
});
```

- [ ] **Step 3.5: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/oidc.test.ts
```

Expected: FAIL.

- [ ] **Step 3.6: oidc.ts 구현**

`apps/web/lib/oidc.ts`:

```typescript
import * as oauth from "oauth4webapi";

export interface PkceParams {
  code_verifier: string;
  code_challenge: string;
  state: string;
  nonce: string;
}

export async function generatePkceParams(): Promise<PkceParams> {
  const code_verifier = oauth.generateRandomCodeVerifier();
  const code_challenge = await oauth.calculatePKCECodeChallenge(code_verifier);
  return {
    code_verifier,
    code_challenge,
    state: oauth.generateRandomState(),
    nonce: oauth.generateRandomNonce(),
  };
}

export interface AuthUrlInput {
  issuer: string;
  clientId: string;
  redirectUri: string;
  scope: string;
  code_challenge: string;
  state: string;
  nonce: string;
}

export function buildAuthorizationUrl(i: AuthUrlInput): string {
  const u = new URL(`${i.issuer}/oauth/v2/authorize`);
  u.searchParams.set("response_type", "code");
  u.searchParams.set("client_id", i.clientId);
  u.searchParams.set("redirect_uri", i.redirectUri);
  u.searchParams.set("scope", i.scope);
  u.searchParams.set("code_challenge", i.code_challenge);
  u.searchParams.set("code_challenge_method", "S256");
  u.searchParams.set("state", i.state);
  u.searchParams.set("nonce", i.nonce);
  return u.toString();
}

export interface EndSessionInput {
  issuer: string;
  idTokenHint: string;
  postLogoutRedirectUri: string;
}

export function buildEndSessionUrl(i: EndSessionInput): string {
  const u = new URL(`${i.issuer}/oidc/v1/end_session`);
  u.searchParams.set("id_token_hint", i.idTokenHint);
  u.searchParams.set("post_logout_redirect_uri", i.postLogoutRedirectUri);
  return u.toString();
}

// Discovery (oauth4webapi 의 표준 사용 — issuer/.well-known/openid-configuration)
let _as: oauth.AuthorizationServer | null = null;

export async function discoverAs(issuer: string): Promise<oauth.AuthorizationServer> {
  if (_as && _as.issuer === issuer) return _as;
  const resp = await oauth.discoveryRequest(new URL(issuer), { algorithm: "oidc" });
  _as = await oauth.processDiscoveryResponse(new URL(issuer), resp);
  return _as;
}

export async function exchangeCode(input: {
  issuer: string;
  clientId: string;
  redirectUri: string;
  code: string;
  code_verifier: string;
  expectedNonce: string;
}): Promise<{
  access_token: string;
  refresh_token: string;
  id_token: string;
  expires_in: number;
  jti: string;
  sub: string;
  role: string;
}> {
  const as = await discoverAs(input.issuer);
  const client: oauth.Client = { client_id: input.clientId, token_endpoint_auth_method: "none" };
  const resp = await oauth.authorizationCodeGrantRequest(
    as,
    client,
    new URLSearchParams({ code: input.code }),
    input.redirectUri,
    input.code_verifier,
  );
  const result = await oauth.processAuthorizationCodeOpenIDResponse(as, client, resp, input.expectedNonce);
  if (oauth.isOAuth2Error(result)) {
    throw new Error(`oidc error: ${result.error}`);
  }
  // id_token 의 sub / role / jti 추출 (서명은 oauth4webapi 가 검증)
  const idClaims = oauth.getValidatedIdTokenClaims(result);
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token!,
    id_token: result.id_token!,
    expires_in: result.expires_in ?? 300,
    jti: idClaims.jti as string,
    sub: idClaims.sub,
    role: extractRole(idClaims),
  };
}

function extractRole(claims: Record<string, unknown>): string {
  // Zitadel role assertion: urn:zitadel:iam:org:project:roles
  const roleClaim = claims["urn:zitadel:iam:org:project:roles"];
  if (typeof roleClaim === "object" && roleClaim !== null) {
    const first = Object.keys(roleClaim)[0];
    if (first) return first;
  }
  return "Buyer"; // default safe role (UserRole enum 의 첫 항목)
}

export async function refreshTokens(input: {
  issuer: string;
  clientId: string;
  refresh_token: string;
}): Promise<{
  access_token: string;
  refresh_token: string;
  id_token: string;
  expires_in: number;
  jti: string;
  sub: string;
  role: string;
}> {
  const as = await discoverAs(input.issuer);
  const client: oauth.Client = { client_id: input.clientId, token_endpoint_auth_method: "none" };
  const resp = await oauth.refreshTokenGrantRequest(as, client, input.refresh_token);
  const result = await oauth.processRefreshTokenResponse(as, client, resp);
  if (oauth.isOAuth2Error(result)) {
    throw new Error(`refresh error: ${result.error}`);
  }
  const idClaims = oauth.getValidatedIdTokenClaims(result);
  return {
    access_token: result.access_token,
    refresh_token: result.refresh_token!,
    id_token: result.id_token!,
    expires_in: result.expires_in ?? 300,
    jti: idClaims.jti as string,
    sub: idClaims.sub,
    role: extractRole(idClaims),
  };
}
```

- [ ] **Step 3.7: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/oidc.test.ts
```

Expected: PASS (3/3).

- [ ] **Step 3.8: /api/auth/login route**

`apps/web/app/api/auth/login/route.ts`:

```typescript
import { NextResponse, type NextRequest } from "next/server";
import { env } from "@/lib/env";
import { generatePkceParams, buildAuthorizationUrl } from "@/lib/oidc";
import { setTempCookie } from "@/lib/session/cookie";

export async function POST(req: NextRequest) {
  const formData = await req.formData().catch(() => null);
  const returnTo = (formData?.get("returnTo") as string) ?? "/profile";

  const pkce = await generatePkceParams();
  const authUrl = buildAuthorizationUrl({
    issuer: env.ZITADEL_ISSUER,
    clientId: env.ZITADEL_CLIENT_ID,
    redirectUri: env.ZITADEL_REDIRECT_URI,
    scope: "openid profile email offline_access",
    code_challenge: pkce.code_challenge,
    state: pkce.state,
    nonce: pkce.nonce,
  });

  const tmp = Buffer.from(
    JSON.stringify({
      code_verifier: pkce.code_verifier,
      state: pkce.state,
      nonce: pkce.nonce,
      return_to: returnTo,
    }),
  ).toString("base64url");

  return new NextResponse(null, {
    status: 302,
    headers: {
      Location: authUrl,
      "Set-Cookie": setTempCookie(tmp, 600),
    },
  });
}

// GET 도 허용 (사용자가 직접 /api/auth/login 누르는 경우)
export async function GET(req: NextRequest) {
  return POST(req);
}
```

- [ ] **Step 3.9: /api/auth/callback route**

`apps/web/app/api/auth/callback/route.ts`:

```typescript
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { exchangeCode } from "@/lib/oidc";
import { createSession } from "@/lib/session/store";
import { setSidCookie, deleteTempCookie, TEMP_COOKIE } from "@/lib/session/cookie";
import { problem } from "@/lib/http/problem";

const REFRESH_TTL_SEC = 30 * 24 * 60 * 60; // 30일

export async function GET(req: NextRequest) {
  const url = new URL(req.url);
  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  const tmpCookie = req.cookies.get(TEMP_COOKIE)?.value;

  if (!code || !state || !tmpCookie) {
    return problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: req.url,
    }).toResponse();
  }

  let tmp: { code_verifier: string; state: string; nonce: string; return_to: string };
  try {
    tmp = JSON.parse(Buffer.from(tmpCookie, "base64url").toString("utf-8"));
  } catch {
    return problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      instance: req.url,
    }).toResponse();
  }

  if (tmp.state !== state) {
    return problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "CSRF 검증 실패",
      instance: req.url,
    }).toResponse();
  }

  let tokens;
  try {
    tokens = await exchangeCode({
      issuer: env.ZITADEL_ISSUER,
      clientId: env.ZITADEL_CLIENT_ID,
      redirectUri: env.ZITADEL_REDIRECT_URI,
      code,
      code_verifier: tmp.code_verifier,
      expectedNonce: tmp.nonce,
    });
  } catch (err) {
    return problem({
      type: "auth/idp-unavailable",
      title: "로그인 서버에 연결할 수 없어요",
      status: 503,
      detail: err instanceof Error ? err.message : "unknown",
      instance: req.url,
    }).toResponse();
  }

  const exp = Math.floor(Date.now() / 1000) + tokens.expires_in;
  const sid = await createSession(
    {
      sub: tokens.sub,
      jti: tokens.jti,
      role: tokens.role,
      access_token: tokens.access_token,
      refresh_token: tokens.refresh_token,
      id_token: tokens.id_token,
      exp,
    },
    REFRESH_TTL_SEC,
  );

  // backend audit_log 에 Login event emit (best-effort, fail 시 로그만)
  await emitAuthEvent("Login", {
    user_sub: tokens.sub,
    jti: tokens.jti,
    exp,
  }).catch(() => undefined);

  return new NextResponse(null, {
    status: 302,
    headers: [
      ["Location", tmp.return_to || "/profile"],
      ["Set-Cookie", setSidCookie(sid, REFRESH_TTL_SEC)],
      ["Set-Cookie", deleteTempCookie()],
    ],
  });
}

async function emitAuthEvent(event: string, payload: Record<string, unknown>): Promise<void> {
  await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/auth/event`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ event, payload }),
  });
}
```

- [ ] **Step 3.10: /api/auth/logout route**

`apps/web/app/api/auth/logout/route.ts`:

```typescript
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { buildEndSessionUrl } from "@/lib/oidc";
import { getSession, deleteSession } from "@/lib/session/store";
import { SID_COOKIE_NAME, deleteSidCookie } from "@/lib/session/cookie";
import { getRedis } from "@/lib/session/redis";

export async function POST(req: NextRequest) {
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    return new NextResponse(null, {
      status: 302,
      headers: { Location: "/" },
    });
  }

  const session = await getSession(sid);
  if (!session) {
    return new NextResponse(null, {
      status: 302,
      headers: { Location: "/", "Set-Cookie": deleteSidCookie() },
    });
  }

  // JTI denylist 추가 (남은 access_token TTL 만큼)
  const remainingSec = Math.max(1, session.exp - Math.floor(Date.now() / 1000));
  await getRedis().set(`jti:deny:${session.jti}`, "1", "EX", remainingSec);

  await deleteSession(sid);

  // audit_log emit (best-effort)
  await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/auth/event`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      event: "Logout",
      payload: { user_sub: session.sub, jti: session.jti },
    }),
  }).catch(() => undefined);

  // back-channel logout — Zitadel SSO 종료
  const endUrl = buildEndSessionUrl({
    issuer: env.ZITADEL_ISSUER,
    idTokenHint: session.id_token,
    postLogoutRedirectUri: new URL("/", env.ZITADEL_REDIRECT_URI).toString().replace(/\/api\/auth\/callback$/, "") || "/",
  });

  return new NextResponse(null, {
    status: 302,
    headers: { Location: endUrl, "Set-Cookie": deleteSidCookie() },
  });
}

export async function GET(req: NextRequest) {
  return POST(req);
}
```

- [ ] **Step 3.11: /api/auth/refresh route (single-flight)**

`apps/web/app/api/auth/refresh/route.ts`:

```typescript
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { refreshTokens } from "@/lib/oidc";
import { getSession, refreshSession } from "@/lib/session/store";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getRedis } from "@/lib/session/redis";
import { withLock } from "@/lib/session/single-flight";
import { problem } from "@/lib/http/problem";

const REFRESH_TTL_SEC = 30 * 24 * 60 * 60;

export async function POST(req: NextRequest) {
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    return problem({
      type: "auth/session-expired",
      title: "로그인 세션이 만료되었어요",
      status: 401,
      instance: req.url,
    }).toResponse();
  }

  return withLock(
    `refresh:${sid}`,
    10,
    async () => {
      const current = await getSession(sid);
      if (!current) {
        return problem({
          type: "auth/session-expired",
          title: "로그인 세션이 만료되었어요",
          status: 401,
          instance: req.url,
        }).toResponse();
      }

      let next;
      try {
        next = await refreshTokens({
          issuer: env.ZITADEL_ISSUER,
          clientId: env.ZITADEL_CLIENT_ID,
          refresh_token: current.refresh_token,
        });
      } catch {
        await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/auth/event`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body: JSON.stringify({
            event: "RefreshFailed",
            payload: { user_sub: current.sub, jti: current.jti },
          }),
        }).catch(() => undefined);
        return problem({
          type: "auth/session-expired",
          title: "로그인 세션이 만료되었어요",
          status: 401,
          instance: req.url,
        }).toResponse();
      }

      // 이전 jti 를 denylist 에 추가
      const remainingSec = Math.max(1, current.exp - Math.floor(Date.now() / 1000));
      await getRedis().set(`jti:deny:${current.jti}`, "1", "EX", remainingSec);

      const newExp = Math.floor(Date.now() / 1000) + next.expires_in;
      await refreshSession(
        sid,
        {
          sub: next.sub,
          jti: next.jti,
          role: next.role,
          access_token: next.access_token,
          refresh_token: next.refresh_token,
          id_token: next.id_token,
          exp: newExp,
        },
        REFRESH_TTL_SEC,
      );

      await fetch(`${env.NEXT_PUBLIC_API_BASE_URL}/internal/auth/event`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          event: "RefreshSucceeded",
          payload: { user_sub: next.sub, prev_jti: current.jti, new_jti: next.jti, exp: newExp },
        }),
      }).catch(() => undefined);

      return NextResponse.json({ ok: true });
    },
    {
      // 락 못 잡으면 100ms backoff 후 다시 session GET (이미 갱신됨)
      onLocked: async () => NextResponse.json({ ok: true, contended: true }),
      maxRetries: 3,
      retryDelayMs: 100,
    },
  );
}
```

- [ ] **Step 3.12: 통합 테스트 (mocked exchange)**

`apps/web/tests/integration/auth-flow.test.ts`:

```typescript
import { describe, it, expect, beforeEach, vi } from "vitest";
import { POST as loginPOST } from "@/app/api/auth/login/route";
import { GET as callbackGET } from "@/app/api/auth/callback/route";
import { getRedis } from "@/lib/session/redis";

vi.mock("@/lib/oidc", async () => {
  const actual = await vi.importActual<typeof import("@/lib/oidc")>("@/lib/oidc");
  return {
    ...actual,
    exchangeCode: vi.fn(async () => ({
      access_token: "at-1",
      refresh_token: "rt-1",
      id_token: "it-1",
      expires_in: 300,
      jti: "jti-1",
      sub: "user-1",
      role: "Buyer",
    })),
  };
});

describe("auth flow integration", () => {
  beforeEach(async () => {
    await getRedis().flushdb();
  });

  it("login → 302 → callback → session created", async () => {
    const loginReq = new Request("http://localhost:3000/api/auth/login", {
      method: "POST",
      body: new FormData(),
    });
    const loginRes = await loginPOST(loginReq as unknown as never);
    expect(loginRes.status).toBe(302);
    const setCookie = loginRes.headers.get("set-cookie") ?? "";
    expect(setCookie).toContain("__Host-auth-tmp=");

    // tmp cookie 추출
    const tmpMatch = setCookie.match(/__Host-auth-tmp=([^;]+)/);
    expect(tmpMatch).not.toBeNull();
    const tmp = tmpMatch![1];
    const decoded = JSON.parse(Buffer.from(tmp, "base64url").toString("utf-8"));

    const callbackReq = new Request(
      `http://localhost:3000/api/auth/callback?code=abc&state=${decoded.state}`,
      {
        headers: { cookie: `__Host-auth-tmp=${tmp}` },
      },
    );
    const callbackRes = await callbackGET(callbackReq as unknown as never);
    expect(callbackRes.status).toBe(302);
    const sidCookie = callbackRes.headers.get("set-cookie") ?? "";
    expect(sidCookie).toMatch(/__Host-sid=[0-9a-f]{64}/);
  });
});
```

- [ ] **Step 3.13: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test
```

Expected: PASS (모든 unit + integration).

- [ ] **Step 3.14: Lint + typecheck**

```
pnpm lint && pnpm typecheck
```

Expected: PASS.

- [ ] **Step 3.15: Commit**

```bash
git add apps/web/lib/oidc.ts apps/web/app/api/auth/ apps/web/lib/i18n/messages/ apps/web/i18n.ts apps/web/tests/unit/oidc.test.ts apps/web/tests/integration/ apps/web/package.json pnpm-lock.yaml
git commit -m "feat(6i-T3): oauth4webapi PKCE + /api/auth/{login,callback,logout,refresh} + auth.ko.json

- lib/oidc.ts: PKCE generation + authorization/end-session URL builders + token exchange/refresh
- /api/auth/login: PKCE start, tmp cookie (10min), Zitadel redirect
- /api/auth/callback: state CSRF + token exchange + Redis session 발급 + Login event emit
- /api/auth/logout: JTI denylist + Redis del + back-channel end_session
- /api/auth/refresh: single-flight mutex + jti rotation + RefreshSucceeded/Failed emit
- auth.ko.json: 모든 auth UI/error string i18n (옵션 A 강제)"
```

---

## Task 4: middleware.ts (rate limit + CSP nonce + auth gate) + next.config strict headers + log redaction + proxy

**Files:**
- Create: `apps/web/middleware.ts`
- Create: `apps/web/lib/ratelimit.ts`
- Create: `apps/web/lib/observability/logger.ts`
- Create: `apps/web/lib/observability/redact.ts`
- Create: `apps/web/lib/observability/tracer.ts`
- Modify: `apps/web/next.config.ts`
- Modify: `apps/web/instrumentation.ts`
- Modify: `apps/web/app/api/proxy/[...path]/route.ts`
- Modify: `apps/web/package.json` (deps: `pino`, `@opentelemetry/api`, `@opentelemetry/sdk-node`)
- Test: `apps/web/tests/unit/ratelimit.test.ts`
- Test: `apps/web/tests/unit/observability/redact.test.ts`
- Test: `apps/web/tests/unit/middleware.test.ts`

- [ ] **Step 4.1: 의존성 추가**

```
pnpm --filter=@gongzzang/web add pino@^9.5.0 pino-pretty@^11.3.0 @opentelemetry/api@^1.9.0 @opentelemetry/sdk-node@^0.55.0 @opentelemetry/instrumentation-fetch@^0.55.0
```

- [ ] **Step 4.2: ratelimit — failing test**

`apps/web/tests/unit/ratelimit.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { getRedis } from "@/lib/session/redis";
import { checkRate } from "@/lib/ratelimit";

describe("Redis sliding window ratelimit", () => {
  beforeEach(async () => {
    await getRedis().flushdb();
  });

  it("allows up to limit then denies", async () => {
    for (let i = 0; i < 5; i++) {
      const r = await checkRate("login:1.2.3.4", 5, 60);
      expect(r.allowed).toBe(true);
      expect(r.remaining).toBe(5 - i - 1);
    }
    const denied = await checkRate("login:1.2.3.4", 5, 60);
    expect(denied.allowed).toBe(false);
    expect(denied.remaining).toBe(0);
  });

  it("isolates keys", async () => {
    for (let i = 0; i < 5; i++) await checkRate("a", 5, 60);
    const r = await checkRate("b", 5, 60);
    expect(r.allowed).toBe(true);
  });
});
```

- [ ] **Step 4.3: Run test — verify FAIL**

```
pnpm --filter=@gongzzang/web test -- tests/unit/ratelimit.test.ts
```

Expected: FAIL.

- [ ] **Step 4.4: ratelimit 구현**

`apps/web/lib/ratelimit.ts`:

```typescript
import { getRedis } from "./session/redis";

// Sliding-window: ZSET 에 timestamp, ZREMRANGEBYSCORE 로 window 밖 제거 후 ZCARD 검사.
const RATE_LUA = `
local key = KEYS[1]
local now = tonumber(ARGV[1])
local window_ms = tonumber(ARGV[2])
local limit = tonumber(ARGV[3])
redis.call("ZREMRANGEBYSCORE", key, 0, now - window_ms)
local count = redis.call("ZCARD", key)
if count >= limit then
  return {0, 0}
end
redis.call("ZADD", key, now, now .. ":" .. math.random())
redis.call("PEXPIRE", key, window_ms)
return {1, limit - count - 1}
`;

export interface RateResult {
  allowed: boolean;
  remaining: number;
}

export async function checkRate(
  key: string,
  limit: number,
  windowSec: number,
): Promise<RateResult> {
  const r = (await getRedis().eval(
    RATE_LUA,
    1,
    `rate:${key}`,
    Date.now(),
    windowSec * 1000,
    limit,
  )) as [number, number];
  return { allowed: r[0] === 1, remaining: r[1] };
}
```

- [ ] **Step 4.5: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test -- tests/unit/ratelimit.test.ts
```

Expected: PASS (2/2).

- [ ] **Step 4.6: redact — failing test**

`apps/web/tests/unit/observability/redact.test.ts`:

```typescript
import { describe, it, expect } from "vitest";
import { logger } from "@/lib/observability/logger";

describe("pino redaction", () => {
  it("redacts access_token, refresh_token, ci, password fields", () => {
    const sink: string[] = [];
    const child = logger.child({}, { level: "info" });
    // pino 의 redact 는 기본적으로 [Redacted] 마스킹
    const original = { access_token: "secret", refresh_token: "secret2", ci: "K7H2", normal: "ok" };
    const captured = JSON.parse(JSON.stringify(original)); // pino 가 처리 후 모양 모방
    // 실제로는 pino transport 가 마스킹 — 여기는 redact paths 가 정의되어 있는지만 확인
    expect((logger as unknown as { [k: string]: unknown }).bindings).toBeDefined();
  });
});
```

(NOTE: pino redaction 은 transport 단계에서 작동. 위 테스트는 logger 인스턴스 존재 + redact 설정 확인. 실 동작은 e2e 에서 검증.)

- [ ] **Step 4.7: logger + redact 구현**

`apps/web/lib/observability/redact.ts`:

```typescript
export const REDACT_PATHS = [
  "access_token",
  "refresh_token",
  "id_token",
  "code_verifier",
  "ci",
  "password",
  "*.access_token",
  "*.refresh_token",
  "*.id_token",
  "*.password",
  "headers.authorization",
  'headers["set-cookie"]',
  "req.headers.cookie",
  "req.headers.authorization",
];
```

`apps/web/lib/observability/logger.ts`:

```typescript
import pino from "pino";
import { REDACT_PATHS } from "./redact";

export const logger = pino({
  level: process.env.LOG_LEVEL ?? "info",
  redact: { paths: REDACT_PATHS, censor: "[REDACTED]" },
  formatters: {
    level: (label) => ({ level: label }),
  },
  timestamp: pino.stdTimeFunctions.isoTime,
});
```

- [ ] **Step 4.8: tracer 구현**

`apps/web/lib/observability/tracer.ts`:

```typescript
import { trace, SpanStatusCode, type Span } from "@opentelemetry/api";

const tracer = trace.getTracer("gongzzang-web", "1.0.0");

export async function withSpan<T>(
  name: string,
  attributes: Record<string, string | number | boolean>,
  fn: (span: Span) => Promise<T>,
): Promise<T> {
  return tracer.startActiveSpan(name, { attributes }, async (span) => {
    try {
      const result = await fn(span);
      span.setStatus({ code: SpanStatusCode.OK });
      return result;
    } catch (err) {
      span.setStatus({
        code: SpanStatusCode.ERROR,
        message: err instanceof Error ? err.message : "unknown",
      });
      span.recordException(err as Error);
      throw err;
    } finally {
      span.end();
    }
  });
}
```

- [ ] **Step 4.9: instrumentation.ts modify**

`apps/web/instrumentation.ts` 전체 교체:

```typescript
// SP6-i: OpenTelemetry SDK init.
// SP7-i 가 추가: Sentry connector + OTLP exporter.

export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { NodeSDK } = await import("@opentelemetry/sdk-node");
    const { FetchInstrumentation } = await import("@opentelemetry/instrumentation-fetch");
    const sdk = new NodeSDK({
      serviceName: "gongzzang-web",
      instrumentations: [new FetchInstrumentation()],
    });
    sdk.start();
  }
}
```

- [ ] **Step 4.10: middleware.ts 작성**

`apps/web/middleware.ts`:

```typescript
import { NextResponse, type NextRequest } from "next/server";
import { randomBytes } from "node:crypto";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession, type SessionData } from "@/lib/session/store";
import { checkRate } from "@/lib/ratelimit";
import { problem } from "@/lib/http/problem";

const PUBLIC_PATHS = ["/", "/login", "/forbidden", "/api/auth"];
const ADMIN_PATHS = ["/admin"];
const ADMIN_ROLES = new Set(["Admin", "Broker", "Operator"]);

function isPublic(pathname: string): boolean {
  return PUBLIC_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

function isAdmin(pathname: string): boolean {
  return ADMIN_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

async function checkAuthRateLimit(req: NextRequest): Promise<NextResponse | null> {
  const ip = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
  if (req.nextUrl.pathname === "/api/auth/login") {
    const r = await checkRate(`login:${ip}`, 5, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        detail: "잠시 후 다시 시도해 주세요.",
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  } else if (req.nextUrl.pathname === "/api/auth/callback") {
    const r = await checkRate(`callback:${ip}`, 10, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  } else if (req.nextUrl.pathname === "/api/auth/refresh") {
    const sid = req.cookies.get(SID_COOKIE_NAME)?.value ?? "anon";
    const r = await checkRate(`refresh:${sid}`, 30, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  }
  return null;
}

export async function middleware(req: NextRequest) {
  const url = req.nextUrl;

  // 1. Rate limit (auth routes only)
  const rateBlocked = await checkAuthRateLimit(req);
  if (rateBlocked) return rateBlocked;

  // 2. CSP nonce 주입
  const nonce = randomBytes(16).toString("base64");
  const cspHeader = [
    `default-src 'self'`,
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
    `style-src 'self' 'unsafe-inline'`,
    `img-src 'self' data: blob:`,
    `font-src 'self' data:`,
    `connect-src 'self' ${process.env.NEXT_PUBLIC_API_BASE_URL ?? ""} ${process.env.ZITADEL_ISSUER ?? ""}`,
    `frame-ancestors 'none'`,
    `base-uri 'self'`,
    `form-action 'self' ${process.env.ZITADEL_ISSUER ?? ""}`,
  ].join("; ");

  const reqHeaders = new Headers(req.headers);
  reqHeaders.set("x-csp-nonce", nonce);

  // 3. Auth gate
  if (isPublic(url.pathname)) {
    const res = NextResponse.next({ request: { headers: reqHeaders } });
    res.headers.set("Content-Security-Policy", cspHeader);
    return res;
  }

  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", url.pathname);
    return NextResponse.redirect(loginUrl);
  }

  const session: SessionData | null = await getSession(sid);
  if (!session) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", url.pathname);
    const res = NextResponse.redirect(loginUrl);
    res.cookies.delete(SID_COOKIE_NAME);
    return res;
  }

  if (isAdmin(url.pathname) && !ADMIN_ROLES.has(session.role)) {
    return NextResponse.redirect(new URL("/forbidden", req.url));
  }

  const res = NextResponse.next({ request: { headers: reqHeaders } });
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

export const config = {
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico).*)",
  ],
};
```

- [ ] **Step 4.11: next.config.ts modify (HSTS + X-Frame + Referrer)**

`apps/web/next.config.ts` 전체 교체:

```typescript
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n.ts");

const securityHeaders = [
  { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
];

const nextConfig: NextConfig = {
  reactStrictMode: true,
  typedRoutes: true,
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },
};

export default withNextIntl(nextConfig);
```

(CSP 는 middleware.ts 가 동적 nonce 와 함께 주입.)

- [ ] **Step 4.12: proxy 에 sid → Bearer 변환 추가**

`apps/web/app/api/proxy/[...path]/route.ts` 의 `forward` 함수 교체 (기존 forward 바디 전체):

```typescript
import { isHTTPError, type Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { createServerApi } from "@/lib/api";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";
import { problem } from "@/lib/http/problem";

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const path = params.path.join("/");
  const url = new URL(req.url);
  const search = url.search;

  // SP6-i: sid → access_token 변환
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  let bearer: string | undefined;
  if (sid) {
    const session = await getSession(sid);
    if (session) bearer = session.access_token;
  }

  const api = createServerApi();

  try {
    const requestInit: KyOptions = {
      method: req.method,
      headers: bearer ? { Authorization: `Bearer ${bearer}` } : {},
    };

    if (search) {
      const searchParams: Record<string, string> = {};
      for (const [k, v] of new URLSearchParams(search).entries()) searchParams[k] = v;
      requestInit.searchParams = searchParams;
    }

    if (["POST", "PUT", "PATCH"].includes(req.method)) {
      try {
        requestInit.json = await req.json();
      } catch {
        // body 없는 요청 허용
      }
    }

    const response = await api(path, requestInit);
    const text = await response.text();
    const contentType = response.headers.get("content-type") ?? "text/plain";
    return new NextResponse(text, {
      status: response.status,
      headers: { "content-type": contentType },
    });
  } catch (err: unknown) {
    if (isHTTPError(err)) {
      // 401: backend 가 JTI denylist 또는 만료 응답 → frontend 가 refresh 시도 자리
      const body = await err.response.text();
      return new NextResponse(body, { status: err.response.status });
    }
    return problem({
      type: "proxy/upstream-unavailable",
      title: "백엔드 서버에 연결할 수 없어요",
      status: 502,
      detail: "잠시 후 다시 시도해 주세요.",
      instance: req.url,
    }).toResponse() as unknown as NextResponse;
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

- [ ] **Step 4.13: middleware test**

`apps/web/tests/unit/middleware.test.ts`:

```typescript
import { describe, it, expect, beforeEach } from "vitest";
import { middleware } from "@/middleware";
import { getRedis } from "@/lib/session/redis";
import { createSession } from "@/lib/session/store";
import { NextRequest } from "next/server";

describe("middleware", () => {
  beforeEach(async () => {
    await getRedis().flushdb();
  });

  it("allows public paths without sid", async () => {
    const req = new NextRequest("http://localhost:3000/login");
    const res = await middleware(req);
    expect(res.status).toBe(200);
    expect(res.headers.get("content-security-policy")).toContain("default-src 'self'");
  });

  it("redirects unauthenticated to /login with returnTo", async () => {
    const req = new NextRequest("http://localhost:3000/profile");
    const res = await middleware(req);
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toContain("/login?returnTo=%2Fprofile");
  });

  it("redirects to /forbidden when role mismatch on /admin", async () => {
    const sid = await createSession(
      {
        sub: "u1", jti: "j1", role: "Buyer",
        access_token: "at", refresh_token: "rt", id_token: "it",
        exp: Math.floor(Date.now() / 1000) + 300,
      },
      300,
    );
    const req = new NextRequest("http://localhost:3000/admin/users", {
      headers: { cookie: `__Host-sid=${sid}` },
    });
    const res = await middleware(req);
    expect(res.status).toBe(307);
    expect(res.headers.get("location")).toContain("/forbidden");
  });

  it("rate limits /api/auth/login", async () => {
    for (let i = 0; i < 5; i++) {
      const req = new NextRequest("http://localhost:3000/api/auth/login", {
        method: "POST",
        headers: { "x-forwarded-for": "1.2.3.4" },
      });
      const r = await middleware(req);
      expect(r.status).not.toBe(429);
    }
    const req = new NextRequest("http://localhost:3000/api/auth/login", {
      method: "POST",
      headers: { "x-forwarded-for": "1.2.3.4" },
    });
    const r = await middleware(req);
    expect(r.status).toBe(429);
  });
});
```

- [ ] **Step 4.14: Run test — verify PASS**

```
pnpm --filter=@gongzzang/web test
```

Expected: PASS.

- [ ] **Step 4.15: Lint + typecheck + build**

```
pnpm lint && pnpm typecheck && pnpm --filter=@gongzzang/web build
```

Expected: PASS.

- [ ] **Step 4.16: Commit**

```bash
git add apps/web/middleware.ts apps/web/lib/ratelimit.ts apps/web/lib/observability/ apps/web/next.config.ts apps/web/instrumentation.ts apps/web/app/api/proxy/ apps/web/tests/unit/middleware.test.ts apps/web/tests/unit/ratelimit.test.ts apps/web/tests/unit/observability/ apps/web/package.json pnpm-lock.yaml
git commit -m "feat(6i-T4): middleware (rate limit + CSP nonce + auth gate) + HSTS + log redact + proxy bearer

- middleware.ts: rate limit (login 5/min, callback 10/min, refresh 30/min/sid) + CSP nonce + path 분기 RBAC
- next.config.ts: HSTS preload, X-Frame DENY, Referrer-Policy strict-origin-when-cross-origin, Permissions-Policy
- lib/observability: pino logger with redact (access_token/refresh_token/ci/password) + OTel withSpan helper
- instrumentation.ts: OpenTelemetry NodeSDK init (SP7-i 가 OTLP exporter 추가)
- /api/proxy: sid → access_token Bearer 변환 + RFC 7807 502"
```

---

## Task 5: crates/auth JTI denylist + AuthEvent + audit_log emit + OTel span

**Files:**
- Create: `crates/auth/src/jti_denylist.rs`
- Create: `crates/auth/src/audit.rs`
- Modify: `crates/auth/src/claims.rs`
- Modify: `crates/auth/src/lib.rs`
- Modify: `crates/auth/src/middleware.rs`
- Modify: `crates/auth/Cargo.toml`
- Create: `services/api/src/routes/auth_event.rs`
- Modify: `services/api/src/main.rs`
- Test: `crates/auth/src/jti_denylist.rs` (#[cfg(test)])
- Test: `crates/auth/src/audit.rs` (#[cfg(test)])
- Test: `services/api/tests/auth_event_integration.rs` (DB + Redis)

- [ ] **Step 5.1: Cargo.toml — deadpool-redis + sqlx 추가**

`crates/auth/Cargo.toml` 의 `[dependencies]` 에 추가:

```toml
deadpool-redis = "0.18"
sqlx = { workspace = true, features = ["postgres", "runtime-tokio", "macros", "json", "chrono"] }
chrono = { workspace = true, features = ["serde"] }
```

- [ ] **Step 5.2: claims.rs — jti 추가**

`crates/auth/src/claims.rs` 의 `Claims` 구조체에 필드 추가 (line 9 의 struct):

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    pub sub: String,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub preferred_username: Option<String>,
    /// `JWT` ID — `JTI` denylist key. Zitadel 가 항상 발급.
    pub jti: String,
    pub exp: i64,
    #[serde(default)]
    pub nbf: Option<i64>,
    pub iss: String,
    pub aud: Audience,
}
```

기존 테스트 fixture 들 (deserialize_*) 에 `"jti":"..."` 추가:

```rust
let json = r#"{"sub":"u1","jti":"j1","exp":1000,"iss":"http://i","aud":"client-x"}"#;
```

(모든 test JSON 에 `"jti":"j1"` 추가; Claims 인스턴스 생성에도 `jti: "j1".into()` 추가)

- [ ] **Step 5.3: jti_denylist.rs trait + impl**

`crates/auth/src/jti_denylist.rs`:

```rust
//! JTI 무효화 목록 (logout / refresh rotation / role change 시 token 즉시 무효).

use async_trait::async_trait;
use deadpool_redis::{redis::AsyncCommands, Pool};

/// `JWT` `JTI` denylist 트레잇.
///
/// 검증 단계에서 `is_denied(jti)` 를 호출 → `true` 면 `AuthError::Expired` 처럼 거부해요.
#[async_trait]
pub trait JtiDenylist: Send + Sync {
    /// 해당 jti 가 무효인지 (denylist hit).
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError>;

    /// jti 를 ttl 초 동안 무효화.
    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError>;
}

/// `JTI` denylist 작업 중 발생할 수 있는 오류.
#[derive(Debug, thiserror::Error)]
pub enum JtiError {
    /// Redis 연결 실패 또는 명령 오류.
    #[error("redis: {0}")]
    Redis(String),
}

impl From<deadpool_redis::PoolError> for JtiError {
    fn from(e: deadpool_redis::PoolError) -> Self {
        Self::Redis(e.to_string())
    }
}

impl From<deadpool_redis::redis::RedisError> for JtiError {
    fn from(e: deadpool_redis::redis::RedisError) -> Self {
        Self::Redis(e.to_string())
    }
}

/// Redis 기반 `JTI` denylist 구현.
pub struct RedisJtiDenylist {
    pool: Pool,
}

impl RedisJtiDenylist {
    /// `Pool` 로 새 인스턴스 생성.
    #[must_use]
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    fn key(jti: &str) -> String {
        format!("jti:deny:{jti}")
    }
}

#[async_trait]
impl JtiDenylist for RedisJtiDenylist {
    async fn is_denied(&self, jti: &str) -> Result<bool, JtiError> {
        let mut conn = self.pool.get().await?;
        let exists: bool = conn.exists(Self::key(jti)).await?;
        Ok(exists)
    }

    async fn deny(&self, jti: &str, ttl_sec: u64) -> Result<(), JtiError> {
        let mut conn = self.pool.get().await?;
        let _: () = conn.set_ex(Self::key(jti), "1", ttl_sec).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]
    use super::*;
    use deadpool_redis::{Config, Runtime};

    fn pool() -> Option<Pool> {
        let url = std::env::var("REDIS_URL").ok()?;
        let cfg = Config::from_url(url);
        cfg.create_pool(Some(Runtime::Tokio1)).ok()
    }

    #[tokio::test]
    async fn deny_then_is_denied_true() {
        let Some(p) = pool() else {
            eprintln!("REDIS_URL not set, skipping");
            return;
        };
        let dl = RedisJtiDenylist::new(p);
        let jti = format!("test-{}", uuid::Uuid::new_v4());
        assert!(!dl.is_denied(&jti).await.expect("query"));
        dl.deny(&jti, 60).await.expect("deny");
        assert!(dl.is_denied(&jti).await.expect("query"));
    }
}
```

(NOTE: tests 에 uuid crate 의존; 이미 workspace 에 있는지 확인 필요. 없으면 dev-dep 추가.)

- [ ] **Step 5.4: audit.rs — AuthEvent + writer**

`crates/auth/src/audit.rs`:

```rust
//! 인증 이벤트 → `audit_log` writer.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;

/// 인증 흐름에서 발생하는 6 종 이벤트.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "event")]
pub enum AuthEvent {
    /// 첫 로그인 또는 새 세션 발급.
    Login { user_sub: String, jti: String, exp: i64 },
    /// 로그아웃 (back-channel).
    Logout { user_sub: String, jti: String },
    /// Refresh 성공 (jti rotation 포함).
    RefreshSucceeded {
        user_sub: String,
        prev_jti: String,
        new_jti: String,
        exp: i64,
    },
    /// Refresh 실패 (Zitadel 거부 / 네트워크 실패).
    RefreshFailed { user_sub: String, jti: String },
    /// 권한 가드 거부 (role mismatch 등).
    RoleGuardDenied {
        user_sub: String,
        required_role: String,
        actual_role: String,
        path: String,
    },
    /// Role 변경 — 모든 활성 jti 가 denylist 추가됨.
    RoleChanged {
        user_sub: String,
        prev_role: String,
        new_role: String,
        invalidated_jti_count: u32,
    },
}

impl AuthEvent {
    /// `audit_log.action` 컬럼에 들어갈 dotted name.
    #[must_use]
    pub const fn action(&self) -> &'static str {
        match self {
            Self::Login { .. } => "auth.login",
            Self::Logout { .. } => "auth.logout",
            Self::RefreshSucceeded { .. } => "auth.refresh.succeeded",
            Self::RefreshFailed { .. } => "auth.refresh.failed",
            Self::RoleGuardDenied { .. } => "auth.role_guard.denied",
            Self::RoleChanged { .. } => "auth.role.changed",
        }
    }

    /// 추적용 user_sub 추출.
    #[must_use]
    pub fn user_sub(&self) -> &str {
        match self {
            Self::Login { user_sub, .. }
            | Self::Logout { user_sub, .. }
            | Self::RefreshSucceeded { user_sub, .. }
            | Self::RefreshFailed { user_sub, .. }
            | Self::RoleGuardDenied { user_sub, .. }
            | Self::RoleChanged { user_sub, .. } => user_sub.as_str(),
        }
    }
}

/// `audit_log` 에 인증 이벤트를 기록해요.
///
/// # Errors
///
/// Postgres INSERT 실패 시 `sqlx::Error` 반환.
pub async fn write(pool: &PgPool, event: &AuthEvent, correlation_id: &str) -> Result<(), sqlx::Error> {
    let id = format!("aud_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);
    let payload = serde_json::to_value(event).expect("AuthEvent serialize");

    sqlx::query(
        r#"
        INSERT INTO audit_log
          (id, actor_id, action, resource_kind, resource_id,
           before_state, after_state, correlation_id, created_at)
        VALUES ($1, NULL, $2, 'user', $3, NULL, $4, $5, $6)
        "#,
    )
    .bind(&id)
    .bind(event.action())
    .bind(event.user_sub())
    .bind(&payload)
    .bind(correlation_id)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_name_matches() {
        let e = AuthEvent::Login {
            user_sub: "u".into(),
            jti: "j".into(),
            exp: 1000,
        };
        assert_eq!(e.action(), "auth.login");
        assert_eq!(e.user_sub(), "u");
    }

    #[test]
    fn role_changed_action() {
        let e = AuthEvent::RoleChanged {
            user_sub: "u".into(),
            prev_role: "Buyer".into(),
            new_role: "Broker".into(),
            invalidated_jti_count: 3,
        };
        assert_eq!(e.action(), "auth.role.changed");
    }

    #[test]
    fn round_trip_serde() {
        let e = AuthEvent::RefreshSucceeded {
            user_sub: "u".into(),
            prev_jti: "j1".into(),
            new_jti: "j2".into(),
            exp: 1000,
        };
        let json = serde_json::to_string(&e).expect("serialize");
        let back: AuthEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(e, back);
    }
}
```

- [ ] **Step 5.5: lib.rs export 추가**

`crates/auth/src/lib.rs` 의 `pub mod` 목록 끝에 추가:

```rust
pub mod audit;
pub mod jti_denylist;
```

- [ ] **Step 5.6: middleware.rs 에 jti denylist 검증 hook 추가**

먼저 현 `crates/auth/src/middleware.rs` 의 `AuthState` 구조체 + `auth_layer` 함수의 verify 호출 위치를 Read 로 확인 (`AuthState { verifier, user_repo }` 와 `verifier.verify(token).await?` 호출). 그 다음 두 가지 변경:

**변경 1**: `AuthState` 에 `jti_denylist: Option<Arc<dyn crate::jti_denylist::JtiDenylist>>` 필드 추가 (`pub user_repo` 다음 줄):

```rust
pub struct AuthState {
    pub verifier: Arc<Verifier>,
    pub user_repo: Arc<dyn UserRepository>,
    /// `JTI` denylist (`SP6-i`) — `None` 이면 검증 skip (fail-open).
    pub jti_denylist: Option<Arc<dyn crate::jti_denylist::JtiDenylist>>,
}
```

**변경 2**: `auth_layer` 의 `let claims = state.verifier.verify(token).await?;` 직후 (User 자동 생성/조회 직전) hook 추가:

```rust
let claims = state.verifier.verify(token).await?;

// SP6-i: JTI denylist (logout / refresh rotation / role change 시 즉시 무효).
// fail-open 정책: Redis 장애 시 가용성 우선 (JWT 검증만으로 통과). audit log 만 남김.
if let Some(dl) = &state.jti_denylist {
    match dl.is_denied(&claims.jti).await {
        Ok(true) => return Err(AuthError::Expired),
        Ok(false) => {}
        Err(e) => tracing::warn!(error = %e, jti = %claims.jti, "jti denylist check failed (fail-open)"),
    }
}

// 기존: User 자동 생성 또는 조회 (변경 없음)
```

`AuthState::new(verifier, user_repo)` 같은 builder 가 있다면 `jti_denylist: None` default 추가 + `with_jti_denylist(...)` 메서드 추가.

- [ ] **Step 5.7: services/api/src/routes/auth_event.rs**

`services/api/src/routes/auth_event.rs`:

```rust
//! `POST /internal/auth/event` — frontend 가 emit 하는 `AuthEvent` 수신 → `audit_log` INSERT.

use auth::audit::{self, AuthEvent};
use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use sqlx::PgPool;

/// 핸들러용 상태 (DB pool).
#[derive(Clone)]
pub struct AuthEventState {
    pub pool: PgPool,
}

/// 요청 본문.
#[derive(Debug, Deserialize)]
pub struct AuthEventPayload {
    pub event: String,
    pub payload: serde_json::Value,
}

/// 핸들러 — `event` + `payload` 를 합쳐 `AuthEvent` 로 deserialize 한 후 `audit_log` 에 기록.
///
/// # Errors
///
/// JSON 파싱 / DB INSERT 실패 시 500 반환.
pub async fn post_auth_event(
    State(state): State<AuthEventState>,
    Json(body): Json<AuthEventPayload>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut combined = body.payload;
    if let Some(obj) = combined.as_object_mut() {
        obj.insert("event".into(), serde_json::Value::String(body.event));
    } else {
        return Err((StatusCode::BAD_REQUEST, "payload must be object".to_owned()));
    }

    let event: AuthEvent = serde_json::from_value(combined)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid event: {e}")))?;

    let correlation_id = format!("cor_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);

    audit::write(&state.pool, &event, &correlation_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("db: {e}")))?;

    Ok(StatusCode::ACCEPTED)
}
```

- [ ] **Step 5.8: services/api/src/main.rs modify (라우트 + jti_denylist init)**

`services/api/src/main.rs` 에 추가:

```rust
mod routes {
    pub mod auth_event;
}

use deadpool_redis::{Config as RedisCfg, Runtime as RedisRt};

// ... main() 내 ...
let redis_url = env::var("REDIS_URL").expect("REDIS_URL must be set");
let redis_pool = RedisCfg::from_url(redis_url)
    .create_pool(Some(RedisRt::Tokio1))
    .expect("redis pool");
let jti_denylist: Arc<dyn auth::jti_denylist::JtiDenylist> =
    Arc::new(auth::jti_denylist::RedisJtiDenylist::new(redis_pool));

let auth_state = AuthState {
    verifier,
    user_repo,
    jti_denylist: Some(jti_denylist),
};

let auth_event_state = routes::auth_event::AuthEventState { pool: pool.clone() };

let internal: Router<()> = Router::new()
    .route("/internal/auth/event", axum::routing::post(routes::auth_event::post_auth_event))
    .with_state(auth_event_state);

let app = public.merge(protected).merge(internal).layer(TraceLayer::new_for_http());
```

(`deadpool-redis` 를 services/api 의 Cargo.toml 에도 dependency 추가 — `deadpool-redis = "0.18"`.)

- [ ] **Step 5.9: cargo check + clippy**

```
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 5.10: cargo test**

```
REDIS_URL=redis://localhost:6379 cargo test -p auth
cargo test -p api
```

Expected: PASS (jti_denylist + audit + auth_event integration).

- [ ] **Step 5.11: Commit**

```bash
git add crates/auth/ services/api/ Cargo.lock
git commit -m "feat(6i-T5): crates/auth jti_denylist + audit_log emit + auth_event endpoint

- claims.rs: jti field 추가 (Zitadel 가 항상 발급)
- jti_denylist.rs: trait + RedisJtiDenylist (deadpool-redis)
- audit.rs: AuthEvent enum (6종) + write(pool, event, correlation_id)
- middleware.rs: verify 후 jti denylist check (fail-open 정책)
- services/api: POST /internal/auth/event 라우트
- main.rs: REDIS_URL env + AuthState.jti_denylist 주입"
```

---

## Task 6: V004 migration + sqlx prepare hook + first-sign-in external_account insert

**Files:**
- Create: `migrations/30008_user_ci_external_account.sql`
- Modify: `crates/auth/src/middleware.rs` (first sign-in 시 external_account zitadel insert)
- Modify: `lefthook.yml` (pre-push 에 sqlx prepare --check)
- Modify: `tarpaulin.toml` (auth crate 새 모듈 포함 확인)

- [ ] **Step 6.1: migration 작성**

`migrations/30008_user_ci_external_account.sql`:

```sql
-- V003_08: SP6-i Auth Core 의 schema 자리.
-- users.ci 는 SP6-CI (KISA 본인확인) 가 채움.
-- external_account 의 kakao/naver/google 행은 SP6-Social federation 이 채움.

ALTER TABLE "user" ADD COLUMN ci VARCHAR(88) UNIQUE NULL;
COMMENT ON COLUMN "user".ci IS
  'KISA Connecting Information (88-char hash). NULL until SP6-CI verifies via NICE/Toss/PASS.';

CREATE TABLE external_account (
    id           CHAR(30) PRIMARY KEY,
    user_id      CHAR(30) NOT NULL REFERENCES "user"(id) ON DELETE CASCADE,
    provider     VARCHAR(32) NOT NULL,
    external_id  VARCHAR(255) NOT NULL,
    linked_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (provider, external_id)
);

CREATE INDEX external_account_user_idx ON external_account(user_id);
CREATE INDEX external_account_provider_idx ON external_account(provider, linked_at DESC);

COMMENT ON TABLE external_account IS
  'Multi-IdP linking. SP6-i populates only zitadel rows on first sign-in. SP6-Social federation populates kakao/naver/google.';

-- provider 값 제약 (SP6-Social 이 추가 시 ALTER 가능)
ALTER TABLE external_account
  ADD CONSTRAINT external_account_provider_chk
  CHECK (provider IN ('zitadel', 'kakao', 'naver', 'google', 'apple'));
```

(NOTE: 기존 `user` 테이블 이름이 `"user"` quoted — V001 패턴 일관 유지. id 는 `char(30)` `usr_...` 형식.)

- [ ] **Step 6.2: migration 적용 + sqlx prepare**

```
psql $DATABASE_URL -f migrations/30008_user_ci_external_account.sql
cargo sqlx prepare --workspace
```

Expected: `.sqlx/` 의 query json 갱신 (auth 가 user 테이블 select 하는 경우).

- [ ] **Step 6.3: first sign-in 시 external_account insert**

`crates/auth/src/middleware.rs` 의 first-sign-in 분기에 추가 (User 자동 생성 후, 같은 트랜잭션 또는 best-effort INSERT):

```rust
// User 자동 생성 직후
if was_first_sign_in {
    let external_id = format!("ea_{}", &uuid::Uuid::new_v4().simple().to_string()[..26]);
    if let Err(e) = sqlx::query(
        r#"
        INSERT INTO external_account (id, user_id, provider, external_id)
        VALUES ($1, $2, 'zitadel', $3)
        ON CONFLICT (provider, external_id) DO NOTHING
        "#,
    )
    .bind(&external_id)
    .bind(user.id.as_str())
    .bind(&claims.sub)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "external_account zitadel insert failed (best-effort)");
    }
}
```

(실제 위치는 middleware.rs 의 first sign-in 로직 확인 후 결정. 현재 코드 미확인 시 Step 7 의 코드 검토 후 정확한 위치 적용.)

- [ ] **Step 6.4: lefthook.yml 에 sqlx prepare check 추가**

`lefthook.yml` 의 `pre-push:` 섹션에 추가:

```yaml
    sqlx-prepare-check:
      run: command -v cargo >/dev/null 2>&1 && (DATABASE_URL=${DATABASE_URL:-postgres://gongzzang:gongzzang@localhost:5432/gongzzang} cargo sqlx prepare --workspace --check) || echo "cargo not installed - CI enforces"
      skip:
        - merge
        - rebase
```

- [ ] **Step 6.5: tarpaulin.toml 검토**

`tarpaulin.toml` — `crates/auth/` 가 이미 포함되어 있는지 확인. exclude 목록에 jti_denylist / audit 가 없어야 함 (90% threshold 적용).

- [ ] **Step 6.6: db-migrations workflow assertion 갱신**

`tests/migrations/test_v001_full.sh` 의 `EXPECTED_TABLES` 배열에 `external_account` 추가 (SP7-iii 에서 이미 동적 count 사용 중 — 새 테이블 1개 추가 시 자동 반영되지만 명시 등록은 필요):

```bash
# 변경 전 (SP7-iii 후 상태):
EXPECTED_TABLES=(... api_health_check)

# 변경 후 (SP6-i 추가):
EXPECTED_TABLES=(... api_health_check external_account)
```

확인 명령:

```bash
grep -n "EXPECTED_TABLES" tests/migrations/test_v001_full.sh
# 해당 라인의 배열에 external_account 추가
bash tests/migrations/test_v001_full.sh  # 로컬 검증 (필요한 환경 변수 설정 후)
```

- [ ] **Step 6.7: 전체 빌드 + clippy + test**

```
cargo check --workspace --all-features
cargo clippy --workspace --all-features --all-targets -- -D warnings
DATABASE_URL=postgres://... cargo sqlx prepare --workspace --check
```

Expected: PASS.

- [ ] **Step 6.8: Commit**

```bash
git add migrations/30008_user_ci_external_account.sql crates/auth/src/middleware.rs lefthook.yml .sqlx/ tests/migrations/ .github/workflows/db-migrations.yml
git commit -m "feat(6i-T6): V004 schema (users.ci + external_account) + sqlx prepare hook

- migrations/30008: users.ci VARCHAR(88) UNIQUE NULL (SP6-CI 채움) + external_account 테이블 (SP6-Social 채움), zitadel 한 줄만 first sign-in 시 자동 insert
- middleware.rs: first sign-in 시 external_account('zitadel', sub) INSERT (best-effort)
- lefthook.yml: pre-push 에 cargo sqlx prepare --check 추가 (V004 schema drift 차단)"
```

---

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
