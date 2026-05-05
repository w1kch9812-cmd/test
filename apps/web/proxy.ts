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
  const cspHeader = [
    `default-src 'self'`,
    // https://oapi.map.naver.com: Naver Maps SDK script loader
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic' https://oapi.map.naver.com`,
    `style-src 'self' 'unsafe-inline'`,
    // *.map.naver.com: map tiles/images; *.pstatic.net: Naver static assets
    `img-src 'self' data: blob: https://*.map.naver.com https://*.pstatic.net`,
    `font-src 'self' data:`,
    // *.map.naver.com + *.naver.com: Naver Maps API calls and geocoding
    `connect-src 'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} https://*.map.naver.com https://*.naver.com`,
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
