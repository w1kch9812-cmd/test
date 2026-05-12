import { timingSafeEqual } from "node:crypto";
import { type NextRequest, NextResponse } from "next/server";
import { getTranslations } from "next-intl/server";
import { emitAuthEvent } from "@/lib/auth/internal-event";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { exchangeCode, type TokenResult } from "@/lib/oidc";
import {
  deleteTempCookie,
  setSidCookie,
  TEMP_COOKIE_NAME,
  verifyTempPayload,
} from "@/lib/session/cookie";
import { createSession, REFRESH_TTL_SEC } from "@/lib/session/store";
import { sanitizeReturnTo } from "@/lib/url";

export async function GET(req: NextRequest) {
  const t = await getTranslations("server.auth.callback");
  const url = new URL(req.url);
  const code = url.searchParams.get("code");
  const state = url.searchParams.get("state");
  const tmpCookie = req.cookies.get(TEMP_COOKIE_NAME)?.value;

  if (!code || !state || !tmpCookie) {
    return problem({
      type: "auth/state-mismatch",
      title: t("verifyFailedTitle"),
      status: 401,
      detail: t("verifyFailedDetail"),
      instance: req.url,
    }).toResponse();
  }

  // C2: verify HMAC signature before trusting cookie contents
  const rawPayload = verifyTempPayload(tmpCookie);
  if (!rawPayload) {
    return problem({
      type: "auth/state-mismatch",
      title: t("verifyFailedTitle"),
      status: 401,
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
    tmp = JSON.parse(rawPayload) as typeof tmp;
  } catch {
    return problem({
      type: "auth/state-mismatch",
      title: t("verifyFailedTitle"),
      status: 401,
      instance: req.url,
    }).toResponse();
  }

  // I1: timing-safe state comparison (timing attack prevention)
  const a = Buffer.from(tmp.state, "utf-8");
  const b = Buffer.from(state, "utf-8");
  if (a.length !== b.length || !timingSafeEqual(a, b)) {
    return problem({
      type: "auth/state-mismatch",
      title: t("verifyFailedTitle"),
      // M6: detail 제거 — type 이 식별자 역할 (RFC 7807 SSOT)
      status: 401,
      instance: req.url,
    }).toResponse();
  }

  let tokens: TokenResult;
  try {
    tokens = await exchangeCode({
      issuer: env.ZITADEL_ISSUER,
      clientId: env.ZITADEL_CLIENT_ID,
      redirectUri: env.ZITADEL_REDIRECT_URI,
      callbackUrl: new URL(req.url),
      expectedState: tmp.state,
      code_verifier: tmp.code_verifier,
      expectedNonce: tmp.nonce,
    });
  } catch (err) {
    // I4: production 에서 err.message 노출 제거 (internal info leak 방지)
    // dev 에서만 디버깅 detail 표시.
    console.error("[auth/callback] exchangeCode failed:", err);
    const isDev = process.env.NODE_ENV !== "production";
    return problem({
      type: "auth/idp-unavailable",
      title: t("idpUnavailableTitle"),
      status: 503,
      detail: isDev ? (err instanceof Error ? err.message : String(err)) : undefined,
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

  // C1: sanitize return_to before redirect (open redirect prevention — second line of defense)
  const redirectTo = sanitizeReturnTo(tmp.return_to);

  return new NextResponse(null, {
    status: 302,
    headers: [
      ["Location", redirectTo],
      ["Set-Cookie", setSidCookie(sid, REFRESH_TTL_SEC)],
      ["Set-Cookie", deleteTempCookie()],
    ],
  });
}

// emitAuthEvent — apps/web/lib/auth/internal-event.ts SSOT helper 사용 (audit 2026-05-08).
