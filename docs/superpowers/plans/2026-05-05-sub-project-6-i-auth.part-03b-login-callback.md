# Sub-project 6-i Auth - Part 03B: Login And Callback Routes

Parent index: [Sub-project 6-i Auth - Part 03](./2026-05-05-sub-project-6-i-auth.part-03.md).

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
