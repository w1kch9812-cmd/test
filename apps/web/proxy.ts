import { randomBytes } from "node:crypto";
import { type NextRequest, NextResponse } from "next/server";
import { env } from "@/lib/env";
import { problem } from "@/lib/http/problem";
import { tStatic } from "@/lib/i18n/static";
import { resolveVectorTileAllowedOrigins } from "@/lib/map/vector-tile-manifest";
import {
  GENERATED_AUTH_RATE_ROUTE_POLICIES,
  GENERATED_PAGE_ROUTE_POLICIES,
  GENERATED_PUBLIC_MAP_ROUTE_POLICIES,
} from "@/lib/policies/traffic-auth-policy.generated";
import { checkRate } from "@/lib/ratelimit";
import { API, AUTH_PATH_PREFIX, ROUTES } from "@/lib/routes";
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
  ROUTES.login,
  ROUTES.forbidden,
  AUTH_PATH_PREFIX,
  API.platformCore.events,
  "/fonts",
];
const LISTING_MARKER_MASK_PREFIX = API.proxy.listingMarkerMasksPrefix;
const DEV_ONLY_PUBLIC_PATHS = ["/dev-tiles", "/dev-x9-test"];

type PublicMapRoutePolicy = {
  kind: "exact" | "prefix";
  path: string;
  exposure: {
    class: "public_derived";
    allowedDataClasses: readonly string[];
    rawRecordAccess: "forbidden";
    bulkExport: "forbidden";
  };
  rate: {
    keyPrefix: string;
    limit: number;
    windowSec: number;
  };
};

type AuthRateRoutePolicy = {
  path: string;
  methods: readonly string[];
  rate: {
    keyPrefix: string;
    keyStrategy: "client_ip" | "session_or_anon";
    limit: number;
    windowSec: number;
    problemType: string;
  };
};

type PageRoutePolicy =
  | {
      kind: "exact";
      path: string;
      requiredRoles: readonly string[];
    }
  | {
      kind: "prefix";
      path: string;
      requiredRoles: readonly string[];
    }
  | {
      kind: "prefix_suffix";
      prefix: string;
      suffix: string;
      requiredRoles: readonly string[];
    };

function resolveAuthPath(pathSource: string): string {
  switch (pathSource) {
    case "API.auth.login":
      return API.auth.login;
    case "API.auth.callback":
      return API.auth.callback;
    case "API.auth.refresh":
      return API.auth.refresh;
    case "API.auth.logout":
      return API.auth.logout;
    default:
      throw new Error(`Unknown auth rate route policy path source: ${pathSource}`);
  }
}

function resolvePagePathSource(pathSource: string): string {
  switch (pathSource) {
    case "ROUTES.listings.new":
      return ROUTES.listings.new;
    case "ROUTES.listings.index":
      return ROUTES.listings.index;
    default:
      throw new Error(`Unknown page route policy path source: ${pathSource}`);
  }
}

function resolvePublicMapPath(pathSource: string): string {
  switch (pathSource) {
    case "API.proxy.listingMarkerTilesPrefix":
      return API.proxy.listingMarkerTilesPrefix;
    case "API.proxy.listingMarkerCounts":
      return API.proxy.listingMarkerCounts;
    case "API.proxy.listingMarkerFilters":
      return API.proxy.listingMarkerFilters;
    case "API.proxy.listingMarkerDeltasPrefix":
      return API.proxy.listingMarkerDeltasPrefix;
    case "LISTING_MARKER_MASK_PREFIX":
      return LISTING_MARKER_MASK_PREFIX;
    case "API.proxy.listingMarkerTombstonesPrefix":
      return API.proxy.listingMarkerTombstonesPrefix;
    default:
      throw new Error(`Unknown public map route policy path source: ${pathSource}`);
  }
}

const AUTH_RATE_ROUTE_POLICIES: readonly AuthRateRoutePolicy[] =
  GENERATED_AUTH_RATE_ROUTE_POLICIES.map((policy) => ({
    path: resolveAuthPath(policy.pathSource),
    methods: policy.methods,
    rate: policy.rate,
  }));

const PAGE_ROUTE_POLICIES: readonly PageRoutePolicy[] = GENERATED_PAGE_ROUTE_POLICIES.map(
  (policy) => {
    if (policy.kind === "prefix_suffix") {
      const prefix = policy.prefix ?? resolvePagePathSource(policy.prefixSource ?? "");
      return {
        kind: policy.kind,
        prefix: `${prefix}/`,
        suffix: policy.suffix ?? "",
        requiredRoles: policy.requiredRoles,
      };
    }

    return {
      kind: policy.kind,
      path: policy.path ?? resolvePagePathSource(policy.pathSource ?? ""),
      requiredRoles: policy.requiredRoles,
    };
  },
);

const PUBLIC_MAP_ROUTE_POLICIES: readonly PublicMapRoutePolicy[] =
  GENERATED_PUBLIC_MAP_ROUTE_POLICIES.map((policy) => ({
    kind: policy.kind,
    path: resolvePublicMapPath(policy.pathSource),
    exposure: policy.exposure,
    rate: policy.rate,
  }));

function getPublicMapRoutePolicy(pathname: string): PublicMapRoutePolicy | undefined {
  return PUBLIC_MAP_ROUTE_POLICIES.find((policy) => {
    if (policy.kind === "exact") return pathname === policy.path;
    return pathname === policy.path || pathname.startsWith(`${policy.path}/`);
  });
}

function getAuthRateRoutePolicy(pathname: string, method: string): AuthRateRoutePolicy | undefined {
  return AUTH_RATE_ROUTE_POLICIES.find(
    (policy) => pathname === policy.path && policy.methods.includes(method),
  );
}

function getPageRoutePolicy(pathname: string): PageRoutePolicy | undefined {
  return PAGE_ROUTE_POLICIES.find((policy) => {
    if (policy.kind === "exact") return pathname === policy.path;
    if (policy.kind === "prefix") {
      return pathname === policy.path || pathname.startsWith(`${policy.path}/`);
    }
    return pathname.startsWith(policy.prefix) && pathname.endsWith(policy.suffix);
  });
}

function isPublic(pathname: string): boolean {
  return (
    getPublicMapRoutePolicy(pathname) !== undefined ||
    [...PUBLIC_PATHS, ...DEV_ONLY_PUBLIC_PATHS].some(
      (p) => pathname === p || pathname.startsWith(`${p}/`),
    )
  );
}

function isProductionDevOnlyPath(pathname: string): boolean {
  return (
    process.env.NODE_ENV === "production" &&
    DEV_ONLY_PUBLIC_PATHS.some((p) => pathname === p || pathname.startsWith(`${p}/`))
  );
}

function isLocalHostname(hostname: string): boolean {
  return hostname === "localhost" || hostname === "127.0.0.1" || hostname === "::1";
}

function clientIp(req: NextRequest): string {
  return req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
}

function rateLimitedProblem(req: NextRequest, type: string): NextResponse {
  return problem({
    type,
    title: tStatic("server.proxy.rateLimitedTitle"),
    status: 429,
    detail: tStatic("server.proxy.retryLaterDetail"),
    instance: req.url,
  }).toResponse() as unknown as NextResponse;
}

async function checkAuthRateLimit(req: NextRequest): Promise<NextResponse | null> {
  const policy = getAuthRateRoutePolicy(req.nextUrl.pathname, req.method);
  if (!policy) return null;

  const r = await checkRate(
    resolveAuthRateKey(req, policy),
    policy.rate.limit,
    policy.rate.windowSec,
  );
  if (r.allowed) return null;

  return rateLimitedProblem(req, policy.rate.problemType);
}

function resolveAuthRateKey(req: NextRequest, policy: AuthRateRoutePolicy): string {
  const subject =
    policy.rate.keyStrategy === "session_or_anon"
      ? (req.cookies.get(SID_COOKIE_NAME)?.value ?? "anon")
      : clientIp(req);
  return `${policy.rate.keyPrefix}:${subject}`;
}

async function checkPublicMapRateLimit(req: NextRequest): Promise<NextResponse | null> {
  const policy = getPublicMapRoutePolicy(req.nextUrl.pathname);
  if (!policy) return null;

  const r = await checkRate(
    `${policy.rate.keyPrefix}:${clientIp(req)}`,
    policy.rate.limit,
    policy.rate.windowSec,
  );
  if (r.allowed) return null;

  return rateLimitedProblem(req, "map/too-many-public-marker-requests");
}

function buildCspHeader(hostname: string, nonce: string): string {
  const isDev = process.env.NODE_ENV !== "production";
  const allowLocalHttpMapRuntime = isDev || isLocalHostname(hostname);

  // Dev: Naver Maps gl 이 일부 tile/cursor 를 HTTP 로 요청 (legacy) → http:/https: 둘 다 허용.
  // Production: SP6-iam-infra 가 strict allowlist 정리.
  const imgSrc = allowLocalHttpMapRuntime
    ? "'self' data: blob: http: https:"
    : "'self' data: blob: https://*.map.naver.com https://map.naver.com https://*.map.naver.net https://map.naver.net https://*.pstatic.net";
  // ADR 0036 / platform-core ADR 0004: Gongzzang은 manifest consumer only.
  // Manifest는 platform-core Catalog 또는 public R2/CDN manifest URL에서 읽고,
  // 실제 tile URL은 manifest.tiles_url_template이 결정한다.
  const tileOrigins = resolveVectorTileAllowedOrigins().join(" ");
  const tileConnectSrc = tileOrigins ? ` ${tileOrigins}` : "";
  const connectSrc = allowLocalHttpMapRuntime
    ? `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} http: https:${tileConnectSrc}`
    : `'self' ${env.NEXT_PUBLIC_API_BASE_URL} ${env.ZITADEL_ISSUER} https://*.map.naver.com https://*.map.naver.net https://*.naver.com https://*.navercorp.com${tileConnectSrc}`;

  // Naver Maps gl 이 WebGL + eval 사용 → 'unsafe-eval' 필수.
  // 'strict-dynamic' 와 함께 modern browser 에서 호환 (CSP3).
  return [
    `default-src 'self'`,
    // Naver Maps SDK 는 app/layout.tsx 의 <head> 에서 sync 로드되므로 strict-dynamic 을 쓰면
    // SDK 자체가 차단된다. 대신 명시적 allowlist + nonce + unsafe-eval (gl WebGL 필수) 조합.
    // Naver gl SDK 는 nrbe.map.naver.net (style json) + auth 등 일부 리소스를 HTTP 로 호출하므로 dev 에서는 http: 도 허용.
    allowLocalHttpMapRuntime
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
}

function nextWithSecurityHeaders(reqHeaders: Headers, cspHeader: string): NextResponse {
  const res = NextResponse.next({ request: { headers: reqHeaders } });
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

function notFoundWithSecurityHeaders(cspHeader: string): NextResponse {
  const res = new NextResponse(null, { status: 404 });
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

function redirectWithSecurityHeaders(url: URL, cspHeader: string): NextResponse {
  const res = NextResponse.redirect(url);
  res.headers.set("Content-Security-Policy", cspHeader);
  return res;
}

function redirectToLogin(req: NextRequest, pathname: string): NextResponse {
  const loginUrl = new URL(ROUTES.login, req.url);
  loginUrl.searchParams.set("returnTo", sanitizeReturnTo(pathname));
  return NextResponse.redirect(loginUrl);
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

  // 1. Rate limit
  const rateBlocked = await checkAuthRateLimit(req);
  if (rateBlocked) return rateBlocked;
  const publicMapRateBlocked = await checkPublicMapRateLimit(req);
  if (publicMapRateBlocked) return publicMapRateBlocked;

  // 2. CSP nonce 주입
  const nonce = randomBytes(16).toString("base64");
  const cspHeader = buildCspHeader(url.hostname, nonce);
  const reqHeaders = new Headers(req.headers);
  reqHeaders.set("x-csp-nonce", nonce);

  // 3. Auth gate
  if (isProductionDevOnlyPath(url.pathname)) {
    return notFoundWithSecurityHeaders(cspHeader);
  }

  if (url.pathname === ROUTES.home) {
    return redirectWithSecurityHeaders(new URL(ROUTES.listings.index, req.url), cspHeader);
  }

  if (isPublic(url.pathname)) {
    return nextWithSecurityHeaders(reqHeaders, cspHeader);
  }

  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) {
    return redirectToLogin(req, url.pathname);
  }

  const session: SessionData | null = await getSession(sid);
  if (!session) {
    const res = redirectToLogin(req, url.pathname);
    res.cookies.delete(SID_COOKIE_NAME);
    return res;
  }

  const pageRoutePolicy = getPageRoutePolicy(url.pathname);
  if (pageRoutePolicy && !pageRoutePolicy.requiredRoles.includes(session.role)) {
    return NextResponse.redirect(new URL(ROUTES.forbidden, req.url));
  }

  return nextWithSecurityHeaders(reqHeaders, cspHeader);
}

export const config = {
  matcher: ["/((?!_next/static|_next/image|favicon.ico).*)"],
};
