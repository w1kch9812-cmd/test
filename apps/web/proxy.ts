import { randomBytes } from "node:crypto";
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { tStatic } from "@/lib/i18n/static";
import { resolveVectorTileAllowedOrigins } from "@/lib/map/vector-tile-manifest";
import { checkRate } from "@/lib/ratelimit";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession, type SessionData } from "@/lib/session/store";
import { sanitizeReturnTo } from "@/lib/url";

// SP9 ADR 0036 / platform-core ADR 0004 채택 후 — 우리 origin 의 PMTiles/SW prefix
// 모두 폐기. 클라가 platform-core manifest 를 읽고, 표준 mapbox-gl `type:"vector"`
// source 의 tile URL 은 manifest.tiles_url_template 이 결정한다.
//
// `/dev-tiles` — dev 환경에서만 Next dev 가 자체 호스팅 (apps/web/public/dev-tiles/).
// `/dev-x9-test` — ADR 0021 X9 path 시각 검증 page. dev 한정.
// `/fonts` — Pretendard variable subset CSS + woff2 정적 자산 (모든 페이지에 필요).
// production 빌드는 R2 직결이라 본 prefix 없음. dev 의 *X9 path 시각 검증* 용도.
const PUBLIC_PATHS = [
  "/",
  "/login",
  "/forbidden",
  "/api/auth",
  "/platform-core/events",
  "/dev-tiles",
  "/dev-x9-test",
  "/fonts",
];
const ADMIN_PATHS = ["/admin"];
const ADMIN_ROLES = new Set(["Admin", "Broker", "Operator"]);
// SP6-iv: 매물 등록/수정 = Broker 전용 (admin 도 허용 — 운영 세부 결정).
//
// audit 2026-05-08 round 2 (P4 — Codex 발견): BROKER 게이트 SSOT.
// `/listings/new` exact + `/listings/<id>/edit` (dynamic id) 두 패턴.
// 새 broker-gated path 추가 시 본 배열 한 곳만 수정 → `isBrokerGated` 자동 반영.
type BrokerRule =
  | { kind: "exact"; path: string }
  | { kind: "prefix-suffix"; prefix: string; suffix: string };
const BROKER_GATED_RULES: readonly BrokerRule[] = [
  { kind: "exact", path: "/listings/new" },
  { kind: "prefix-suffix", prefix: "/listings/", suffix: "/edit" },
];
const BROKER_ALLOWED_ROLES = new Set(["Broker", "Admin"]);

function isPublic(pathname: string): boolean {
  return PUBLIC_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

function isAdmin(pathname: string): boolean {
  return ADMIN_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`));
}

function isBrokerGated(pathname: string): boolean {
  return BROKER_GATED_RULES.some((rule) => {
    if (rule.kind === "exact") return pathname === rule.path;
    return pathname.startsWith(rule.prefix) && pathname.endsWith(rule.suffix);
  });
}

async function checkAuthRateLimit(req: NextRequest): Promise<NextResponse | null> {
  const ip = req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
  if (req.nextUrl.pathname === "/api/auth/login") {
    const r = await checkRate(`login:${ip}`, 5, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: tStatic("server.proxy.rateLimitedTitle"),
        status: 429,
        detail: tStatic("server.proxy.retryLaterDetail"),
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  } else if (req.nextUrl.pathname === "/api/auth/callback") {
    const r = await checkRate(`callback:${ip}`, 10, 60);
    if (!r.allowed) {
      return problem({
        type: "auth/too-many-requests",
        title: tStatic("server.proxy.rateLimitedTitle"),
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
        title: tStatic("server.proxy.rateLimitedTitle"),
        status: 429,
        instance: req.url,
      }).toResponse() as unknown as NextResponse;
    }
  }
  return null;
}

/**
 * Next.js 16 `proxy` — *모든 request* 의 auth gate + CSP nonce + rate limit.
 *
 * Next.js 16 부터 `middleware.ts` → `proxy.ts` 로 rename (deprecated).
 * runtime = `nodejs` (configurable 안 됨) — `node:crypto` / Redis 직접 사용 가능.
 * 본 file 이 `apps/web/proxy.ts` + `proxy` 이름 export 일 때만 Next.js 가 자동 invoke.
 */
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
  // ADR 0036 / platform-core ADR 0004: Gongzzang은 manifest consumer only.
  // Manifest는 platform-core Catalog 또는 public R2/CDN manifest URL에서 읽고,
  // 실제 tile URL은 manifest.tiles_url_template이 결정한다.
  const tileOrigins = resolveVectorTileAllowedOrigins().join(" ");
  const connectSrc = isDev
    ? `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} http: https:`
    : `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} https://*.map.naver.com https://*.map.naver.net https://*.naver.com https://*.navercorp.com${tileOrigins ? ` ${tileOrigins}` : ""}`;

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
  // SP6-iv: 매물 등록/수정 broker (또는 admin) 만 진입.
  if (isBrokerGated(url.pathname) && !BROKER_ALLOWED_ROLES.has(session.role)) {
    return NextResponse.redirect(new URL("/forbidden", req.url));
  }

  const res = NextResponse.next({ request: { headers: reqHeaders } });
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
