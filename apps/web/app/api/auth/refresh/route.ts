import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { refreshTokens, type TokenResult } from "@/lib/oidc";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getRedis } from "@/lib/session/redis";
import { withLock } from "@/lib/session/single-flight";
import { getSession, refreshSession } from "@/lib/session/store";

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

      let next: TokenResult;
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
          payload: {
            user_sub: next.sub,
            prev_jti: current.jti,
            new_jti: next.jti,
            exp: newExp,
          },
        }),
      }).catch(() => undefined);

      return NextResponse.json({ ok: true });
    },
    {
      onLocked: async () => NextResponse.json({ ok: true, contended: true }),
      maxRetries: 3,
      retryDelayMs: 100,
    },
  );
}
