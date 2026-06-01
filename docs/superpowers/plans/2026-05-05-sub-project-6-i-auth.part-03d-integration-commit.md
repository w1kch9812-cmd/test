# Sub-project 6-i Auth - Part 03D: Integration Test And Commit

Parent index: [Sub-project 6-i Auth - Part 03](./2026-05-05-sub-project-6-i-auth.part-03.md).

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
