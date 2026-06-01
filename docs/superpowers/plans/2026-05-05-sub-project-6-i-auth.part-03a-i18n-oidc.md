# Sub-project 6-i Auth - Part 03A: i18n And OIDC

Parent index: [Sub-project 6-i Auth - Part 03](./2026-05-05-sub-project-6-i-auth.part-03.md).
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
