# Sub-project 6-i Auth - Part 02A: Problem Details And Cookie Helpers

Parent index: [Sub-project 6-i Auth - Part 02](./2026-05-05-sub-project-6-i-auth.part-02.md).
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
