import type { Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { getTranslations } from "next-intl/server";
import { createServerApi } from "@/lib/api";
import { problem } from "@/lib/http/problem";
import { GENERATED_API_PROXY_ROUTE_POLICIES } from "@/lib/policies/traffic-auth-policy.generated";
import { checkRate } from "@/lib/ratelimit";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession, type SessionData } from "@/lib/session/store";

const BODY_METHODS = new Set(["POST", "PUT", "PATCH"]);

async function resolveSession(req: NextRequest): Promise<SessionData | null> {
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) return null;

  return getSession(sid);
}

function createRequestId(req: NextRequest): string {
  return (
    req.headers.get("x-request-id") ??
    `req_${(globalThis.crypto?.randomUUID?.() ?? Math.random().toString(36).slice(2)).replace(/-/g, "").slice(0, 26).toUpperCase()}`
  );
}

function searchParamsRecord(search: string): Record<string, string> | undefined {
  if (!search) return undefined;

  const searchParams: Record<string, string> = {};
  for (const [key, value] of new URLSearchParams(search).entries()) {
    searchParams[key] = value;
  }
  return searchParams;
}

async function assignJsonBody(req: NextRequest, requestInit: KyOptions): Promise<void> {
  if (!BODY_METHODS.has(req.method)) return;

  try {
    requestInit.json = await req.json();
  } catch {
    // body 없는 요청 허용
  }
}

async function buildProxyRequestOptions(
  req: NextRequest,
  requestId: string,
  bearer: string | undefined,
): Promise<KyOptions> {
  const headers: Record<string, string> = { "x-request-id": requestId };
  if (bearer) headers.Authorization = `Bearer ${bearer}`;

  // throwHttpErrors=false: 4xx/5xx 도 정상 Response 로 받아 body 한 번만 읽음.
  // 기존 동작 (throw → catch → err.response.text()) 은 ky 가 error 생성 시 body 를
  // 내부 소비해서 "Body is unusable" 에러 발생. body single consumption 으로 우회.
  const requestInit: KyOptions = {
    method: req.method,
    headers,
    throwHttpErrors: false,
    searchParams: searchParamsRecord(new URL(req.url).search),
  };

  await assignJsonBody(req, requestInit);
  return requestInit;
}

async function readProxyBody(
  response: Response,
  contentType: string,
): Promise<string | ArrayBuffer> {
  return isBinaryProxyResponse(contentType) ? response.arrayBuffer() : response.text();
}

function isTemplateSegment(segment: string): boolean {
  return segment.startsWith(":") && segment.length > 1;
}

function matchesTemplatePath(template: string, path: string): boolean {
  const templateSegments = template.split("/");
  const pathSegments = path.split("/");
  if (templateSegments.length !== pathSegments.length) return false;

  return templateSegments.every((segment, index) => {
    const pathSegment = pathSegments[index];
    if (pathSegment === undefined) return false;
    if (isTemplateSegment(segment)) return pathSegment.length > 0;
    return segment === pathSegment;
  });
}

function matchesApiProxyRoutePolicy(
  policy: (typeof GENERATED_API_PROXY_ROUTE_POLICIES)[number],
  method: string,
  path: string,
): boolean {
  if (!(policy.methods as readonly string[]).includes(method)) return false;

  if (policy.kind === "exact") return path === policy.targetPath;
  if (policy.kind === "prefix") {
    return path === policy.targetPath || path.startsWith(`${policy.targetPath}/`);
  }
  return matchesTemplatePath(policy.targetPath, path);
}

function getApiProxyRoutePolicy(method: string, path: string) {
  return GENERATED_API_PROXY_ROUTE_POLICIES.find((policy) =>
    matchesApiProxyRoutePolicy(policy, method, path),
  );
}

function sessionRequiredProblem(req: NextRequest, t: (key: string) => string): NextResponse {
  return problem({
    type: "proxy/session-required",
    title: t("sessionRequiredTitle"),
    status: 401,
    detail: t("sessionRequiredDetail"),
    instance: req.url,
  }).toResponse() as unknown as NextResponse;
}

function insufficientRoleProblem(req: NextRequest, t: (key: string) => string): NextResponse {
  return problem({
    type: "proxy/insufficient-role",
    title: t("insufficientRoleTitle"),
    status: 403,
    detail: t("insufficientRoleDetail"),
    instance: req.url,
  }).toResponse() as unknown as NextResponse;
}

function rateLimitedProblem(
  req: NextRequest,
  t: (key: string) => string,
  type: string,
): NextResponse {
  return problem({
    type,
    title: t("rateLimitedTitle"),
    status: 429,
    detail: t("retryLaterDetail"),
    instance: req.url,
  }).toResponse() as unknown as NextResponse;
}

async function enforceApiProxyExposure(
  req: NextRequest,
  policy: (typeof GENERATED_API_PROXY_ROUTE_POLICIES)[number],
  t: (key: string) => string,
): Promise<{ session: SessionData | null; response: NextResponse | null }> {
  if (policy.exposureClass === "public_derived") {
    return { session: null, response: null };
  }

  const session = await resolveSession(req);
  if (!session) {
    return { session, response: sessionRequiredProblem(req, t) };
  }

  if (policy.exposureClass === "privileged" && !policy.requiredRoles.includes(session.role)) {
    return { session, response: insufficientRoleProblem(req, t) };
  }

  return { session, response: null };
}

function clientIp(req: NextRequest): string {
  return req.headers.get("x-forwarded-for")?.split(",")[0]?.trim() ?? "unknown";
}

function resolveApiProxyRateKey(
  req: NextRequest,
  policy: (typeof GENERATED_API_PROXY_ROUTE_POLICIES)[number],
  session: SessionData | null,
): string {
  if (!policy.rate) {
    throw new Error(`API proxy route policy has no rate profile: ${policy.targetPath}`);
  }

  const subject = session?.sub ?? clientIp(req);
  return `${policy.rate.keyPrefix}:${subject}`;
}

async function checkApiProxyRateLimit(
  req: NextRequest,
  policy: (typeof GENERATED_API_PROXY_ROUTE_POLICIES)[number],
  session: SessionData | null,
  t: (key: string) => string,
): Promise<NextResponse | null> {
  if (!policy.rate) return null;

  const result = await checkRate(
    resolveApiProxyRateKey(req, policy, session),
    policy.rate.limit,
    policy.rate.windowSec,
  );
  if (result.allowed) return null;

  return rateLimitedProblem(req, t, policy.rate.problemType);
}

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const t = await getTranslations("server.proxy");
  const path = params.path.join("/");
  const routePolicy = getApiProxyRoutePolicy(req.method, path);
  if (!routePolicy) {
    return problem({
      type: "proxy/route-not-allowed",
      title: t("routeNotAllowedTitle"),
      status: 404,
      detail: t("routeNotAllowedDetail"),
      instance: req.url,
    }).toResponse() as unknown as NextResponse;
  }
  const exposureGate = await enforceApiProxyExposure(req, routePolicy, t);
  if (exposureGate.response) return exposureGate.response;
  const rateLimitGate = await checkApiProxyRateLimit(req, routePolicy, exposureGate.session, t);
  if (rateLimitGate) return rateLimitGate;

  const api = createServerApi();
  // SP-Obs T2: X-Request-Id propagation -- 프론트가 보낸 ID 또는 자동 생성.
  // backend Axum middleware 가 동일 ID 를 응답 echo + tracing span 에 attach.
  const requestId = createRequestId(req);

  try {
    const bearer = exposureGate.session?.access_token;
    const requestInit = await buildProxyRequestOptions(req, requestId, bearer);
    const response = await api(path, requestInit);
    const contentType = response.headers.get("content-type") ?? "text/plain";
    const body = await readProxyBody(response, contentType);
    // SP-Obs T2: backend echo 받은 X-Request-Id 응답 propagate (debugging UX).
    const responseRequestId = response.headers.get("x-request-id") ?? requestId;
    return new NextResponse(body, {
      status: response.status,
      headers: {
        "content-type": contentType,
        "x-request-id": responseRequestId,
      },
    });
  } catch {
    // 진짜 네트워크/연결 실패만 여기로. 4xx/5xx 는 throwHttpErrors=false 로 위에서 처리.
    return problem({
      type: "proxy/upstream-unavailable",
      title: t("backendUnavailableTitle"),
      status: 502,
      detail: t("retryLaterDetail"),
      instance: req.url,
    }).toResponse() as unknown as NextResponse;
  }
}

function isBinaryProxyResponse(contentType: string): boolean {
  const normalized = contentType.toLowerCase();
  return (
    normalized.startsWith("application/vnd.mapbox-vector-tile") || normalized.startsWith("image/")
  );
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
