# Sub-project 6-i Auth - Part 04C: Tests And Commit

Parent index: [Sub-project 6-i Auth - Part 04](./2026-05-05-sub-project-6-i-auth.part-04.md).

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
