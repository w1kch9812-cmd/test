import ky, { isHTTPError } from "ky";
import { env } from "./env";

/**
 * Frontend → /api/proxy/[...path] → services/api 호출 ky client.
 *
 * 직접 services/api 호출 X — 항상 Next.js proxy route 통과.
 * (httpOnly cookie 검증 + secrets server-only)
 *
 * SP6-i 가 추가:
 *   - 401 → /login redirect
 */
export const api = ky.create({
  prefix: "/api/proxy",
  retry: {
    limit: 1,
    methods: ["get"],
  },
  timeout: 10000,
  hooks: {
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
