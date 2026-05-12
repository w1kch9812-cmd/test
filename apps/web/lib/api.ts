import ky, { isHTTPError } from "ky";
import { env } from "./env";

/**
 * SP-Obs T2: 모든 outbound 요청에 `X-Request-Id` 자동 추가.
 *
 * `crypto.randomUUID()` 가 SSR + CSR 모두 동작 (Node 19+ / 모든 모던 브라우저).
 * 응답 header `x-request-id` 가 echo 되면 같은 ID — backend 가 그대로 받음
 * 일관성. inbound (서버에서 client 로 push) 는 SSE/WebSocket FU 80 단계에서.
 */
function generateRequestId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `req_${crypto.randomUUID().replace(/-/g, "").slice(0, 26).toUpperCase()}`;
  }
  // 매우 오래된 환경 fallback — Math.random 은 entropy 약함이라 dev 만 수용.
  return `req_${Math.random().toString(36).slice(2, 12).toUpperCase()}`;
}

/**
 * Frontend → /api/proxy/[...path] → services/api 호출 ky client.
 *
 * 직접 services/api 호출 X — 항상 Next.js proxy route 통과.
 * (httpOnly cookie 검증 + secrets server-only)
 *
 * SP6-i 가 추가:
 *   - 401 → /login redirect
 *
 * SP-Obs T2 가 추가:
 *   - 모든 요청에 `X-Request-Id: req_<26 alphanumeric>` 자동 부여
 */
import { API } from "@/lib/routes";

export const api = ky.create({
  prefix: API.proxy.base,
  retry: {
    limit: 1,
    methods: ["get"],
  },
  timeout: 10000,
  hooks: {
    beforeRequest: [
      ({ request }) => {
        if (!request.headers.has("x-request-id")) {
          request.headers.set("x-request-id", generateRequestId());
        }
      },
    ],
    beforeError: [
      ({ error }) => {
        if (isHTTPError(error) && error.response.status === 401) {
          // SP6-i 가 redirect 로직 추가
          console.warn("[api] 401 — login required");
        }
        return error;
      },
    ],
  },
});

/**
 * Server-side direct API client (Route Handler / Server Component 만).
 * Browser bundle 에 포함되지 않음.
 */
export function createServerApi(authHeader?: string) {
  return ky.create({
    prefix: env.NEXT_PUBLIC_API_BASE_URL,
    timeout: 10000,
    headers: authHeader ? { Authorization: authHeader } : {},
  });
}
