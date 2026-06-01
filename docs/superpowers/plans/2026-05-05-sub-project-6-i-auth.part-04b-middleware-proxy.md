# Sub-project 6-i Auth - Part 04B: Middleware And Proxy

Parent index: [Sub-project 6-i Auth - Part 04](./2026-05-05-sub-project-6-i-auth.part-04.md).

- [ ] **Step 4.10: middleware.ts 작성**

`apps/web/middleware.ts`:

```typescript
import { NextResponse, type NextRequest } from "next/server";
import { randomBytes } from "node:crypto";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession, type SessionData } from "@/lib/session/store";
import { checkRate } from "@/lib/ratelimit";
import { problem } from "@/lib/http/problem";

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

export async function middleware(req: NextRequest) {
  const url = req.nextUrl;

  // 1. Rate limit (auth routes only)
  const rateBlocked = await checkAuthRateLimit(req);
  if (rateBlocked) return rateBlocked;

  // 2. CSP nonce 주입
  const nonce = randomBytes(16).toString("base64");
  const cspHeader = [
    `default-src 'self'`,
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'`,
    `style-src 'self' 'unsafe-inline'`,
    `img-src 'self' data: blob:`,
    `font-src 'self' data:`,
    `connect-src 'self' ${process.env.NEXT_PUBLIC_API_BASE_URL ?? ""} ${process.env.ZITADEL_ISSUER ?? ""}`,
    `frame-ancestors 'none'`,
    `base-uri 'self'`,
    `form-action 'self' ${process.env.ZITADEL_ISSUER ?? ""}`,
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
    loginUrl.searchParams.set("returnTo", url.pathname);
    return NextResponse.redirect(loginUrl);
  }

  const session: SessionData | null = await getSession(sid);
  if (!session) {
    const loginUrl = new URL("/login", req.url);
    loginUrl.searchParams.set("returnTo", url.pathname);
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
  matcher: [
    "/((?!_next/static|_next/image|favicon.ico).*)",
  ],
};
```

- [ ] **Step 4.11: next.config.ts modify (HSTS + X-Frame + Referrer)**

`apps/web/next.config.ts` 전체 교체:

```typescript
import type { NextConfig } from "next";
import createNextIntlPlugin from "next-intl/plugin";

const withNextIntl = createNextIntlPlugin("./i18n.ts");

const securityHeaders = [
  { key: "Strict-Transport-Security", value: "max-age=63072000; includeSubDomains; preload" },
  { key: "X-Frame-Options", value: "DENY" },
  { key: "X-Content-Type-Options", value: "nosniff" },
  { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
  { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
];

const nextConfig: NextConfig = {
  reactStrictMode: true,
  typedRoutes: true,
  async headers() {
    return [
      {
        source: "/(.*)",
        headers: securityHeaders,
      },
    ];
  },
};

export default withNextIntl(nextConfig);
```

(CSP 는 middleware.ts 가 동적 nonce 와 함께 주입.)

- [ ] **Step 4.12: proxy 에 sid → Bearer 변환 추가**

`apps/web/app/api/proxy/[...path]/route.ts` 의 `forward` 함수 교체 (기존 forward 바디 전체):

```typescript
import { isHTTPError, type Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { createServerApi } from "@/lib/api";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";
import { problem } from "@/lib/http/problem";

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const path = params.path.join("/");
  const url = new URL(req.url);
  const search = url.search;

  // SP6-i: sid → access_token 변환
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  let bearer: string | undefined;
  if (sid) {
    const session = await getSession(sid);
    if (session) bearer = session.access_token;
  }

  const api = createServerApi();

  try {
    const requestInit: KyOptions = {
      method: req.method,
      headers: bearer ? { Authorization: `Bearer ${bearer}` } : {},
    };

    if (search) {
      const searchParams: Record<string, string> = {};
      for (const [k, v] of new URLSearchParams(search).entries()) searchParams[k] = v;
      requestInit.searchParams = searchParams;
    }

    if (["POST", "PUT", "PATCH"].includes(req.method)) {
      try {
        requestInit.json = await req.json();
      } catch {
        // body 없는 요청 허용
      }
    }

    const response = await api(path, requestInit);
    const text = await response.text();
    const contentType = response.headers.get("content-type") ?? "text/plain";
    return new NextResponse(text, {
      status: response.status,
      headers: { "content-type": contentType },
    });
  } catch (err: unknown) {
    if (isHTTPError(err)) {
      // 401: backend 가 JTI denylist 또는 만료 응답 → frontend 가 refresh 시도 자리
      const body = await err.response.text();
      return new NextResponse(body, { status: err.response.status });
    }
    return problem({
      type: "proxy/upstream-unavailable",
      title: "백엔드 서버에 연결할 수 없어요",
      status: 502,
      detail: "잠시 후 다시 시도해 주세요.",
      instance: req.url,
    }).toResponse() as unknown as NextResponse;
  }
}

export async function GET(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function POST(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function PUT(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function PATCH(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
export async function DELETE(req: NextRequest, ctx: { params: Promise<{ path: string[] }> }) {
  return forward(req, await ctx.params);
}
```
