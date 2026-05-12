import { type NextRequest, NextResponse } from "next/server";
import { getTranslations } from "next-intl/server";
import { emitAuthEvent } from "@/lib/auth/internal-event";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { refreshTokens, type TokenResult } from "@/lib/oidc";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getRedis } from "@/lib/session/redis";
import { withLock } from "@/lib/session/single-flight";
import { deleteSession, getSession, REFRESH_TTL_SEC, refreshSession } from "@/lib/session/store";

export async function POST(req: NextRequest) {
  const t = await getTranslations("server.auth.refresh");
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    return problem({
      type: "auth/session-expired",
      title: t("expiredTitle"),
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
          title: t("expiredTitle"),
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
        await emitAuthEvent("RefreshFailed", {
          user_sub: current.sub,
          jti: current.jti,
        });
        // I5: zombie session 방지 — refresh_token 만료 시 session 도 종료
        await deleteSession(sid);
        return problem({
          type: "auth/session-expired",
          title: t("expiredTitle"),
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

      await emitAuthEvent("RefreshSucceeded", {
        user_sub: next.sub,
        prev_jti: current.jti,
        new_jti: next.jti,
        exp: newExp,
      });

      return NextResponse.json({ ok: true });
    },
    {
      onLocked: async () => {
        // I3: 다른 refresh 가 진행 중 — lock 경합 시 최신 session 으로 status 결정
        // 다른 refresh 가 완료됐을 가능성 높음. 재조회 후 exp 확인.
        const refreshed = await getSession(sid);
        if (refreshed && refreshed.exp > Math.floor(Date.now() / 1000)) {
          return NextResponse.json({ ok: true, contended: true });
        }
        // 여전히 stale — refresh fail 처리
        return problem({
          type: "auth/session-expired",
          title: t("expiredTitle"),
          status: 401,
          instance: req.url,
        }).toResponse();
      },
      maxRetries: 3,
      retryDelayMs: 100,
    },
  );
}
