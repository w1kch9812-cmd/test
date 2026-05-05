import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { exchangeCode, type TokenResult } from "@/lib/oidc";
import { deleteTempCookie, setSidCookie, TEMP_COOKIE_NAME } from "@/lib/session/cookie";
import { createSession } from "@/lib/session/store";

const REFRESH_TTL_SEC = 30 * 24 * 60 * 60; // 30일

export async function GET(req: NextRequest) {
  const url = new URL(req.url);
  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  const tmpCookie = req.cookies.get(TEMP_COOKIE_NAME)?.value;

  if (!code || !state || !tmpCookie) {
    return problem({
      type: "auth/state-mismatch",
      title: "로그인 검증에 실패했어요",
      status: 401,
      detail: "보안을 위해 다시 로그인해 주세요.",
      instance: req.url,
    }).toResponse();
  }

  let tmp: {
    code_verifier: string;
    state: string;
    nonce: string;
    return_to: string;
  };
  try {
    tmp = JSON.parse(Buffer.from(tmpCookie, "base64url").toString("utf-8")) as typeof tmp;
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

  let tokens: TokenResult;
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
