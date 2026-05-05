import { randomBytes } from "node:crypto";
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { checkRate } from "@/lib/ratelimit";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession, type SessionData } from "@/lib/session/store";
import { sanitizeReturnTo } from "@/lib/url";

const PUBLIC_PATHS = ["/", "/login", "/forbidden", "/api/auth"];
const ADMIN_PATHS = ["/admin"];
const ADMIN_ROLES = new Set(["Admin", "Broker", "Operator"]);

function isPublic(pathname: string): boolean {
  return PUBLIC_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

function isAdmin(pathname: string): boolean {
  return ADMIN_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

async function checkAuthRateLimit(req: NextRequest): Promise<NextResponse | null> {
  const ip = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
  if (req.nextUrl.pathname === "/api/auth/login") {
    const r = await checkRate(`login:${ip}`, 5, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        detail: "잠시 후 다시 시도해 주세요.",
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  } else if (req.nextUrl.pathname === "/api/auth/callback") {
    const r = await checkRate(`callback:${ip}`, 10, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  } else if (req.nextUrl.pathname === "/api/auth/refresh") {
    const sid = req.cookies.get(SID_COOKIE_NAME)?.value ?? "anon";
    const r = await checkRate(`refresh:${sid}`, 30, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: "요청이 너무 많아요",
        status: 429,
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  }
  return null;
}

export async function proxy(req: NextRequest) {
  const url = req.nextUrl;

  // 1. Rate limit (auth routes only)
  const rateBlocked = await checkAuthRateLimit(req);
  if (rateBlocked) return rateBlocked;

  // 2. CSP nonce 주입
  const nonce = randomBytes(16).toString("base64");
  const isDev = process.env.NODE_ENV !== "production";

  // Dev: Naver Maps gl 이 일부 tile/cursor 를 HTTP 로 요청 (legacy) → http:/https: 둘 다 허용.
  // Production: SP6-iam-infra 가 strict allowlist 정리.
  const imgSrc = isDev
    ? "'self' data: blob: http: https:"
    : "'self' data: blob: https://*.map.naver.com https://map.naver.com https://*.map.naver.net https://map.naver.net https://*.pstatic.net";
  const connectSrc = isDev
    ? `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} http: https:`
    : `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} https://*.map.naver.com https://*.map.naver.net https://*.naver.com https://*.navercorp.com`;

  // Naver Maps gl 이 WebGL + eval 사용 → 'unsafe-eval' 필수.
  // 'strict-dynamic' 와 함께 modern browser 에서 호환 (CSP3).
  const cspHeader = [
    `default-src 'self'`,
    // Naver Maps SDK 는 app/layout.tsx 의 <head> 에서 sync 로드되므로 strict-dynamic 을 쓰면
    // SDK 자체가 차단된다. 대신 명시적 allowlist + nonce + unsafe-eval (gl WebGL 필수) 조합.
    // Naver gl SDK 는 nrbe.map.naver.net (style json) + auth 등 일부 리소스를 HTTP 로 호출하므로 dev 에서는 http: 도 허용.
    isDev
      ? `script-src 'self' 'nonce-${nonce}' 'unsafe-eval' 'unsafe-inline' http: https:`
      : `script-src 'self' 'nonce-${nonce}' 'unsafe-eval' 'unsafe-inline' https://oapi.map.naver.com https://*.map.naver.net https://*.pstatic.net`,
    `worker-src 'self' blob:`,
    `style-src 'self' 'unsafe-inline'`,
    `img-src ${imgSrc}`,
    `font-src 'self' data:`,
    `connect-src ${connectSrc}`,
    `frame-ancestors 'none'`,
    `base-uri 'self'`,
    `form-action 'self' ${env.ZITADEL_ISSUER}`,
  ].join("; ");

  const reqHeaders = new Headers(req.headers);
  reqHeaders.set("x-csp-nonce", nonce);

  // 3. Auth gate
  if (isPublic(url.pathname)) {
    const res = NextResponse.next({ request: { headers: reqHeaders } });
    res.headers.set("Content-Security-Policy", cspHeader);
    return res;
  }

  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", sanitizeReturnTo(url.pathname));
    return NextResponse.redirect(loginUrl);
  }

  const session: SessionData | null = await getSession(sid);
  if (!session) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", sanitizeReturnTo(url.pathname));
    const res = NextResponse.redirect(loginUrl);
    res.cookies.delete(SID_COOKIE_NAME);
    return res;
  }

  if (isAdmin(url.pathname) && !ADMIN_ROLES.has(session.role)) {
    return NextResponse.redirect(new URL("/forbidden", req.url));
  }

  const res = NextResponse.next({ request: { headers: reqHeaders } });
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
