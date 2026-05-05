import { isHTTPError, type Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { createServerApi } from "@/lib/api";
import { problem } from "@/lib/http/problem";
import { SID_COOKIE_NAME } from "@/lib/session/cookie";
import { getSession } from "@/lib/session/store";

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
