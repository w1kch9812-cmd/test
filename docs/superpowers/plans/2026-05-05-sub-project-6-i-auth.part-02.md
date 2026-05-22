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

