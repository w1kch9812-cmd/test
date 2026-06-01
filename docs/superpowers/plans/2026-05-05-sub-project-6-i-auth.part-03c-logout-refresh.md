# Sub-project 6-i Auth - Part 03C: Logout And Refresh Routes

Parent index: [Sub-project 6-i Auth - Part 03](./2026-05-05-sub-project-6-i-auth.part-03.md).

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
