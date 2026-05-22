import type { Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { getTranslations } from "next-intl/server";
import { createServerApi } from "@/lib/api";
import { problem } from "@/lib/http/problem";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

const BODY_METHODS = new Set(["POST", "PUT", "PATCH"]);

async function resolveBearer(req: NextRequest): Promise<string | undefined> {
  const sid = req.cookies.get(SID_COOKIE_NAME)?.value;
  if (!sid) return undefined;

  const session = await getSession(sid);
  return session?.access_token;
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

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const t = await getTranslations("server.proxy");
  const path = params.path.join("/");
  const api = createServerApi();
  // SP-Obs T2: X-Request-Id propagation -- 프론트가 보낸 ID 또는 자동 생성.
  // backend Axum middleware 가 동일 ID 를 응답 echo + tracing span 에 attach.
  const requestId = createRequestId(req);

  try {
    const bearer = await resolveBearer(req);
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
  return contentType.toLowerCase().startsWith("application/vnd.mapbox-vector-tile");
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
