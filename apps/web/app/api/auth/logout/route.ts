import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { buildEndSessionUrl } from "@/lib/oidc";
import { deleteSidCookie, SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getRedis } from "@/lib/session/redis";
import { deleteSession, getSession } from "@/lib/session/store";

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
  // NOTE: getSession → set jti:deny → deleteSession 사이의 사소한 race window 존재.
  // 동시 요청이 jti denylist set 직전 도착 시 backend verify 통과 가능 (sub-ms window).
  // access_token TTL 5분 + 동일 jti 재사용 불가 + audit log 기록 으로 mitigation.
  // 완전한 atomicity 가 필요하면 Redis MULTI/EXEC pipeline 또는 Lua script 로 변경.
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

  // back-channel logout — Zitadel 의 post_logout_redirect_uri 는 전체 URL 필요
  // env.ZITADEL_REDIRECT_URI 의 origin 기반으로 "/" 생성
  const redirectOrigin = new URL(env.ZITADEL_REDIRECT_URI).origin;
  const endUrl = buildEndSessionUrl({
    issuer: env.ZITADEL_ISSUER,
    idTokenHint: session.id_token,
    postLogoutRedirectUri: `${redirectOrigin}/`,
  });

  return new NextResponse(null, {
    status: 302,
    headers: { Location: endUrl, "Set-Cookie": deleteSidCookie() },
  });
}

export async function GET(req: NextRequest) {
  return POST(req);
}
