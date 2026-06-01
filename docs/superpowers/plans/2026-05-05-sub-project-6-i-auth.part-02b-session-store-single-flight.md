# Sub-project 6-i Auth - Part 02B: Session Store And Single Flight

Parent index: [Sub-project 6-i Auth - Part 02](./2026-05-05-sub-project-6-i-auth.part-02.md).

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
