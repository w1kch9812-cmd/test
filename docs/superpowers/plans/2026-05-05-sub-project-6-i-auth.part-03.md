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

