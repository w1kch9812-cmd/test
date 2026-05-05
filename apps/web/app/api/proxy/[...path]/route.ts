import { isHTTPError, type Options as KyOptions } from "ky";
import { type NextRequest, NextResponse } from "next/server";
import { createServerApi } from "@/lib/api";

/**
 * SP6-foundation: backend proxy skeleton — auth 검증 X (unauthenticated).
 * SP6-i 가 채울 부분:
 *   1) iron-session cookie 검증
 *   2) Authorization: Bearer <jwt> 헤더 추가
 *   3) 401 → /login redirect
 *
 * 본 sub-project 는 단순 forward 만 — /healthz 같은 unauthenticated endpoint smoke 가능.
 */

async function forward(req: NextRequest, params: { path: string[] }): Promise<NextResponse> {
  const path = params.path.join("/");
  const url = new URL(req.url);
  const search = url.search;

  // SP6-i 가 cookie 검증 + Authorization 헤더 추가
  const api = createServerApi();

  try {
    const requestInit: KyOptions = {
      method: req.method,
    };

    if (search) {
      const searchParams: Record<string, string> = {};
      for (const [key, value] of new URLSearchParams(search).entries()) {
        searchParams[key] = value;
      }
      requestInit.searchParams = searchParams;
    }

    if (["POST", "PUT", "PATCH"].includes(req.method)) {
      try {
        requestInit.json = await req.json();
      } catch {
        // body 없는 POST/PUT/PATCH 도 허용
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
    return NextResponse.json({ error: "Backend unreachable", code: "PROXY_FAIL" }, { status: 502 });
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
